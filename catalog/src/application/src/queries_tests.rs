use crate::queries::{FindCategoryQuery, FindProductQuery};
use uuid::Uuid;

#[test]
fn test_find_category_query() {
    let category_id = Uuid::new_v4();
    let query = FindCategoryQuery::new(category_id);

    assert_eq!(category_id, query.id());
}

#[test]
fn test_find_product_query() {
    let product_id = Uuid::new_v4();
    let query = FindProductQuery::new(product_id);

    assert_eq!(product_id, query.id());
}
