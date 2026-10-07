//! HTTP adapters for the wealth use cases.

use std::collections::BTreeMap;
use std::str::FromStr;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use domain::wealth::{
    AccountId, AccountKind, AccountSummary, Currency, NetWorth, NewAccount, Owner, Valuation,
    WealthError, parse_amount,
};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{Date, OffsetDateTime};
use tower_http::set_header::SetResponseHeaderLayer;
use uuid::Uuid;

use crate::auth_routes::CurrentUser;
use crate::state::AppState;

/// Wealth sub-router; every route requires a session.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/wealth/accounts",
            get(list_accounts).post(create_account),
        )
        .route("/api/wealth/accounts/{id}/archive", post(archive_account))
        .route(
            "/api/wealth/accounts/{id}/valuations",
            get(account_history).post(record_valuation),
        )
        .route("/api/wealth/net-worth", get(net_worth))
        .route("/api/wealth/net-worth/history", get(net_worth_history))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
}

/// Body of `POST /api/wealth/accounts`.
#[derive(Deserialize)]
pub struct CreateAccountRequest {
    name: String,
    kind: String,
    owner: String,
    currency: String,
    notes: Option<String>,
}

/// Body of `POST /api/wealth/accounts/{id}/valuations`.
#[derive(Deserialize)]
pub struct RecordValuationRequest {
    amount: String,
    as_of: String,
}

/// Query of `GET /api/wealth/net-worth`.
#[derive(Deserialize)]
pub struct NetWorthQuery {
    at: Option<String>,
}

/// Query of `GET /api/wealth/net-worth/history`.
#[derive(Deserialize)]
pub struct HistoryQuery {
    from: String,
    to: Option<String>,
}

/// A valuation as exposed to the PWA.
#[derive(Serialize)]
pub struct ValuationResponse {
    id: Uuid,
    as_of: String,
    amount: String,
    currency: String,
    source: &'static str,
    recorded_at: String,
}

/// An account with its latest valuation.
#[derive(Serialize)]
pub struct AccountResponse {
    id: Uuid,
    name: String,
    kind: &'static str,
    owner: &'static str,
    currency: String,
    is_archived: bool,
    notes: Option<String>,
    latest_valuation: Option<ValuationResponse>,
    is_stale: bool,
}

/// Net worth with breakdowns, every amount as a decimal string in euros.
#[derive(Serialize)]
pub struct NetWorthResponse {
    as_of: String,
    total: String,
    by_owner: BTreeMap<&'static str, String>,
    by_kind: BTreeMap<&'static str, String>,
    stale_account_ids: Vec<Uuid>,
}

/// API error: HTTP status plus business code.
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
}

impl From<WealthError> for ApiError {
    fn from(error: WealthError) -> Self {
        let status = match &error {
            WealthError::AccountNotFound => StatusCode::NOT_FOUND,
            WealthError::ExchangeRateMissing { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            WealthError::VaultSealed => StatusCode::SERVICE_UNAVAILABLE,
            WealthError::Storage(message) => {
                tracing::error!(code = error.code(), error = %message, "wealth internal failure");
                StatusCode::INTERNAL_SERVER_ERROR
            }
            WealthError::Cipher => {
                tracing::error!(code = error.code(), "wealth internal failure");
                StatusCode::INTERNAL_SERVER_ERROR
            }
            _ => StatusCode::BAD_REQUEST,
        };
        Self {
            status,
            code: error.code(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(serde_json::json!({ "code": self.code }))).into_response()
    }
}

/// Longest accepted net worth history range, about ten years.
const MAX_HISTORY_DAYS: i64 = 3660;

const INVALID_DATE: ApiError = ApiError {
    status: StatusCode::BAD_REQUEST,
    code: "invalid_date",
};

fn parse_date(input: &str) -> Result<Date, ApiError> {
    Date::parse(input, format_description!("[year]-[month]-[day]")).map_err(|_| INVALID_DATE)
}

fn today() -> Date {
    OffsetDateTime::now_utc().date()
}

fn to_valuation_response(valuation: &Valuation) -> ValuationResponse {
    ValuationResponse {
        id: valuation.id.0,
        as_of: valuation.as_of.to_string(),
        amount: valuation.amount.amount.to_string(),
        currency: valuation.amount.currency.to_string(),
        source: valuation.source.code(),
        recorded_at: valuation.recorded_at.format(&Rfc3339).unwrap_or_default(),
    }
}

fn to_account_response(summary: AccountSummary) -> AccountResponse {
    let AccountSummary {
        account,
        latest_valuation,
        is_stale,
    } = summary;
    AccountResponse {
        id: account.id.0,
        name: account.name,
        kind: account.kind.code(),
        owner: account.owner.code(),
        currency: account.currency.to_string(),
        is_archived: account.is_archived,
        notes: account.notes,
        latest_valuation: latest_valuation.as_ref().map(to_valuation_response),
        is_stale,
    }
}

fn to_net_worth_response(net_worth: &NetWorth) -> NetWorthResponse {
    NetWorthResponse {
        as_of: net_worth.as_of.to_string(),
        total: net_worth.total.amount.to_string(),
        by_owner: net_worth
            .by_owner
            .iter()
            .map(|(owner, money)| (owner.code(), money.amount.to_string()))
            .collect(),
        by_kind: net_worth
            .by_kind
            .iter()
            .map(|(kind, money)| (kind.code(), money.amount.to_string()))
            .collect(),
        stale_account_ids: net_worth.stale_accounts.iter().map(|id| id.0).collect(),
    }
}

async fn list_accounts(
    _user: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<AccountResponse>>, ApiError> {
    let summaries = state
        .wealth
        .wealth()
        .await?
        .account_summaries(today())
        .await?;
    Ok(Json(
        summaries.into_iter().map(to_account_response).collect(),
    ))
}

async fn create_account(
    _user: CurrentUser,
    State(state): State<AppState>,
    Json(body): Json<CreateAccountRequest>,
) -> Result<(StatusCode, Json<AccountResponse>), ApiError> {
    let input = NewAccount {
        name: body.name,
        kind: AccountKind::from_code(&body.kind)?,
        owner: Owner::from_code(&body.owner)?,
        currency: Currency::from_str(&body.currency)?,
        notes: body.notes,
    };
    let account = state
        .wealth
        .wealth()
        .await?
        .create_account(input, OffsetDateTime::now_utc())
        .await?;
    let summary = AccountSummary {
        account,
        latest_valuation: None,
        is_stale: true,
    };
    Ok((StatusCode::CREATED, Json(to_account_response(summary))))
}

async fn archive_account(
    _user: CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state
        .wealth
        .wealth()
        .await?
        .archive_account(AccountId(id))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn record_valuation(
    _user: CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<RecordValuationRequest>,
) -> Result<(StatusCode, Json<ValuationResponse>), ApiError> {
    let amount = parse_amount(&body.amount)?;
    let as_of = parse_date(&body.as_of)?;
    let valuation = state
        .wealth
        .wealth()
        .await?
        .record_valuation(AccountId(id), as_of, amount, OffsetDateTime::now_utc())
        .await?;
    Ok((StatusCode::CREATED, Json(to_valuation_response(&valuation))))
}

async fn account_history(
    _user: CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ValuationResponse>>, ApiError> {
    let valuations = state
        .wealth
        .wealth()
        .await?
        .account_history(AccountId(id))
        .await?;
    Ok(Json(valuations.iter().map(to_valuation_response).collect()))
}

async fn net_worth(
    _user: CurrentUser,
    State(state): State<AppState>,
    Query(query): Query<NetWorthQuery>,
) -> Result<Json<NetWorthResponse>, ApiError> {
    let at = query
        .at
        .as_deref()
        .map(parse_date)
        .transpose()?
        .unwrap_or_else(today);
    let net_worth = state.wealth.wealth().await?.net_worth(at).await?;
    Ok(Json(to_net_worth_response(&net_worth)))
}

async fn net_worth_history(
    _user: CurrentUser,
    State(state): State<AppState>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<NetWorthResponse>>, ApiError> {
    let from = parse_date(&query.from)?;
    let to = query
        .to
        .as_deref()
        .map(parse_date)
        .transpose()?
        .unwrap_or_else(today);
    if (to - from).whole_days() > MAX_HISTORY_DAYS {
        return Err(WealthError::InvalidRange.into());
    }
    let points = state
        .wealth
        .wealth()
        .await?
        .net_worth_history(from, to)
        .await?;
    Ok(Json(points.iter().map(to_net_worth_response).collect()))
}
