use application::dtos::{CategoryDto, ProductDto};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Envelope used by every successful response: `{ "data": ... }`.
#[derive(Serialize, ToSchema)]
pub struct DataResponse<T> {
    pub data: T,
}

#[readonly::make]
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({"environment": "development"}))]
pub struct AppInfoResponse {
    environment: String,
}

impl AppInfoResponse {
    pub fn new(environment: impl Into<String>) -> Self {
        Self {
            environment: environment.into(),
        }
    }
}

#[readonly::make]
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({"id": "6f1c1f0e-3b8e-4a43-9d0e-2a4b1c7d9e10", "title": "Pizza", "description": "Italian pizzas"}))]
pub struct CategoryResponse {
    id: Uuid,
    title: String,
    description: String,
}

impl From<&CategoryDto> for CategoryResponse {
    fn from(category: &CategoryDto) -> Self {
        Self {
            id: category.id(),
            title: category.title().into(),
            description: category.description().into(),
        }
    }
}

impl From<CategoryDto> for CategoryResponse {
    fn from(category: CategoryDto) -> Self {
        Self::from(&category)
    }
}

#[readonly::make]
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "id": "0411a4a9-1edb-4180-9556-800cb2b84721",
    "title": "Margherita",
    "description": "Tomato, mozzarella, basil",
    "quantity": 10,
    "price": "12.50",
    "category": {"id": "6f1c1f0e-3b8e-4a43-9d0e-2a4b1c7d9e10", "title": "Pizza", "description": "Italian pizzas"}
}))]
pub struct ProductResponse {
    id: Uuid,
    title: String,
    description: String,
    quantity: i32,
    /// Decimal price serialized as string.
    price: String,
    category: CategoryResponse,
}

impl From<ProductDto> for ProductResponse {
    fn from(product: ProductDto) -> Self {
        Self {
            id: product.id(),
            title: product.title().into(),
            description: product.description().into(),
            quantity: product.quantity(),
            price: product.price().to_string(),
            category: product.category().into(),
        }
    }
}
