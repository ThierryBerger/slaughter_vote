mod models;

use chrono::Utc;
use models::*;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{env, sync::Arc};
use supabase_jwt::{Claims, JwksCache};
use tower_http::cors::CorsLayer;

// ===== App State =====

#[derive(Clone)]
struct AppState {
    db: PgPool,
    jwks_cache: Arc<JwksCache>,
}

// ===== Auth Middleware =====

async fn verify_jwt(jwks_cache: &Arc<JwksCache>, headers: &HeaderMap) -> Result<String, AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::BadRequest("no auth".into()))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::BadRequest("no bearer".into()))?;

    let claims = Claims::from_token(token, jwks_cache)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED);
    pub fn is_expired(exp: i64) -> bool {
        Utc::now().timestamp() > exp
    }

    match claims {
        Err(_) => Err(AppError::Unauthorized),
        Ok(claims) if is_expired(claims.exp as i64) => Err(AppError::Unauthorized),
        Ok(claims) => Ok(claims.sub),
    }
}

// ===== Main =====

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Setup database connection
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let jwks_cache = Arc::new(JwksCache::new(
        "https://haiqmpqncyioxkwaegiu.supabase.co/auth/v1/.well-known/jwks.json",
    ));
    let state = AppState { db, jwks_cache };

    // Build router
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/themes/next", get(get_next_theme))
        .route("/themes/vote", post(submit_vote))
        // TODO: these may have to not exist or be protected.
        .route("/admin/stats", get(get_stats))
        .route("/admin/export", get(export_votes))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Server running on http://0.0.0.0:3000");

    axum::serve(listener, app).await?;
    Ok(())
}

// ===== Handlers =====

async fn root() -> &'static str {
    "Theme Voting Backend (Supabase Auth) - Use /health to check status"
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    // Check DB connection
    match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => Json(serde_json::json!({
            "status": "ok",
            "database": "connected"
        })),
        Err(_) => Json(serde_json::json!({
            "status": "error",
            "database": "disconnected"
        })),
    }
}

async fn get_next_theme(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ThemeResponse>, AppError> {
    let user_id = verify_jwt(&state.jwks_cache, &headers).await?;

    // Get total themes count
    let total_themes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM themes")
        .fetch_one(&state.db)
        .await?;

    // Get themes already voted on by this user
    let voted_theme_ids: Vec<i32> =
        sqlx::query_scalar("SELECT theme_id FROM votes WHERE user_id = $1")
            .bind(&user_id)
            .fetch_all(&state.db)
            .await?;

    // Get a random unvoted theme
    let theme: Option<Theme> = if voted_theme_ids.is_empty() {
        sqlx::query_as("SELECT id, content FROM themes ORDER BY RANDOM() LIMIT 1")
            .fetch_optional(&state.db)
            .await?
    } else {
        sqlx::query_as(
            "SELECT id, content FROM themes 
             WHERE id != ALL($1) 
             ORDER BY RANDOM() 
             LIMIT 1",
        )
        .bind(&voted_theme_ids)
        .fetch_optional(&state.db)
        .await?
    };

    Ok(Json(ThemeResponse {
        theme,
        total: total_themes,
        seen: voted_theme_ids.len() as i64,
    }))
}

async fn submit_vote(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(vote_req): Json<VoteRequest>,
) -> Result<StatusCode, AppError> {
    let user_id = verify_jwt(&state.jwks_cache, &headers).await?;

    // Validate vote type
    if !["yes", "no", "skip"].contains(&vote_req.vote_type.as_str()) {
        return Err(AppError::BadRequest("Invalid vote type".into()));
    }

    // Check theme exists
    let theme_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM themes WHERE id = $1)")
            .bind(vote_req.theme_id)
            .fetch_one(&state.db)
            .await?;

    if !theme_exists {
        return Err(AppError::BadRequest("Theme not found".into()));
    }

    // Insert or update vote
    sqlx::query(
        "INSERT INTO votes (user_id, theme_id, vote_type) 
         VALUES ($1, $2, $3)
         ON CONFLICT (user_id, theme_id) 
         DO UPDATE SET vote_type = $3, created_at = NOW()",
    )
    .bind(&user_id)
    .bind(vote_req.theme_id)
    .bind(&vote_req.vote_type)
    .execute(&state.db)
    .await?;

    Ok(StatusCode::OK)
}

async fn get_stats(State(state): State<AppState>) -> Result<Json<Vec<VoteStats>>, AppError> {
    let stats: Vec<VoteStats> = sqlx::query_as!(
        VoteStats,
        r#"
        SELECT 
            t.id as theme_id,
            t.content,
            COUNT(CASE WHEN v.vote_type = 'yes' THEN 1 END) as "yes_votes!",
            COUNT(CASE WHEN v.vote_type = 'no' THEN 1 END) as "no_votes!",
            COUNT(CASE WHEN v.vote_type = 'skip' THEN 1 END) as "skip_votes!",
            COUNT(v.id) as "total_votes!"
        FROM themes t
        LEFT JOIN votes v ON t.id = v.theme_id
        GROUP BY t.id, t.content
        ORDER BY COUNT(CASE WHEN v.vote_type = 'yes' THEN 1 END) DESC
        "#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(stats))
}

async fn export_votes(State(state): State<AppState>) -> Result<Json<Vec<ExportVote>>, AppError> {
    let votes: Vec<ExportVote> = sqlx::query_as!(
        ExportVote,
        r#"
        SELECT 
            v.user_id,
            v.theme_id,
            t.content as theme_content,
            v.vote_type
        FROM votes v 
        JOIN themes t ON v.theme_id = t.id 
        ORDER BY v.created_at DESC
        "#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(votes))
}

// ===== Error Handling =====

enum AppError {
    Unauthorized,
    BadRequest(String),
    Database(sqlx::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Unauthorized - Invalid or missing JWT token".to_string(),
            ),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Database(err) => {
                tracing::error!("Database error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            }
        };

        (status, message).into_response()
    }
}
