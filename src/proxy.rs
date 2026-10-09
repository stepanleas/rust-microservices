use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Instant};

use axum::{
    body::Body,
    extract::{ConnectInfo, Path, Request, State},
    response::{IntoResponse, Response},
};
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header};

use crate::{circuit_breaker::CircuitBreaker, config::Settings};

pub const RESPONSE_TIME: &str = "x-response-time";
const FALLBACK_MESSAGE: &str =
    "An error has occurred. Please try after some time or contact support team.";

pub struct Upstream {
    base_url: String,
    breaker: CircuitBreaker,
}

pub struct ProxyState {
    client: reqwest::Client,
    upstreams: HashMap<String, Upstream>,
    response_timeout: std::time::Duration,
}

impl ProxyState {
    pub fn new(settings: &Settings) -> anyhow::Result<Arc<Self>> {
        let client = reqwest::Client::builder()
            .connect_timeout(settings.connect_timeout)
            // The gateway must hand redirects back to the caller, not follow them.
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let upstreams = settings
            .routes
            .iter()
            .map(|route| {
                let upstream = Upstream {
                    base_url: route.upstream.clone(),
                    breaker: CircuitBreaker::new(
                        format!("{}CircuitBreaker", route.name),
                        settings.circuit_breaker.clone(),
                    ),
                };
                (route.name.clone(), upstream)
            })
            .collect();

        Ok(Arc::new(Self {
            client,
            upstreams,
            response_timeout: settings.response_timeout,
        }))
    }
}

/// Handles `/shop/{service}` and `/shop/{service}/{*rest}` by forwarding the
/// request to `<upstream>/{rest}` with the prefix stripped.
pub async fn forward(
    State(state): State<Arc<ProxyState>>,
    Path(params): Path<HashMap<String, String>>,
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    request: Request,
) -> Response {
    let started = Instant::now();
    let service = params
        .get("service")
        .map(String::as_str)
        .unwrap_or_default();
    let Some(upstream) = state.upstreams.get(service) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let Some(permit) = upstream.breaker.try_acquire() else {
        tracing::warn!(service, "circuit open, returning fallback");
        return with_response_time(fallback(), started);
    };

    let rest = params.get("rest").map(String::as_str).unwrap_or_default();
    let mut url = format!("{}/{}", upstream.base_url, rest);
    if let Some(query) = request.uri().query() {
        url.push('?');
        url.push_str(query);
    }

    let (parts, body) = request.into_parts();
    let headers = forwarded_headers(&parts.headers, client_addr, service);
    let upstream_request = state
        .client
        .request(parts.method, &url)
        .headers(headers)
        .body(reqwest::Body::wrap_stream(body.into_data_stream()));

    let response = match tokio::time::timeout(state.response_timeout, upstream_request.send()).await
    {
        Ok(Ok(response)) => {
            // Like the Spring circuit breaker filter, only transport errors and
            // timeouts count as failures; a 5xx is a valid upstream answer.
            permit.record(true);
            response
        }
        Ok(Err(error)) => {
            permit.record(false);
            tracing::error!(service, %url, %error, "upstream request failed");
            return with_response_time(fallback(), started);
        }
        Err(_) => {
            permit.record(false);
            tracing::error!(service, %url, "upstream request timed out");
            return with_response_time(fallback(), started);
        }
    };

    let mut builder = Response::builder().status(response.status());
    if let Some(headers) = builder.headers_mut() {
        copy_end_to_end_headers(response.headers(), headers);
    }
    let response = builder
        .body(Body::from_stream(response.bytes_stream()))
        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response());

    with_response_time(response, started)
}

fn fallback() -> Response {
    (StatusCode::SERVICE_UNAVAILABLE, FALLBACK_MESSAGE).into_response()
}

fn with_response_time(mut response: Response, started: Instant) -> Response {
    let elapsed = format!("{}ms", started.elapsed().as_millis());
    if let Ok(value) = HeaderValue::from_str(&elapsed) {
        response.headers_mut().insert(RESPONSE_TIME, value);
    }
    response
}

fn forwarded_headers(incoming: &HeaderMap, client_addr: SocketAddr, service: &str) -> HeaderMap {
    let mut headers = HeaderMap::with_capacity(incoming.len() + 4);
    copy_end_to_end_headers(incoming, &mut headers);
    // reqwest derives Host from the upstream URL.
    headers.remove(header::HOST);

    let client_ip = client_addr.ip().to_string();
    let forwarded_for = match incoming
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        Some(existing) => format!("{existing}, {client_ip}"),
        None => client_ip,
    };
    let forwarded = [
        ("x-forwarded-for", Some(forwarded_for)),
        ("x-forwarded-proto", Some("http".to_string())),
        ("x-forwarded-prefix", Some(format!("/shop/{service}"))),
        (
            "x-forwarded-host",
            incoming
                .get(header::HOST)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string),
        ),
    ];
    for (name, value) in forwarded {
        if let Some(value) = value.and_then(|v| HeaderValue::from_str(&v).ok()) {
            headers.insert(HeaderName::from_static(name), value);
        }
    }

    headers
}

/// Copies headers, dropping hop-by-hop ones (RFC 9110 §7.6.1), which apply to
/// a single connection and must not be forwarded by a proxy.
fn copy_end_to_end_headers(from: &HeaderMap, to: &mut HeaderMap) {
    let connection_listed: Vec<String> = from
        .get_all(header::CONNECTION)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(|name| name.trim().to_ascii_lowercase())
        .collect();

    for (name, value) in from {
        if is_hop_by_hop(name)
            || connection_listed
                .iter()
                .any(|listed| listed == name.as_str())
        {
            continue;
        }
        to.append(name.clone(), value.clone());
    }
}

fn is_hop_by_hop(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "proxy-connection"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}
