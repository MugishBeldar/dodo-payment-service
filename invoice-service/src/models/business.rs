use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Business {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}
