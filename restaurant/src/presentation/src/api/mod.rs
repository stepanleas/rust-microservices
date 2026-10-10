pub mod api_info;
pub mod api_order_approvals;
pub mod api_products;
pub mod docs;

use actix_web::web;
use actix_web::web::ServiceConfig;
use shared::web::ApiError;

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.app_data(
        web::PathConfig::default()
            .error_handler(|error, _| ApiError::bad_request(error.to_string()).into()),
    );
    cfg.service(web::scope("/api/order-approvals").service(api_order_approvals::find_by_order_id));
    cfg.service(
        web::scope("/api/products")
            .service(api_products::list)
            .service(api_products::find_by_id),
    );
    cfg.service(web::scope("/api/health").configure(shared::web::health::configure));
    cfg.service(web::scope("/api/info").service(api_info::info));
}
