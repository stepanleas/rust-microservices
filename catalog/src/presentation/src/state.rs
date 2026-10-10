use application::use_cases::{
    CreateCategory, CreateProduct, DeleteCategory, DeleteProduct, FindCategory, FindProduct,
    ListCategories, ListProducts, UpdateCategory, UpdateProduct,
};
use std::sync::Arc;

/// Everything the controllers need: the use cases, built in `starter::wiring`.
#[derive(Clone)]
pub struct AppState {
    pub environment: String,

    pub list_categories: Arc<ListCategories>,
    pub find_category: Arc<FindCategory>,
    pub create_category: Arc<CreateCategory>,
    pub update_category: Arc<UpdateCategory>,
    pub delete_category: Arc<DeleteCategory>,

    pub list_products: Arc<ListProducts>,
    pub find_product: Arc<FindProduct>,
    pub create_product: Arc<CreateProduct>,
    pub update_product: Arc<UpdateProduct>,
    pub delete_product: Arc<DeleteProduct>,
}
