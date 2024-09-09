use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;
use zero2prod::telemetry::init_subscriber;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    init_subscriber("zero2prod".into(), "trace".into(), std::io::stdout);
    let configuration = get_configuration().expect("Failed to read configuration.");
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect_lazy(configuration.database.connection_string().expose_secret())
        .expect("Failed to connect to Postgres.");
    run(listener, pool).await
}
