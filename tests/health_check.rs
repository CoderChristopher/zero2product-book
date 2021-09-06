use std::net::TcpListener;
use sqlx::{PgConnection, Connection};

use zero2prod::startup::run;
use zero2prod::configuration::get_configuration;

#[actix_rt::test]
async fn health_check_works() {
	let address = spawn_app();
	let client = reqwest::Client::new();
	let response = client
		.get(&format!("{}/health_check",&address))
		.send()
		.await
		.expect("Failed to execute request");


	assert!(response.status().is_success());
	assert_eq!(Some(0), response.content_length());
}
fn spawn_app() -> String {
	let listener = TcpListener::bind("127.0.0.1:0")
		.expect("Failed to bind a random port.");
	let port = listener.local_addr().unwrap().port();
	let server = run(listener).expect("Failed to bind the address");
	let _ = tokio::spawn(server);
	format!("http://127.0.0.1:{}",port)
}

#[actix_rt::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
	let app_address = spawn_app();
	let configuration = get_configuration().expect("Failed to read configuration.");
	let connection_string = configuration.database.connection_string();
	let mut connection = PgConnection::connect(&connection_string)
		.await
		.expect("failed to connect to Postgres.");
	let client = reqwest::Client::new();
	let body = "name=Chris%20Copeland&email=copelandwebdesign%40gmail.com";

	let response = client
		.post(&format!("{}/subscribe",&app_address))
		.header("Content-Type","application/x-www-form-urlencoded")
		.body(body)
		.send()
		.await
		.expect("Failed to execute request.");
	
	assert_eq!(200,response.status().as_u16());

	let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
		.fetch_one(&mut connection)
		.await
		.expect("Failed to fetch saved subscription.");
	assert_eq!(saved.email, "copelandwebdesign@gmail.com");
	assert_eq!(saved.name, "Chris Copeland");
}
#[actix_rt::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
	let address = spawn_app();
	let mut test_cases = vec![
		("name=Chris%20Copeland","missing the email",false),
		("email=copelandwebdesign%40gmail.com","missing the name",false),
		("","missing email and name",false),
	];
	let mut pass_all = true;
	for (ind,mut test) in test_cases.iter_mut().enumerate(){
		let client = reqwest::Client::new();
		let response = client
			.post(&format!("{}/subscribe",&address))
			.header("Content-Type","application/www-x-urlencoded")
			.body(test.0)
			.send()
			.await
			.expect("Failed to execute request.");
		if response.status().as_u16() == 400 {
			println!("Test {}: {}",ind,response.status().as_u16());
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
				},
				false => {
					failed.push_str(test.1);
					failed.push_str(", ");
				}
			};
		}
		println!("\n***Tests***\nFailed: {}\nPassed: {}\n", failed,passed);
		panic!();
	}
}
