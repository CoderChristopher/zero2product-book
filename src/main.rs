use sqlx::PgPool;
use std::net::TcpListener;

use zero2prod::configuration::get_configuration;
use zero2prod::email_client::EmailClient;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    let address = format!("{}:{}", configuration.application_ip, configuration.application_port);

	let sender_email = configuration.email_client.sender()
		.expect("Invalid sender email");

	let email_client = EmailClient::new(
		configuration.email_client.base_url,
		sender_email
	);

    println!("Listening: {}", address);
    let listener = TcpListener::bind(address).expect("Unable to bind to address.");
    run(listener, connection_pool, email_client)?.await?;
	Ok(())
}
