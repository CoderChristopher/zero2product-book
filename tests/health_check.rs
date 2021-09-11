use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::TcpListener;
use uuid::Uuid;

use zero2prod::configuration::*;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "debug".into();
    let subscriber_name = "test".into();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

fn parse_ipv4_addr(address: Ipv4Addr) -> String {
    let octets = address.octets();
    format!("{}.{}.{}.{}", octets[0], octets[1], octets[2], octets[3])
}

async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);
    let mut configuration = get_configuration().expect("Failed to read configuration.");
	let address = format!("{}:0",configuration.application_ip);

    let listener = TcpListener::bind(address).expect("Failed to bind a random port.");
    let listener_address = listener.local_addr().unwrap();
    let port = listener_address.port();
    let address = match listener_address {
        SocketAddr::V4(ip) => {
            format!("http://{}:{}", parse_ipv4_addr(*ip.ip()), port)
        }
        SocketAddr::V6(_) => {
            panic!("Expecting an ipv4 address and recieved a ipv6 address");
            String::from("")
        }
    };

    configuration.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&configuration.database).await;

    let server = run(listener, connection_pool.clone()).expect("Failed to bind the address");
    let _ = tokio::spawn(server);
    TestApp {
        address,
        db_pool: connection_pool,
    }
}
pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect(&config.connection_string_without_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(&*format!(r#"CREATE DATABASE "{}";"#, config.database_name))
        .await
        .expect("Failed to create database.");
    let connection_pool = PgPool::connect(&config.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}

#[actix_rt::test]
async fn health_check_works() {
    let app_address = spawn_app().await;
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/health_check", &app_address.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[actix_rt::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app_address = spawn_app().await;
    let client = reqwest::Client::new();
    let body = "name=Chris%20Copeland&email=copelandwebdesign%40gmail.com";

    let response = client
        .post(&format!("{}/subscribe", &app_address.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app_address.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.email, "copelandwebdesign@gmail.com");
    assert_eq!(saved.name, "Chris Copeland");
}
#[actix_rt::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
	let app_address = spawn_app().await;
	let client = reqwest::Client::new();
	let body = "name=&email";

	let test_cases = vec![
		("name=testing&email=","Name but no email"),
		("name=&email=email%40123.com","Email but no name"),
		("name=testing&email=this-is-not-an-email","Invalid Email"),
	];

	for (body,description) in test_cases {
			let response = client
				.post(&format!("{}/subscribe",&app_address.address))
				.header("Content-Type","application/x-www-form-urlencoded")
				.body(body)
				.send()
				.await
				.expect("Failed to execute request");
			
			assert_eq!(
				400,
				response.status().as_u16(),
				"The API did not return a 400 when the payload was {}.",
				description
			);
	}
}
#[actix_rt::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let app_address = spawn_app().await;
    let mut test_cases = vec![
        ("name=Chris%20Copeland", "missing the email", false),
        (
            "email=copelandwebdesign%40gmail.com",
            "missing the name",
            false,
        ),
        ("", "missing email and name", false),
    ];
    let mut pass_all = true;
    for (ind, mut test) in test_cases.iter_mut().enumerate() {
        let client = reqwest::Client::new();
        let response = client
            .post(&format!("{}/subscribe", &app_address.address))
            .header("Content-Type", "application/www-x-urlencoded")
            .body(test.0)
            .send()
            .await
            .expect("Failed to execute request.");

        if response.status().as_u16() == 400 {
            println!("Test {}: {}", ind, response.status().as_u16());
            test.2 = true;
        } else {
            pass_all = false;
        }
    }
    if pass_all {
        let mut passed = String::new();
        for test in test_cases {
            passed.push_str(test.1);
            passed.push_str(", ");
        }
        println!("All tests: {} passed.", passed);
    } else {
        let mut passed = String::new();
        let mut failed = String::new();
        for test in test_cases {
            match test.2 {
                true => {
                    passed.push_str(test.1);
                    passed.push_str(", ");
                }
                false => {
                    failed.push_str(test.1);
                    failed.push_str(", ");
                }
            };
        }
        println!("\n***Tests***\nFailed: {}\nPassed: {}\n", failed, passed);
        panic!();
    }
}
