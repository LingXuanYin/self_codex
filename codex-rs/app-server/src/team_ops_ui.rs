use axum::extract::ConnectInfo;
use axum::http::StatusCode;
use axum::http::header::CACHE_CONTROL;
use axum::http::header::CONTENT_TYPE;
use axum::response::Html;
use axum::response::IntoResponse;
use std::net::SocketAddr;

const NO_STORE: &str = "no-store, max-age=0";

pub(crate) async fn dashboard_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    if !team_ops_ui_allowed(addr) {
        return StatusCode::NOT_FOUND.into_response();
    }
    Html(include_str!("team_ops_ui/index.html")).into_response()
}

pub(crate) async fn script_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    if !team_ops_ui_allowed(addr) {
        return StatusCode::NOT_FOUND.into_response();
    }
    (
        [
            (CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (CACHE_CONTROL, NO_STORE),
        ],
        include_str!("team_ops_ui/app.js"),
    )
        .into_response()
}

pub(crate) async fn stylesheet_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    if !team_ops_ui_allowed(addr) {
        return StatusCode::NOT_FOUND.into_response();
    }
    (
        [
            (CONTENT_TYPE, "text/css; charset=utf-8"),
            (CACHE_CONTROL, NO_STORE),
        ],
        include_str!("team_ops_ui/styles.css"),
    )
        .into_response()
}

fn team_ops_ui_allowed(addr: SocketAddr) -> bool {
    addr.ip().is_loopback() || std::env::var_os("CODEX_TEAM_OPS_UI_ALLOW_REMOTE").is_some()
}

#[cfg(test)]
mod tests {
    use super::team_ops_ui_allowed;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn team_ops_ui_is_allowed_for_loopback_only_by_default() {
        assert!(team_ops_ui_allowed(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            8080,
        )));
        assert!(!team_ops_ui_allowed(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 4)),
            8080,
        )));
    }
}
