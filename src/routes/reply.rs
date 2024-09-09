use axum::extract::{Form, State};
use axum::{http::StatusCode, response::IntoResponse};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct FormData {
    username: String,
    message: String,
}

#[tracing::instrument(
    name = "adding a new message",
    skip(pool, form),
    fields(
        username = %form.username,
        message = %form.message
    )
)]
pub async fn reply(State(pool): State<PgPool>, Form(form): Form<FormData>) -> impl IntoResponse {
    match insert_message(&pool, &form).await {
        Ok(_) => {
            tracing::info!("new message details have been saved");
            StatusCode::OK
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[tracing::instrument(name = "saving message details in the database", skip(pool, form))]
async fn insert_message(pool: &PgPool, form: &FormData) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO messages (message_id, username, message_content, publish_date)
        VALUES ($1, $2, $3, $4);
        "#,
        Uuid::new_v4(),
        form.username,
        form.message,
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("failed to execute query: {e}");
        e
    })?;
    Ok(())
}

pub async fn greet() -> &'static str {
    "Hello, World!"
}
