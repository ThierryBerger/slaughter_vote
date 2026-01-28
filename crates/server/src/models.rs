use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Theme {
    pub id: i32,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Vote {
    pub id: i32,
    pub user_id: String,
    pub theme_id: i32,
    pub vote_type: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VoteRequest {
    pub theme_id: i32,
    pub vote_type: String, // "yes", "no", "skip"
}

#[derive(Debug, Serialize)]
pub struct ThemeResponse {
    pub theme: Option<Theme>,
    pub total: i64,
    pub seen: i64,
}

#[derive(Debug, Serialize)]
pub struct VoteStats {
    pub theme_id: i32,
    pub content: String,
    pub yes_votes: i64,
    pub no_votes: i64,
    pub skip_votes: i64,
    pub total_votes: i64,
}

#[derive(Debug, Serialize)]
pub struct ExportVote {
    pub user_id: String,
    pub theme_id: i32,
    pub theme_content: String,
    pub vote_type: String,
}
