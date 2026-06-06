//! Joel API server entry point.

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        tracing::error!("DATABASE_URL is not set");
        std::process::exit(1);
    };
    let pool = match sqlx::PgPool::connect(&database_url).await {
        Ok(pool) => pool,
        Err(error) => {
            tracing::error!(%error, "cannot connect to database");
            std::process::exit(1);
        }
    };

    let bind = std::env::var("APP_BIND").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    let listener = match tokio::net::TcpListener::bind(&bind).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!(%bind, %error, "cannot bind listener");
            std::process::exit(1);
        }
    };
    tracing::info!(%bind, "api listening");

    let app = api::build_router(pool).layer(tower_http::trace::TraceLayer::new_for_http());
    if let Err(error) = axum::serve(listener, app).await {
        tracing::error!(%error, "server stopped unexpectedly");
        std::process::exit(1);
    }
}
