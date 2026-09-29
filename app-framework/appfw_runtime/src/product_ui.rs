//! Runtime-owned product SPA static serving.

use axum::{
    extract::OriginalUri,
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use std::{
    env,
    path::{Path, PathBuf},
    sync::Arc,
};
use tower_http::services::ServeDir;
use tracing::warn;

pub const PRODUCT_UI_DIST_DIR_ENV_VAR: &str = "APP_PRODUCT_UI_DIST_DIR";
pub const PRODUCT_UI_INDEX_PATH: &str = "/";
pub const PRODUCT_UI_ASSETS_PATH: &str = "/assets";

const DEFAULT_RESERVED_PREFIXES: &[&str] = &[
    "/admin",
    "/mcp",
    "/info",
    "/health",
    "/readyz",
    "/livez",
    "/metrics",
    "/metrics.json",
];

pub fn product_ui_dist_dir(product_manifest_dir: impl AsRef<Path>) -> PathBuf {
    env::var_os(PRODUCT_UI_DIST_DIR_ENV_VAR)
        .map(PathBuf::from)
        .unwrap_or_else(|| product_manifest_dir.as_ref().join("product_dist"))
}

pub fn product_ui_index_path(product_manifest_dir: impl AsRef<Path>) -> PathBuf {
    product_ui_dist_dir(product_manifest_dir).join("index.html")
}

pub fn product_ui_assets_dir(product_manifest_dir: impl AsRef<Path>) -> PathBuf {
    product_ui_dist_dir(product_manifest_dir).join("assets")
}

pub fn product_ui_routes_if_present<I, S>(
    product_manifest_dir: impl AsRef<Path>,
    api_reserved_prefixes: I,
) -> Option<Router>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let index_path = product_ui_index_path(product_manifest_dir.as_ref());
    if !index_path.is_file() {
        warn!(
            path = %index_path.display(),
            "product UI bundle not mounted because index.html was not found"
        );
        return None;
    }

    Some(product_ui_routes(
        product_manifest_dir,
        api_reserved_prefixes,
    ))
}

pub fn product_ui_routes<I, S>(
    product_manifest_dir: impl AsRef<Path>,
    api_reserved_prefixes: I,
) -> Router
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let index_path = Arc::new(product_ui_index_path(product_manifest_dir.as_ref()));
    let assets_dir = product_ui_assets_dir(product_manifest_dir.as_ref());
    let reserved_prefixes = Arc::new(reserved_prefixes(api_reserved_prefixes));

    Router::new()
        .route(
            PRODUCT_UI_INDEX_PATH,
            get({
                let index_path = Arc::clone(&index_path);
                move || product_ui_index_response(index_path)
            }),
        )
        .nest_service(PRODUCT_UI_ASSETS_PATH, ServeDir::new(assets_dir))
        .fallback(get({
            let index_path = Arc::clone(&index_path);
            let reserved_prefixes = Arc::clone(&reserved_prefixes);
            move |uri: OriginalUri| product_ui_fallback(uri.0, index_path, reserved_prefixes)
        }))
}

pub async fn product_ui_index_response(index_path: Arc<PathBuf>) -> Response {
    match tokio::fs::read_to_string(index_path.as_ref()).await {
        Ok(html) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            Html(html),
        )
            .into_response(),
        Err(err) => (
            StatusCode::SERVICE_UNAVAILABLE,
            format!(
                "product UI bundle not found at {} ({err}). Run `cd frontend && npm install && npm run build`.",
                index_path.display()
            ),
        )
            .into_response(),
    }
}

async fn product_ui_fallback(
    uri: Uri,
    index_path: Arc<PathBuf>,
    reserved_prefixes: Arc<Vec<String>>,
) -> Response {
    if is_reserved_path(uri.path(), &reserved_prefixes) {
        return StatusCode::NOT_FOUND.into_response();
    }

    product_ui_index_response(index_path).await
}

fn reserved_prefixes<I, S>(api_reserved_prefixes: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut prefixes = DEFAULT_RESERVED_PREFIXES
        .iter()
        .map(|prefix| (*prefix).to_string())
        .collect::<Vec<_>>();
    for prefix in api_reserved_prefixes {
        let prefix = prefix.as_ref().trim();
        if prefix.is_empty() {
            continue;
        }
        let normalized = if prefix.starts_with('/') {
            prefix.to_string()
        } else {
            format!("/{prefix}")
        };
        if !prefixes.iter().any(|existing| existing == &normalized) {
            prefixes.push(normalized);
        }
    }
    prefixes
}

fn is_reserved_path(path: &str, reserved_prefixes: &[String]) -> bool {
    reserved_prefixes.iter().any(|prefix| {
        let prefix = prefix.trim_end_matches('/');
        path == prefix || path.starts_with(&format!("{prefix}/"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Method, Request},
    };
    use std::fs;
    use tower::ServiceExt;

    static PRODUCT_UI_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn product_ui_dist_dir_defaults_under_product_manifest_dir() {
        let _guard = PRODUCT_UI_ENV_LOCK.lock().expect("product UI env lock");
        let previous = env::var_os(PRODUCT_UI_DIST_DIR_ENV_VAR);
        env::remove_var(PRODUCT_UI_DIST_DIR_ENV_VAR);

        let dist_dir = product_ui_dist_dir("/tmp/product-backend");

        assert_eq!(dist_dir, PathBuf::from("/tmp/product-backend/product_dist"));
        restore_env(PRODUCT_UI_DIST_DIR_ENV_VAR, previous);
    }

    #[test]
    fn product_ui_assets_dir_uses_dist_dir_override() {
        let _guard = PRODUCT_UI_ENV_LOCK.lock().expect("product UI env lock");
        let previous = env::var_os(PRODUCT_UI_DIST_DIR_ENV_VAR);
        env::set_var(PRODUCT_UI_DIST_DIR_ENV_VAR, "/tmp/product-bundle");

        let assets_dir = product_ui_assets_dir("/tmp/product-backend");

        assert_eq!(assets_dir, PathBuf::from("/tmp/product-bundle/assets"));
        restore_env(PRODUCT_UI_DIST_DIR_ENV_VAR, previous);
    }

    #[test]
    fn product_ui_routes_if_present_skips_missing_bundle() {
        let product_dir = temp_product_dir("missing");

        let routes = product_ui_routes_if_present(&product_dir, ["/crm"]);

        assert!(routes.is_none());
        let _ = fs::remove_dir_all(product_dir);
    }

    #[tokio::test]
    async fn product_ui_serves_index_and_reserves_runtime_paths() {
        let product_dir = temp_product_dir("serving");
        let dist_dir = product_dir.join("product_dist");
        let assets_dir = dist_dir.join("assets");
        fs::create_dir_all(&assets_dir).expect("create assets dir");
        fs::write(dist_dir.join("index.html"), "<main>Product app</main>").expect("write index");
        fs::write(assets_dir.join("app.js"), "console.log('ok');").expect("write asset");

        let router = product_ui_routes(&product_dir, ["/crm", "/system"]);

        assert_eq!(
            route_status(router.clone(), Method::GET, "/").await,
            StatusCode::OK
        );
        assert_eq!(
            route_status(router.clone(), Method::GET, "/accounts/123").await,
            StatusCode::OK
        );
        assert_eq!(
            route_status(router.clone(), Method::GET, "/assets/app.js").await,
            StatusCode::OK
        );
        assert_eq!(
            route_status(router.clone(), Method::GET, "/admin/missing").await,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            route_status(router.clone(), Method::GET, "/crm/missing").await,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            route_status(router, Method::GET, "/metrics/missing").await,
            StatusCode::NOT_FOUND
        );

        let _ = fs::remove_dir_all(product_dir);
    }

    async fn route_status(router: Router, method: Method, path: &str) -> StatusCode {
        router
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response")
            .status()
    }

    fn temp_product_dir(label: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!(
            "appfw-product-ui-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create product dir");
        dir
    }

    fn restore_env(name: &str, value: Option<std::ffi::OsString>) {
        match value {
            Some(value) => env::set_var(name, value),
            None => env::remove_var(name),
        }
    }
}
