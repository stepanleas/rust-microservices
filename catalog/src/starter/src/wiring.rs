use crate::config::Settings;
use application::ports::out::publishers::ProductMessagePublisher;
use application::ports::out::repositories::{CategoryRepository, ProductRepository};
use application::use_cases::{
    CreateCategory, CreateProduct, DeleteCategory, DeleteProduct, FindCategory, FindProduct,
    ListCategories, ListProducts, UpdateCategory, UpdateProduct,
};
use infrastructure::postgres::{PostgresCategoryRepository, PostgresProductRepository};
use messaging::publishers::ProductKafkaMessagePublisher;
use presentation::state::AppState;
use shared::db::{DbPool, DbSettings};
use shared::kafka::KafkaProducer;
use std::sync::Arc;

/// Builds the pool and runs the migrations.
///
/// Blocking (database connection): call it before the server starts.
pub fn connect_database(settings: &Settings) -> anyhow::Result<DbPool> {
    shared::db::connect(
        &DbSettings {
            url: settings.database_url.clone(),
            max_connections: settings.db_max_connections,
            connection_timeout: settings.db_connection_timeout,
        },
        infrastructure::MIGRATIONS,
    )
}

/// Builds the adapters and the use cases.
pub fn build_app_state(settings: &Settings, pool: &DbPool) -> AppState {
    let category_repository: Arc<dyn CategoryRepository> =
        Arc::new(PostgresCategoryRepository::new(pool));
    let product_repository: Arc<dyn ProductRepository> =
        Arc::new(PostgresProductRepository::new(pool));
    let product_publisher: Arc<dyn ProductMessagePublisher> =
        Arc::new(ProductKafkaMessagePublisher::new(KafkaProducer::new(vec![
            settings.kafka_host.clone(),
        ])));

    AppState {
        environment: settings.environment.clone(),

        list_categories: Arc::new(ListCategories::new(category_repository.clone())),
        find_category: Arc::new(FindCategory::new(category_repository.clone())),
        create_category: Arc::new(CreateCategory::new(category_repository.clone())),
        update_category: Arc::new(UpdateCategory::new(category_repository.clone())),
        delete_category: Arc::new(DeleteCategory::new(category_repository.clone())),

        list_products: Arc::new(ListProducts::new(product_repository.clone())),
        find_product: Arc::new(FindProduct::new(product_repository.clone())),
        create_product: Arc::new(CreateProduct::new(
            product_repository.clone(),
            category_repository.clone(),
            product_publisher.clone(),
        )),
        update_product: Arc::new(UpdateProduct::new(
            product_repository.clone(),
            category_repository,
            product_publisher.clone(),
        )),
        delete_product: Arc::new(DeleteProduct::new(product_repository, product_publisher)),
    }
}
