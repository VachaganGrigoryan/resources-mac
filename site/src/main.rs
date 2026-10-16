//! Project website for Resources for Mac: serves the static folder plus a health check.

use axum::{Router, http::HeaderValue, http::header, routing::get};
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

/// Static pages from `dir`, `/healthz` for the deploy check, and strict security headers.
fn app(dir: &str) -> Router {
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .fallback_service(ServeDir::new(dir))
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("default-src 'self'; frame-ancestors 'none'"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let dir = std::env::var("SITE_DIR").unwrap_or_else(|_| "static".into());
    let addr = std::env::var("SITE_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("serving {dir} on {addr}");
    axum::serve(listener, app(&dir)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, http::StatusCode};
    use tower::ServiceExt;

    async fn get(path: &str) -> axum::response::Response {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/static");
        app(dir)
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn serves_index_health_and_404() {
        let index = get("/").await;
        assert_eq!(index.status(), StatusCode::OK);
        assert!(
            index
                .headers()
                .contains_key(header::CONTENT_SECURITY_POLICY)
        );
        assert_eq!(get("/healthz").await.status(), StatusCode::OK);
        assert_eq!(get("/nope").await.status(), StatusCode::NOT_FOUND);
    }
}
