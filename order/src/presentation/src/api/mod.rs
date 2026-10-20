pub mod api_info;
pub mod api_orders;
pub mod docs;

use actix_web::web;
use actix_web::web::ServiceConfig;
use shared::web::ApiError;

/// Registers the routes of the order service.
pub fn configure(cfg: &mut ServiceConfig) {
    cfg.app_data(
        web::PathConfig::default()
            .error_handler(|error, _| ApiError::bad_request(error.to_string()).into()),
    );
    cfg.service(
        web::scope("/api/orders")
            .service(api_orders::create)
            .service(api_orders::track),
    );
    cfg.service(web::scope("/api/health").configure(shared::web::health::configure));
    cfg.service(web::scope("/api/info").service(api_info::info));
}
