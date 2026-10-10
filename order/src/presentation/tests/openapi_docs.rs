use serde_json::Value;

fn openapi() -> anyhow::Result<Value> {
    Ok(serde_json::to_value(
        presentation::api::docs::open_api_docs(),
    )?)
}

fn response_codes(doc: &Value, path: &str, method: &str) -> Vec<String> {
    let mut codes: Vec<String> = doc["paths"][path][method]["responses"]
        .as_object()
        .map(|responses| responses.keys().cloned().collect())
        .unwrap_or_default();
    codes.sort();
    codes
}

/// Keeps the OpenAPI document in sync with the routes registered in `api::configure`.
#[test]
fn test_every_route_is_documented_with_its_responses() -> anyhow::Result<()> {
    let doc = openapi()?;

    let expected = [
        (
            "/api/orders",
            "post",
            "create_order",
            vec!["201", "400", "422", "500"],
        ),
        (
            "/api/orders/{tracking_id}",
            "get",
            "track_order",
            vec!["200", "400", "404", "500"],
        ),
        ("/api/health/startup", "get", "health_startup", vec!["200"]),
        ("/api/health/live", "get", "health_live", vec!["200"]),
        ("/api/health/ready", "get", "health_ready", vec!["200"]),
        ("/api/info", "get", "app_info", vec!["200", "500"]),
    ];

    for (path, method, operation_id, codes) in &expected {
        assert_eq!(
            *operation_id, doc["paths"][path][method]["operationId"],
            "{method} {path}",
        );
        assert_eq!(
            *codes,
            response_codes(&doc, path, method),
            "{method} {path}",
        );
    }

    let documented_paths = doc["paths"].as_object().map(|p| p.len()).unwrap_or(0);
    assert_eq!(expected.len(), documented_paths, "unexpected extra paths");

    Ok(())
}

#[test]
fn test_schemas_are_registered() -> anyhow::Result<()> {
    let doc = openapi()?;
    let schemas = doc["components"]["schemas"]
        .as_object()
        .cloned()
        .unwrap_or_default();

    for schema in [
        "CreateOrderRequest",
        "CreateOrderItem",
        "ErrorResponse",
        "ValidationFieldError",
        "OrderStatusResponse",
        "OrderTrackingResponse",
        "OrderItemResponse",
    ] {
        assert!(schemas.contains_key(schema), "missing schema {schema}");
    }

    let status_values = &schemas["OrderStatusResponse"]["enum"];
    assert_eq!(
        serde_json::json!(["PENDING", "APPROVED", "PAID", "CANCELLING", "CANCELLED"]),
        *status_values,
    );

    let track = &doc["paths"]["/api/orders/{tracking_id}"]["get"];
    assert_eq!("tracking_id", track["parameters"][0]["name"]);
    assert_eq!("path", track["parameters"][0]["in"]);
    assert_eq!("uuid", track["parameters"][0]["schema"]["format"]);

    Ok(())
}
