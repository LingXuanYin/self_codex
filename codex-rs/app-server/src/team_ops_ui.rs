use axum::http::header::CACHE_CONTROL;
use axum::http::header::CONTENT_TYPE;
use axum::response::Html;
use axum::response::IntoResponse;

const NO_STORE: &str = "no-store, max-age=0";

pub(crate) async fn dashboard_handler() -> Html<&'static str> {
    Html(include_str!("team_ops_ui/index.html"))
}

pub(crate) async fn script_handler() -> impl IntoResponse {
    (
        [
            (CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (CACHE_CONTROL, NO_STORE),
        ],
        include_str!("team_ops_ui/app.js"),
    )
}

pub(crate) async fn stylesheet_handler() -> impl IntoResponse {
    (
        [
            (CONTENT_TYPE, "text/css; charset=utf-8"),
            (CACHE_CONTROL, NO_STORE),
        ],
        include_str!("team_ops_ui/styles.css"),
    )
}
