mod db;
mod error;
mod handlers;
mod helpers;
mod models;
mod state;

use axum::http::Method;
use axum::routing::{get, post, put};
use axum::Router;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::db::{load_database, normalize_database, open_database};
use crate::models::TeamDatabase;
use crate::state::{AppState, RateLimiter};

const DEFAULT_BIND_ADDR: &str = "0.0.0.0:25578";
const DEFAULT_DB_PATH: &str = "backend.db";

#[tokio::main]
async fn main() {
    init_tracing();

    let bind_addr = std::env::var("FLUX_FLOW_TEAM_BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string());
    let db_path = std::env::var("FLUX_FLOW_TEAM_DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());
    let db_path = PathBuf::from(db_path);
    let conn = open_database(&db_path).expect("failed to open SQLite database");

    let mut database = match load_database(&conn) {
        Ok(database) => database,
        Err(error) => {
            warn!(error = %error, "Failed to load database, starting with empty state");
            TeamDatabase::default()
        }
    };
    normalize_database(&mut database);

    let state = AppState {
        conn: Arc::new(Mutex::new(conn)),
        db: Arc::new(Mutex::new(database)),
        join_limiter: Arc::new(Mutex::new(RateLimiter::new())),
        create_limiter: Arc::new(Mutex::new(RateLimiter::new())),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::OPTIONS]);

    let app = Router::new()
        .route("/health", get(handlers::health))
        .route("/api/team/create", post(handlers::create_team))
        .route("/api/team/join", post(handlers::join_team))
        .route("/api/team/{team_code}/context", get(handlers::load_team_context))
        .route("/api/team/{team_code}/activity", get(handlers::load_team_activity))
        .route("/api/team/{team_code}/todos", get(handlers::load_todos).put(handlers::save_todos))
        .route("/api/team/{team_code}/ideas", get(handlers::load_ideas).put(handlers::save_ideas))
        .route(
            "/api/team/{team_code}/members/{target_member_id}/role",
            put(handlers::update_member_role),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
        .layer(cors);

    let addr: SocketAddr = match bind_addr.parse() {
        Ok(address) => address,
        Err(error) => {
            warn!(bind_addr = %bind_addr, error = %error, "Invalid bind address, using fallback");
            DEFAULT_BIND_ADDR.parse().expect("default bind address must be valid")
        }
    };
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind socket");
    info!(address = %addr, "Flux Flow backend started");
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.expect("server failed");
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("flux_flow_backend=info,tower_http=info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().compact().with_target(false))
        .init();
}