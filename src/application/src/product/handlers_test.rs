#[cfg(test)]
mod tests {
    use crate::category::repositories::MockCategoryRepository;
    use crate::ports::output::publishers::MockProductMessagePublisher;
    use crate::product::commands::{
        CreateProductCommand, DeleteProductCommand, UpdateProductCommand,
    };
    use crate::product::handlers::{
        CreateProductCommandHandler, DeleteProductCommandHandler, FindProductQueryHandler,
        ListAllProductQueryHandler, UpdateProductCommandHandler,
    };
    use crate::product::queries::FindProductQuery;
    use crate::product::repositories::MockProductRepository;
    use domain::category::entity::Category;
    use domain::product::entity::Product;
    use mockall::predicate;
    use rust_decimal::dec;
    use rusty_money::{Money, iso};
    use shared::domain::value_objects::{CategoryId, ProductId};
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_list_all_product_query_handler_execute() -> anyhow::Result<()> {
        let mut mock_repository = MockProductRepository::new();

        let first_category = Category::new(
            CategoryId::new(),
            "Electronics".to_string(),
            "Electronic devices and gadgets".to_string(),
        );
        let second_category = Category::new(
            CategoryId::new(),
            "Home Appliances".to_string(),
            "Appliances for home use".to_string(),
        );

        let expected_products = vec![
            Product::new(
                ProductId::new(),
                "Laptop".to_string(),
                "A high-performance laptop".to_string(),
                15,
                Money::from_decimal(dec!(30.5), iso::USD),
                first_category,
            ),
            Product::new(
                ProductId::new(),
                "Smartphone".to_string(),
                "A latest model smartphone".to_string(),
                7,
                Money::from_decimal(dec!(13.75), iso::USD),
                second_category,
            ),
        ];

        mock_repository
            .expect_list_all()
            .once()
            .returning(move || Ok(expected_products.clone()));

        let handler = ListAllProductQueryHandler::new(Arc::new(mock_repository));
        let product_dtos = handler.execute().await?;

        assert_eq!(2, product_dtos.len());

        assert_eq!("Laptop", product_dtos[0].title());
        assert_eq!("A high-performance laptop", product_dtos[0].description());
        assert_eq!(15, product_dtos[0].quantity());
        assert_eq!("$30.50", product_dtos[0].price().to_string());
        assert_eq!("Electronics", product_dtos[0].category().title());
        assert_eq!(
            "Electronic devices and gadgets",
            product_dtos[0].category().description(),
        );

        assert_eq!("Smartphone", product_dtos[1].title());
        assert_eq!("A latest model smartphone", product_dtos[1].description());
        assert_eq!(7, product_dtos[1].quantity());
        assert_eq!("$13.75", product_dtos[1].price().to_string());
        assert_eq!("Home Appliances", product_dtos[1].category().title());
        assert_eq!(
            "Appliances for home use",
            product_dtos[1].category().description(),
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_find_product_query_handler_execute() -> anyhow::Result<()> {
        let mut mock_repository = MockProductRepository::new();

        let category = Category::new(
            CategoryId::new(),
            "Electronics".to_string(),
            "Electronic devices and gadgets".to_string(),
        );

        let product_id = ProductId::new();
        let expected_product = Product::new(
            product_id,
            "Laptop".to_string(),
            "A high-performance laptop".to_string(),
            15,
            Money::from_str("30.5", iso::USD)?,
            category,
        );

        mock_repository
            .expect_find_by_id()
            .once()
            .with(predicate::eq(product_id))
            .returning(move |_| Ok(expected_product.clone()));

        let handler = FindProductQueryHandler::new(Arc::new(mock_repository));
        let product_dto = handler
            .execute(FindProductQuery::new(Option::from(*product_id.as_uuid())))
            .await?;

        assert_eq!("Laptop", product_dto.title());
        assert_eq!("A high-performance laptop", product_dto.description());
        assert_eq!(15, product_dto.quantity());
        assert_eq!("$30.50", product_dto.price().to_string());
        assert_eq!("Electronics", product_dto.category().title());
        assert_eq!(
            "Electronic devices and gadgets",
            product_dto.category().description(),
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_create_product_command_handler_execute() -> anyhow::Result<()> {
        let mut mock_category_repository = MockCategoryRepository::new();
        let mut mock_product_repository = MockProductRepository::new();
        let mut mock_message_publisher = MockProductMessagePublisher::new();

        let category_id = CategoryId::new();

        mock_category_repository
            .expect_find_by_id()
            .once()
            .with(predicate::eq(category_id))
            .returning(move |_| {
                Ok(Category::new(
                    CategoryId::new(),
                    "Electronics".to_string(),
                    "Electronic devices and gadgets".to_string(),
                ))
            });

        mock_product_repository
            .expect_save()
            .once()
            .withf(move |product| {
                product.title() == "Laptop"
                    && product.description() == "A high-performance laptop"
                    && product.quantity() == 15
                    && product.price().to_string() == "$30.50"
                    && product.category().title() == "Electronics"
                    && product.category().description() == "Electronic devices and gadgets"
            })
            .returning(Ok);

        mock_message_publisher
            .expect_publish_created()
            .once()
            .withf(|event| {
                event.product().title() == "Laptop"
                    && event.product().description() == "A high-performance laptop"
                    && event.product().quantity() == 15
                    && event.product().price().to_string() == "$30.50"
                    && event.product().category().title() == "Electronics"
                    && event.product().category().description() == "Electronic devices and gadgets"
            })
            .returning(|_| Ok(()));

        let command = CreateProductCommand::new(
            "Laptop",
            "A high-performance laptop",
            15,
            dec!(30.5),
            *category_id.as_uuid(),
        );

        let handler = CreateProductCommandHandler::new(
            Arc::new(mock_product_repository),
            Arc::new(mock_category_repository),
            Arc::new(mock_message_publisher),
        );

        let product_dto = handler.execute(command).await?;
        assert_eq!("Laptop", product_dto.title());
        assert_eq!("A high-performance laptop", product_dto.description());
        assert_eq!(15, product_dto.quantity());
        assert_eq!("$30.50", product_dto.price().to_string());

        assert_eq!("Electronics", product_dto.category().title());
        assert_eq!(
            "Electronic devices and gadgets",
            product_dto.category().description(),
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_create_product_command_handler_execute_with_non_existent_category()
    -> anyhow::Result<()> {
        let mut mock_category_repository = MockCategoryRepository::new();
        let mut mock_product_repository = MockProductRepository::new();
        let mut mock_message_publisher = MockProductMessagePublisher::new();

        let category_id = CategoryId::new();

        mock_category_repository
            .expect_find_by_id()
            .once()
            .with(predicate::eq(category_id))
            .returning(move |_| {
                Err(anyhow::anyhow!(
                    "Category with id {:?} not found",
                    category_id.as_uuid(),
                ))
            });

        mock_product_repository.expect_save().never();

        mock_message_publisher.expect_publish_created().never();

        let command = CreateProductCommand::new(
            "Laptop",
            "A high-performance laptop",
            15,
            dec!(30.5),
            *category_id.as_uuid(),
        );

        let handler = CreateProductCommandHandler::new(
            Arc::new(mock_product_repository),
            Arc::new(mock_category_repository),
            Arc::new(mock_message_publisher),
        );

        let result = handler.execute(command).await;
        assert!(result.is_err());

        let message = result.unwrap_err().to_string();
        assert_eq!(
            format!("Category with id {:?} not found", category_id.as_uuid()).as_str(),
            message
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_create_product_command_handler_execute_with_failed_save() -> anyhow::Result<()> {
        let mut mock_category_repository = MockCategoryRepository::new();
        let mut mock_product_repository = MockProductRepository::new();
        let mut mock_message_publisher = MockProductMessagePublisher::new();

        let category_id = CategoryId::new();

        mock_category_repository
            .expect_find_by_id()
            .once()
            .with(predicate::eq(category_id))
            .returning(move |_| {
                Ok(Category::new(
                    CategoryId::new(),
                    "Electronics".to_string(),
                    "Electronic devices and gadgets".to_string(),
                ))
            });

        mock_product_repository
            .expect_save()
            .once()
            .withf(move |product| {
                product.title() == "Laptop"
                    && product.description() == "A high-performance laptop"
                    && product.quantity() == 15
                    && product.price().to_string() == "$30.50"
                    && product.category().title() == "Electronics"
                    && product.category().description() == "Electronic devices and gadgets"
            })
            .returning(move |_| Err(anyhow::anyhow!("Failed to save product to the repository",)));

        mock_message_publisher.expect_publish_created().never();

        let command = CreateProductCommand::new(
            "Laptop",
            "A high-performance laptop",
            15,
            dec!(30.5),
            *category_id.as_uuid(),
        );

        let handler = CreateProductCommandHandler::new(
            Arc::new(mock_product_repository),
            Arc::new(mock_category_repository),
            Arc::new(mock_message_publisher),
        );

        let result = handler.execute(command).await;
        assert!(result.is_err());

        let message = result.unwrap_err().to_string();
        assert_eq!("Failed to save product to the repository", message);

        Ok(())
    }

    #[tokio::test]
    async fn test_create_product_command_handler_execute_with_failed_publish() -> anyhow::Result<()>
    {
        let mut mock_category_repository = MockCategoryRepository::new();
        let mut mock_product_repository = MockProductRepository::new();
        let mut mock_message_publisher = MockProductMessagePublisher::new();

        let category_id = CategoryId::new();

        mock_category_repository
            .expect_find_by_id()
            .once()
            .with(predicate::eq(category_id))
            .returning(move |_| {
                Ok(Category::new(
                    CategoryId::new(),
                    "Electronics".to_string(),
                    "Electronic devices and gadgets".to_string(),
                ))
            });

        mock_product_repository
            .expect_save()
            .once()
            .withf(move |product| {
                product.title() == "Laptop"
                    && product.description() == "A high-performance laptop"
                    && product.quantity() == 15
                    && product.price().to_string() == "$30.50"
                    && product.category().title() == "Electronics"
                    && product.category().description() == "Electronic devices and gadgets"
            })
            .returning(Ok);

        mock_message_publisher
            .expect_publish_created()
            .once()
            .withf(|event| {
                event.product().title() == "Laptop"
                    && event.product().description() == "A high-performance laptop"
                    && event.product().quantity() == 15
                    && event.product().price().to_string() == "$30.50"
                    && event.product().category().title() == "Electronics"
                    && event.product().category().description() == "Electronic devices and gadgets"
            })
            .returning(move |_| Err(anyhow::anyhow!("Failed to publish product created event",)));

        let command = CreateProductCommand::new(
            "Laptop",
            "A high-performance laptop",
            15,
            dec!(30.5),
            *category_id.as_uuid(),
        );

        let handler = CreateProductCommandHandler::new(
            Arc::new(mock_product_repository),
            Arc::new(mock_category_repository),
            Arc::new(mock_message_publisher),
        );

        let result = handler.execute(command).await;
        assert!(result.is_err());

        let message = result.unwrap_err().to_string();
        assert_eq!("Failed to publish product created event", message);

        Ok(())
    }

    #[tokio::test]
    async fn test_update_product_command_handler_execute() -> anyhow::Result<()> {
        let mut mock_category_repository = MockCategoryRepository::new();
        let mut mock_product_repository = MockProductRepository::new();
        let mut mock_message_publisher = MockProductMessagePublisher::new();

        let product_id = Uuid::new_v4();
        let category_id = Uuid::new_v4();

        mock_category_repository
            .expect_find_by_id()
            .once()
            .with(predicate::eq(CategoryId::from_uuid(category_id)))
            .returning(move |_| {
                Ok(Category::new(
                    CategoryId::new(),
                    "Electronics".to_string(),
                    "Electronic devices and gadgets".to_string(),
                ))
            });

        mock_product_repository
            .expect_save()
            .once()
            .withf(move |product| {
                product.id() == ProductId::from_uuid(product_id)
                    && product.title() == "Laptop"
                    && product.description() == "A high-performance laptop"
                    && product.quantity() == 15
                    && product.price().to_string() == "$30.50"
                    && product.category().title() == "Electronics"
                    && product.category().description() == "Electronic devices and gadgets"
            })
            .returning(Ok);

        mock_message_publisher
            .expect_publish_updated()
            .once()
            .withf(move |event| {
                event.product().id() == ProductId::from_uuid(product_id)
                    && event.product().title() == "Laptop"
                    && event.product().description() == "A high-performance laptop"
                    && event.product().quantity() == 15
                    && event.product().price().to_string() == "$30.50"
                    && event.product().category().title() == "Electronics"
                    && event.product().category().description() == "Electronic devices and gadgets"
            })
            .returning(|_| Ok(()));

        let command = UpdateProductCommand::new(
            product_id,
            "Laptop",
            "A high-performance laptop",
            15,
            dec!(30.5),
            category_id,
        );

        let handler = UpdateProductCommandHandler::new(
            Arc::new(mock_product_repository),
            Arc::new(mock_category_repository),
            Arc::new(mock_message_publisher),
        );

        let product_dto = handler.execute(command).await?;
        assert_eq!("Laptop", product_dto.title());
        assert_eq!("A high-performance laptop", product_dto.description());
        assert_eq!(15, product_dto.quantity());
        assert_eq!("$30.50", product_dto.price().to_string());

        assert_eq!("Electronics", product_dto.category().title());
        assert_eq!(
            "Electronic devices and gadgets",
            product_dto.category().description(),
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_product_command_handler_execute() -> anyhow::Result<()> {
        let mut mock_repository = MockProductRepository::new();
        let mut mock_message_publisher = MockProductMessagePublisher::new();

        let product_id = Uuid::new_v4();

        let command = DeleteProductCommand::new(product_id);

        mock_repository.expect_delete().once().returning(|_| Ok(()));

        mock_message_publisher
            .expect_publish_deleted()
            .once()
            .withf(move |event| event.product_id() == ProductId::from_uuid(product_id))
            .returning(|_| Ok(()));

        let handler = DeleteProductCommandHandler::new(
            Arc::new(mock_repository),
            Arc::new(mock_message_publisher),
        );

        handler.execute(command).await?;

        Ok(())
    }
}
