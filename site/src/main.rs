//! Website for Resources for Mac and Disk Sweep: serves the static folder plus a health check.

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
        for page in ["/resources/", "/disk-sweep/", "/icons.svg"] {
            assert_eq!(get(page).await.status(), StatusCode::OK, "{page}");
        }
        assert_eq!(get("/healthz").await.status(), StatusCode::OK);
        assert_eq!(get("/nope").await.status(), StatusCode::NOT_FOUND);
    }

    /// Every value of `attr="…"` in `html`.
    fn attrs<'a>(html: &'a str, attr: &str) -> Vec<&'a str> {
        let key = format!("{attr}=\"");
        html.split(key.as_str())
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .collect()
    }

    /// The pages reference screenshots and icons that are copied in at deploy time
    /// (vercel.json); a renamed page or a typo must fail here, not show a broken image.
    #[test]
    fn pages_reference_existing_screenshots_and_icons() {
        let site = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let shots = site.join("../docs/screenshots");
        let sprite = std::fs::read_to_string(site.join("static/icons.svg")).unwrap();
        for page in [
            "index.html",
            "resources/index.html",
            "disk-sweep/index.html",
        ] {
            let html = std::fs::read_to_string(site.join("static").join(page)).unwrap();
            let dir = attrs(&html, "data-dir").first().copied().unwrap_or("");
            let mut wanted: Vec<String> = attrs(&html, "data-shot")
                .iter()
                .map(|s| format!("{dir}{s}"))
                .collect();
            wanted.extend(
                attrs(&html, "data-theme-shot")
                    .iter()
                    .map(|s| s.to_string()),
            );
            assert!(!wanted.is_empty(), "{page} shows no screenshots");
            for shot in wanted {
                let (sub, name) = shot.rsplit_once('/').unwrap_or(("", &shot));
                for theme in ["light", "dark"] {
                    let png = shots.join(sub).join(format!("{theme}-{name}.png"));
                    assert!(png.exists(), "{page}: missing {}", png.display());
                }
            }
            for href in attrs(&html, "href") {
                if let Some(id) = href.strip_prefix("/icons.svg#") {
                    assert!(
                        sprite.contains(&format!("id=\"{id}\"")),
                        "{page}: no icon {id}"
                    );
                }
            }
        }
    }
}
