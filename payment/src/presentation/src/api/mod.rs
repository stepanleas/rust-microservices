pub mod api_info;
pub mod api_payment;
pub mod docs;

use actix_web::web;
use actix_web::web::ServiceConfig;

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/api/payments")
            .service(api_payment::get_by_order_id)
            .service(api_payment::get)
            .service(api_payment::create)
            .service(api_payment::update),
    );
    cfg.service(web::scope("/api/health").configure(shared::web::health::configure));
    cfg.service(web::scope("/api/info").service(api_info::info));
}
