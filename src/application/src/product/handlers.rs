use crate::category::repositories::CategoryRepository;
use crate::ports::output::publishers::ProductMessagePublisher;
use crate::product::commands::{CreateProductCommand, DeleteProductCommand, UpdateProductCommand};
use crate::product::dtos::ProductDto;
use crate::product::queries::FindProductQuery;
use crate::product::repositories::ProductRepository;
use domain::events::{ProductCreatedEvent, ProductDeletedEvent, ProductUpdatedEvent};
use domain::product::entity::Product;
use rusty_money::{Money, iso};
use shared::domain::value_objects::{CategoryId, ProductId};
use std::sync::Arc;

pub struct ListAllProductQueryHandler {
    repository: Arc<dyn ProductRepository>,
}

impl ListAllProductQueryHandler {
    pub fn new(repository: Arc<dyn ProductRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self) -> anyhow::Result<Vec<ProductDto>> {
        self.repository
            .list_all()
            .map(|items| items.into_iter().map(ProductDto::from).collect())
    }
}

pub struct FindProductQueryHandler {
    repository: Arc<dyn ProductRepository>,
}

impl FindProductQueryHandler {
    pub fn new(repository: Arc<dyn ProductRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, query: FindProductQuery) -> anyhow::Result<ProductDto> {
        self.repository
            .find_by_id(ProductId::from_uuid(query.id.unwrap()))
            .map(ProductDto::from)
    }
}

pub struct CreateProductCommandHandler {
    category_repository: Arc<dyn CategoryRepository>,
    product_repository: Arc<dyn ProductRepository>,
    message_publisher: Arc<dyn ProductMessagePublisher>,
}

impl CreateProductCommandHandler {
    pub fn new(
        product_repository: Arc<dyn ProductRepository>,
        category_repository: Arc<dyn CategoryRepository>,
        message_publisher: Arc<dyn ProductMessagePublisher>,
    ) -> Self {
        Self {
            product_repository,
            category_repository,
            message_publisher,
        }
    }

    // TODO: Add transactional outbox
    pub async fn execute(&self, command: CreateProductCommand<'_>) -> anyhow::Result<ProductDto> {
        let category = self
            .category_repository
            .find_by_id(CategoryId::from_uuid(command.category_id()))?;

        let product = Product::builder()
            .id(ProductId::new())
            .title(command.title())
            .description(command.description())
            .quantity(command.quantity())
            .price(Money::from_decimal(command.price(), iso::USD))
            .category(category)
            .build();

        let saved_product = self.product_repository.save(product)?;
        tracing::info!(
            "Product with id: {} created",
            saved_product.id().as_uuid().to_string(),
        );

        let product_dto = ProductDto::from(&saved_product);
        self.message_publisher
            .publish_created(ProductCreatedEvent::new(saved_product))?;

        Ok(product_dto)
    }
}

pub struct UpdateProductCommandHandler {
    category_repository: Arc<dyn CategoryRepository>,
    product_repository: Arc<dyn ProductRepository>,
    message_publisher: Arc<dyn ProductMessagePublisher>,
}

impl UpdateProductCommandHandler {
    pub fn new(
        product_repository: Arc<dyn ProductRepository>,
        category_repository: Arc<dyn CategoryRepository>,
        message_publisher: Arc<dyn ProductMessagePublisher>,
    ) -> Self {
        Self {
            product_repository,
            category_repository,
            message_publisher,
        }
    }

    // TODO: Add transactional outbox
    pub async fn execute(&self, command: UpdateProductCommand<'_>) -> anyhow::Result<ProductDto> {
        let category = self
            .category_repository
            .find_by_id(CategoryId::from_uuid(command.category_id()))?;

        let product = Product::builder()
            .id(ProductId::from_uuid(command.id()))
            .title(command.title())
            .description(command.description())
            .quantity(command.quantity())
            .price(Money::from_decimal(command.price(), iso::USD))
            .category(category)
            .build();

        let saved_product = self.product_repository.save(product)?;
        tracing::info!(
            "Product with id: {} updated",
            saved_product.id().as_uuid().to_string(),
        );

        let product_dto = ProductDto::from(&saved_product);
        self.message_publisher
            .publish_updated(ProductUpdatedEvent::new(saved_product))?;

        Ok(product_dto)
    }
}

pub struct DeleteProductCommandHandler {
    repository: Arc<dyn ProductRepository>,
    message_publisher: Arc<dyn ProductMessagePublisher>,
}

impl DeleteProductCommandHandler {
    pub fn new(
        repository: Arc<dyn ProductRepository>,
        message_publisher: Arc<dyn ProductMessagePublisher>,
    ) -> Self {
        Self {
            repository,
            message_publisher,
        }
    }

    // TODO: Add transactional outbox
    pub async fn execute(&self, command: DeleteProductCommand) -> anyhow::Result<()> {
        let product_id = ProductId::from_uuid(command.id());

        self.repository.delete(product_id)?;
        tracing::info!("Product with id: {} deleted", command.id().to_string());

        self.message_publisher
            .publish_deleted(ProductDeletedEvent::new(product_id))
    }
}
