// TODO: Parse articles as static html
// TODO: Serve articles from /blog
pub mod errors;

use articles::Article;
use axum::{Router, extract::FromRef};
use chrono::DateTime;
use chrono_tz::Tz;
use context::{DEFAULT_CONFIG_FILE, SneakyContext};
use errors::WebError;
use handlebars::Handlebars;
use serde::Serialize;
use serde_json::json;
use std::path::PathBuf;
use tower_http::services::ServeDir;

const SOURCE_ARTICLES_DIR: &str = "_posts/";
const BUILD_DIR: &str = "build";

#[derive(Serialize)]
struct Post {
    pub title: String,
    pub author: String,
    pub date: DateTime<Tz>,
    pub content: String,
    pub url: String,
    pub filename: String,
}

impl From<Article> for Post {
    fn from(article: Article) -> Self {
        let content = article
            .clone()
            .render_html()
            .expect("Could not render article html");

        Post {
            url: format!("/blog/{}", article.filename()),
            filename: article.filename(),
            title: article.title,
            author: article.author,
            date: article.date,
            content: content,
        }
    }
}

#[derive(Clone, FromRef)]
pub struct AppState {
    articles: Vec<Article>,
    context: SneakyContext,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            articles: Article::from_dir(PathBuf::from(SOURCE_ARTICLES_DIR)).unwrap_or_default(),
            context: SneakyContext::from_file(DEFAULT_CONFIG_FILE).unwrap_or_default(),
        }
    }
}

/// Serve the website
pub async fn serve() -> Result<(), WebError> {
    let state = AppState::default();

    // build the static parts of the site
    build(&state)?;

    // build the router
    let router = Router::new().fallback_service(ServeDir::new(format!("{BUILD_DIR}")));

    // run the router
    let port = 3000;
    let host = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&host)
        .await
        .expect("Could not bind the listener");

    tracing::info!("Listening on http://{}", host);
    axum::serve(listener, router)
        .await
        .expect("Could not serve the listener");

    Ok(())
}

/// Pre-render statically served content
fn prerender(state: &AppState) -> Result<(), WebError> {
    // load the articles
    tracing::debug!("loading articles");
    let posts: Vec<Post> = state.articles.iter().map(|a| a.to_owned().into()).collect();

    // initialize template engine
    tracing::debug!("initializing template engine");
    let mut hbs = Handlebars::new();
    let _ = hbs.register_template_file("base", "./templates/base.hbs");
    let _ = hbs.register_template_file("index", "./templates/index.hbs");
    let _ = hbs.register_template_file("blog_index", "./templates/blog.hbs");
    let _ = hbs.register_template_file("post", "./templates/post.hbs");

    // create the build dir
    tracing::debug!("making sure the build directories exist");
    let build_dir = PathBuf::from(BUILD_DIR);
    if !build_dir.exists() {
        std::fs::create_dir(&build_dir).expect("Could not create build directory");
    }

    // render the index page
    tracing::debug!("rendering index page");
    let index_html = hbs.render(
        "index",
        &json!({
            "parent": "base"
        }),
    )?;
    std::fs::write(PathBuf::from(format!("{BUILD_DIR}/index.html")), index_html)?;

    // create the blog dir
    tracing::debug!("making sure the blog directory exists");
    let blog_dir = PathBuf::from(format!("{BUILD_DIR}/blog"));
    if !blog_dir.exists() {
        std::fs::create_dir(&blog_dir).expect("Could not create blog directory");
    }

    // render the blog index
    tracing::debug!("rendering blog index page");
    let blog_index_html = hbs.render(
        "blog_index",
        &json!({
            "parent": "base",
            "posts": &posts
        }),
    )?;
    std::fs::write(
        PathBuf::from(format!("{BUILD_DIR}/blog/index.html")),
        blog_index_html,
    )?;

    // render the posts
    tracing::debug!("rendering blog posts");
    for post in posts {
        // create the post directory
        let post_dir = PathBuf::from(format!("{BUILD_DIR}/blog/{}", post.filename));
        std::fs::create_dir_all(&post_dir)?;

        // render the html
        tracing::debug!("rendering article: {}", post.filename);
        let html = hbs.render(
            "post",
            &json!({
                "parent": "base",
                "title": &post.title,
                "content": &post.content
            }),
        )?;

        // save the html
        let build_path = PathBuf::from(format!("{BUILD_DIR}/blog/{}/index.html", post.filename));
        std::fs::write(&build_path, html)?;
    }

    Ok(())
}

/// Copies the assets directory to the build directory
fn copy_static_assets(_state: &AppState) -> Result<(), WebError> {
    let assets_dir = PathBuf::from("assets");
    let build_assets_dir = PathBuf::from(format!("{BUILD_DIR}/assets"));

    if !assets_dir.exists() {
        // No assets to copy, bail out
        tracing::debug!("No assets directory found, skipping");
        return Ok(());
    }

    std::fs::create_dir_all(&build_assets_dir)?;

    for entry in std::fs::read_dir(&assets_dir)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = build_assets_dir.join(path.file_name().unwrap());

        if path.is_dir() {
            std::fs::create_dir_all(&dest_path)?;
        } else {
            std::fs::copy(&path, &dest_path)?;
        }
    }

    Ok(())
}

/// Builds the static parts of the website
pub fn build(state: &AppState) -> Result<(), WebError> {
    // copy static assets
    copy_static_assets(&state)?;

    // prerender static content
    prerender(&state)?;

    Ok(())
}
