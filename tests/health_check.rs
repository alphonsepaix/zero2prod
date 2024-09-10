use once_cell::sync::Lazy;
use sqlx::{postgres::PgPoolOptions, Executor, PgPool};
use std::time::Duration;
use uuid::Uuid;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::router,
    telemetry::init_subscriber,
};

static TRACING: Lazy<()> = Lazy::new(|| {
    if std::env::var("TEST_LOG").is_ok() {
        init_subscriber("test".into(), "debug".into(), std::io::stdout);
    } else {
        init_subscriber("test".into(), "debug".into(), std::io::sink);
    }
});

struct TestApp {
    address: String,
    pool: PgPool,
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create the database.
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect_with(config.without_db())
        .await
        .expect("Failed to connect to Postgres.");
    pool.execute(format!(r#"CREATE DATABASE "{}";"#, &config.database_name).as_str())
        .await
        .expect("Failed to create the database.");

    // Migrate the database.
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database.");

    connection_pool
}

async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let pool = configure_database(&configuration.database).await;
    let app = TestApp {
        address: format!("http://127.0.0.1:{}", port),
        pool: pool.clone(),
    };

    tokio::spawn(async {
        axum::serve(listener, router().with_state(pool))
            .await
            .unwrap();
    });

    app
}

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length())
}

#[tokio::test]
async fn endpoint_returns_a_200_for_valid_form_data() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let body = "username=Alphonse&message=Hello";
    let response = client
        .post(&format!("{}/reply", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT username, message_content FROM messages;")
        .fetch_one(&app.pool)
        .await
        .expect("Failed to fetch saved messages.");

    assert_eq!(saved.username, "Alphonse");
    assert_eq!(saved.message_content, "Hello");
}

#[tokio::test]
async fn endpoint_returns_a_422_when_data_is_missing() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("username=Alphonse", "missing the message"),
        ("message=Hello", "missing the username"),
        ("", "missing both username and message"),
    ];
    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/reply", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            422,
            response.status().as_u16(),
            "The API did not fail with 422 Unprocessable Entity when the payload was {}.",
            error_message
        );
    }
}
