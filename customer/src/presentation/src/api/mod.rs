pub mod api_customer;
pub mod api_health_check;
pub mod api_info;
pub mod docs;

use actix_web::web;
use actix_web::web::ServiceConfig;

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/api/customers")
            .service(api_customer::create)
            .service(api_customer::update),
    );
    cfg.service(
        web::scope("/api/health")
            .service(api_health_check::startup)
            .service(api_health_check::ready)
            .service(api_health_check::live),
    );
    cfg.service(web::scope("/api/info").service(api_info::info));
}

/// Correlation id stored by `shared::web::CorrelationIdMiddleware`.
pub(crate) fn correlation_id(req: &actix_web::HttpRequest) -> String {
    use actix_web::HttpMessage;

    req.extensions()
        .get::<String>()
        .cloned()
        .unwrap_or_else(|| "unknown".to_string())
}
