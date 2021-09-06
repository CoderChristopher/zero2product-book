use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use std::net::TcpListener;

use crate::routes::subscribe;
use crate::routes::health_check;

pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
	println!("Listening on:{}",listener.local_addr().unwrap().port());
	let server = HttpServer::new(|| {
		App::new()
			.route("/health_check", web::get().to(health_check))
			.route("/subscribe", web::post().to(subscribe))
	})
	.listen(listener)?
	.run();
	Ok(server)
}
