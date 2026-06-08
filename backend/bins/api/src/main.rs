//! Joel API server entry point.

use tracing_subscriber::EnvFilter;

/// Creates the first admin user, printing its temporary credentials to stdout.
///
/// Generates a random password and TOTP secret, hashes and seals them, inserts
/// the user into the database, then prints the credentials for immediate use.
/// The process exits with a non-zero code on any error.
async fn seed_admin(pool: sqlx::PgPool, config: &api::Config, email: &str, display_name: &str) {
    use domain::auth::crypto::{PasswordService, SecretBox, TotpService};
    use domain::auth::model::User;
    use domain::auth::ports::UserRepository;

    let Ok(secret_box) = SecretBox::from_base64(&config.master_key_b64) else {
        tracing::error!("invalid MASTER_KEY");
        std::process::exit(1);
    };
    let mut password_bytes = [0_u8; 18];
    rand::RngCore::fill_bytes(&mut rand::rng(), &mut password_bytes);
    let password = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        password_bytes,
    );
    let totp_secret = TotpService::generate_secret();

    let Ok(password_hash) = PasswordService::hash(&password) else {
        tracing::error!("password hashing failed");
        std::process::exit(1);
    };
    let user = User {
        id: uuid::Uuid::new_v4(),
        email: email.to_owned(),
        display_name: display_name.to_owned(),
        password_hash,
        totp_secret_enc: secret_box.seal(&totp_secret),
    };
    let users = persistence::auth::PgUsers::new(pool);
    if let Err(error) = users.insert(&user).await {
        tracing::error!(%error, "cannot insert admin user");
        std::process::exit(1);
    }
    let otpauth = TotpService::otpauth_url(&totp_secret, email).unwrap_or_default();
    println!("user created: {email}");
    println!("initial password: {password}");
    println!(
        "totp secret (base32): {}",
        TotpService::secret_base32(&totp_secret)
    );
    println!("otpauth url: {otpauth}");
    println!("Add the TOTP secret to your authenticator app, log in, then register a passkey.");
}

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

    if let Err(error) = sqlx::migrate!("../../migrations").run(&pool).await {
        tracing::error!(%error, "database migration failed");
        std::process::exit(1);
    }

    let config = match api::Config::from_env() {
        Ok(config) => config,
        Err(missing) => {
            tracing::error!(variable = %missing, "required environment variable is not set");
            std::process::exit(1);
        }
    };

    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("seed-admin") {
        let (Some(email), Some(display_name)) = (args.get(2), args.get(3)) else {
            tracing::error!("usage: api seed-admin <email> <display_name>");
            std::process::exit(2);
        };
        seed_admin(pool, &config, email, display_name).await;
        return;
    }

    let app = match api::build_router_with(pool, &config) {
        Ok(router) => router.layer(tower_http::trace::TraceLayer::new_for_http()),
        Err(error) => {
            tracing::error!(%error, "failed to build application router");
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

    if let Err(error) = axum::serve(listener, app).await {
        tracing::error!(%error, "server stopped unexpectedly");
        std::process::exit(1);
    }
}
