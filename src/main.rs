use sqlx::PgPool;
use std::net::TcpListener;

use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;

#[actix_web::main]
async fn main() -> std::io::Result<()> {

	//Create a filter to look for info level logs
	let env_filter = EnvFilter::try_from_default_env()
		.unwrap_or_else(|_| EnvFilter::new("info"));

	//Prints to stdout with nicely formatted bunyan json
	let formatting_layer = BunyanFormattingLayer::new(
		"zero2prod".into(),
		std::io::stdout
	);

	//Put it all together into a subscriber
	let subscriber = Registry::default()
		.with(env_filter)
		.with(JsonStorageLayer)
		.with(formatting_layer);
	//set the default subscriber used on all spans
	set_global_default(subscriber).expect("Failed to set subscriber");

    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    let address = format!("127.0.0.1:{}", configuration.application_port);

    println!("Listening: {}", address);
    let listener = TcpListener::bind(address).expect("Unable to bind to random port.");
    run(listener, connection_pool)?.await
}
