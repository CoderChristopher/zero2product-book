use actix_web::{ web, HttpResponse, };

#[derive(serde::Deserialize)]
pub struct SubscribeFormData {
	email: String,
	name: String,
}

pub async fn subscribe(form: web::Form<SubscribeFormData>) -> HttpResponse {
	HttpResponse::Ok().finish()
}
