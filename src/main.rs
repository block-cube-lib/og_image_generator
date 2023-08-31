use anyhow::{anyhow, Context as _, Result};
use axum::{
    extract::Query,
    http::{request, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use easy_scraper::Pattern;
use reqwest::Url;
use serde::Deserialize;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {addr}");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn root(Query(params): Query<Params>) -> Response {
    let Params { encoded_url } = params;
    let Some(encoded_url) = encoded_url else {
        return (StatusCode::BAD_REQUEST, "query parameter error: encoded_url is empty").into_response();
    };
    let Ok(url) = base64::decode(&encoded_url) else {
        return (StatusCode::BAD_REQUEST, "query parameter error: failed decode encoded_url").into_response();
    };
    let Ok(url) = String::from_utf8(url) else {
        return (StatusCode::BAD_REQUEST, "query parameter error: failed convert to utf8").into_response();
    };

    let Ok(response) = reqwest::get(url.clone()).await else {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("get url response error. url = {url}")).into_response();
    };
    let Ok(html) = response.text().await else {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("failed get url content. url = {url}")).into_response();
    };

    let og_info = get_og_info(&html);
    if let Err(e) = og_info {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", e)).into_response();
    };

    let og_info = og_info.unwrap();

    (StatusCode::OK, format!("{og_info:?}")).into_response()
}

fn get_og_info(html: &str) -> Result<OgInfo> {
    let title = get_og_title(html)?;
    let base_image_url = get_og_base_image_url(html)?;
    let icon_image_url = get_og_option_content_url("og-icon-image", html)?;
    let thumbnail_image_url = get_og_option_content_url("og-thumbnail-image", html)?;
    let user_dictionary_url = get_og_option_content_url("og-user-dictionary", html)?;

    let og_info = OgInfo {
        title,
        base_image_url,
        icon_image_url,
        thumbnail_image_url,
        user_dictionary_url,
    };
    Ok(og_info)
}

fn get_og_title(html: &str) -> Result<String> {
    let title_pattern = Pattern::new(r#"<meta property="og:title" content="{{title}}">"#)
        .map_err(|e| anyhow!(e))?;
    let title = title_pattern
        .matches(html)
        .first()
        .context("og:title is not found")?["title"]
        .clone();
    Ok(title)
}

fn get_og_base_image_url(html: &str) -> Result<Url> {
    let base_image_pattern =
        Pattern::new(r#"<meta name="og-base-image" content="{{base_image_url}}">"#)
            .map_err(|e| anyhow!(e))?;

    let base_image_url = base_image_pattern
        .matches(html)
        .first()
        .context("og-base-image is not found")?["base_image_url"]
        .parse::<Url>()?;
    Ok(base_image_url)
}

fn get_og_option_content_url(meta_name: &str, html: &str) -> Result<Option<Url>> {
    let pattern_text = format!(r#"<meta name="{meta_name}" content="{{{{content}}}}">"#);
    let pattern = Pattern::new(&pattern_text).map_err(|e| anyhow!(e))?;
    let matches = pattern.matches(html);
    if let Some(content) = matches.first() {
        Ok(Some(content["content"].parse()?))
    } else {
        Ok(None)
    }
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

#[derive(Debug, Deserialize)]
struct Params {
    encoded_url: Option<String>,
}

#[derive(Debug)]
struct OgInfo {
    title: String,
    base_image_url: Url,
    icon_image_url: Option<Url>,
    thumbnail_image_url: Option<Url>,
    user_dictionary_url: Option<Url>,
}
