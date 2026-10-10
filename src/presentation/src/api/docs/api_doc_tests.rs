use crate::api::docs::open_api_docs;
use serde_json::Value;

/// (path, method, expected response codes) for every route registered in `config::configure`.
const EXPECTED: &[(&str, &str, &[&str])] = &[
    ("/api/customers", "post", &["201", "400", "500"]),
    ("/api/customers/{id}", "put", &["200", "400", "500"]),
    ("/api/health/startup", "get", &["200"]),
    ("/api/health/live", "get", &["200"]),
    ("/api/health/ready", "get", &["200"]),
    ("/api/info", "get", &["200", "500"]),
];

fn docs() -> anyhow::Result<Value> {
    Ok(serde_json::to_value(open_api_docs())?)
}

#[test]
fn test_every_route_is_documented_with_expected_responses() -> anyhow::Result<()> {
    let docs = docs()?;

    for (path, method, codes) in EXPECTED {
        let operation = &docs["paths"][path][method];
        assert!(
            operation.is_object(),
            "missing {method} {path} in OpenAPI docs"
        );
        assert!(
            operation["operationId"].is_string(),
            "missing operationId for {method} {path}"
        );

        let responses = operation["responses"]
            .as_object()
            .unwrap_or_else(|| panic!("missing responses for {method} {path}"));
        let mut actual: Vec<&str> = responses.keys().map(String::as_str).collect();
        actual.sort();
        let mut expected = codes.to_vec();
        expected.sort();
        assert_eq!(expected, actual, "response codes for {method} {path}");
    }

    Ok(())
}

#[test]
fn test_no_undocumented_extra_routes() -> anyhow::Result<()> {
    let docs = docs()?;
    let documented = docs["paths"].as_object().map(|p| p.len()).unwrap_or(0);
    let expected_paths: std::collections::HashSet<&str> =
        EXPECTED.iter().map(|(path, _, _)| *path).collect();

    assert_eq!(expected_paths.len(), documented);

    Ok(())
}

#[test]
fn test_error_and_envelope_schemas_are_registered() -> anyhow::Result<()> {
    let docs = docs()?;
    let schemas = docs["components"]["schemas"]
        .as_object()
        .expect("components.schemas missing");

    for name in [
        "ErrorResponse",
        "ValidationFieldError",
        "CustomerResponse",
        "CreateCustomerRequest",
        "UpdateCustomerRequest",
    ] {
        assert!(schemas.contains_key(name), "schema {name} not registered");
    }

    Ok(())
}
