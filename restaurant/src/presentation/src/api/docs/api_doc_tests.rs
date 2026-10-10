use crate::api::docs::open_api_docs;

#[test]
fn test_open_api_contains_every_endpoint() {
    let doc = open_api_docs();
    let mut paths: Vec<&String> = doc.paths.paths.keys().collect();
    paths.sort();

    assert_eq!(
        vec![
            "/api/health/live",
            "/api/health/ready",
            "/api/health/startup",
            "/api/info",
            "/api/order-approvals/{order_id}",
            "/api/products",
            "/api/products/{id}",
        ],
        paths,
    );
}

#[test]
fn test_open_api_documents_error_responses() {
    let doc = open_api_docs();
    let operation = doc.paths.paths["/api/order-approvals/{order_id}"]
        .get
        .as_ref()
        .unwrap();

    for code in ["200", "400", "404", "500"] {
        assert!(
            operation.responses.responses.contains_key(code),
            "missing {code} response",
        );
    }

    let schemas = &doc.components.as_ref().unwrap().schemas;
    for schema in [
        "ErrorResponse",
        "OrderApprovalDataResponse",
        "ProductListDataResponse",
    ] {
        assert!(schemas.contains_key(schema), "missing {schema} schema");
    }
}

#[test]
fn test_open_api_documents_every_order_approval_status() {
    let doc = open_api_docs();
    let schemas = &doc.components.as_ref().unwrap().schemas;
    let schema = serde_json::to_value(&schemas["OrderApprovalStatusResponse"]).unwrap();

    assert_eq!(
        serde_json::json!(["APPROVED", "REJECTED", "CANCELLED"]),
        schema["enum"],
    );
}

#[test]
fn test_open_api_uses_shared_error_schemas() {
    let doc = open_api_docs();
    let schemas = &doc.components.as_ref().unwrap().schemas;

    let error = serde_json::to_value(&schemas["ErrorResponse"]).unwrap();
    assert!(error["properties"]["errors"].is_object());
    assert!(schemas.contains_key("ValidationFieldError"));
}
