use domain::aggregates::category::Category;
use domain::aggregates::product::Product;
use rusty_money::Money;
use rusty_money::iso::Currency;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CategoryDto {
    id: Uuid,
    title: String,
    description: String,
}

impl CategoryDto {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

impl From<&Category> for CategoryDto {
    fn from(category: &Category) -> Self {
        Self {
            id: category.id().into(),
            title: category.title().into(),
            description: category.description().into(),
        }
    }
}

impl From<Category> for CategoryDto {
    fn from(category: Category) -> Self {
        Self::from(&category)
    }
}

#[derive(Debug)]
pub struct ProductDto {
    id: Uuid,
    title: String,
    description: String,
    quantity: i32,
    price: Money<'static, Currency>,
    category: CategoryDto,
}

impl ProductDto {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn quantity(&self) -> i32 {
        self.quantity
    }

    pub fn price(&self) -> &Money<'static, Currency> {
        &self.price
    }

    pub fn category(&self) -> &CategoryDto {
        &self.category
    }
}

impl From<&Product> for ProductDto {
    fn from(product: &Product) -> Self {
        Self {
            id: product.id().into(),
            title: product.title().into(),
            description: product.description().into(),
            quantity: product.quantity(),
            price: *product.price(),
            category: product.category().into(),
        }
    }
}

impl From<Product> for ProductDto {
    fn from(product: Product) -> Self {
        Self::from(&product)
    }
}
