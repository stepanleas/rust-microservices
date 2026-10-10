pub mod api_categories;
pub mod api_info;
pub mod api_products;
pub mod docs;

use actix_web::web;
use actix_web::web::ServiceConfig;

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/api/categories")
            .service(api_categories::list_all)
            .service(api_categories::find_one)
            .service(api_categories::create)
            .service(api_categories::update)
            .service(api_categories::destroy),
    );
    cfg.service(
        web::scope("/api/products")
            .service(api_products::list_all)
            .service(api_products::find_one)
            .service(api_products::create)
            .service(api_products::update)
            .service(api_products::destroy),
    );
    cfg.service(web::scope("/api/health").configure(shared::web::health::configure));
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
