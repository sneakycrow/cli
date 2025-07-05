// TODO: Parse articles as static html
// TODO: Serve articles from /blog
pub mod errors;

use articles::{Article, Serialize};
use axum::{
    Router,
    extract::{FromRef, Path, State},
    response::IntoResponse,
    routing::get,
};
use axum_template::{Key, RenderHtml, engine::Engine};
use errors::WebError;
use handlebars::Handlebars;
use serde_json::{Value, json};
use std::path::PathBuf;
use tower_http::services::ServeDir;

// TODO: Replace with config loading
const ARTICLES_DIRECTORY: &str = "_posts/";

type AppEngine = Engine<Handlebars<'static>>;

#[derive(Clone, FromRef)]
struct AppState {
    engine: AppEngine,
    articles: Vec<Article>,
}

/// Serve the website
pub async fn serve() -> Result<(), WebError> {
    // load the articles
    let articles = Article::from_dir(PathBuf::from(ARTICLES_DIRECTORY)).unwrap();

    // initialize template engine
    let mut hbs = Handlebars::new();
    let _ = hbs.register_template_file("base", "./templates/partials/base.hbs");
    let _ = hbs.register_template_file("/", "./templates/index.hbs");
    let _ = hbs.register_template_file("/blog", "./templates/blog.hbs");
    let _ = hbs.register_template_file("post", "./templates/post.hbs");

    // construct app state
    let app_state = AppState {
        engine: Engine::from(hbs),
        articles,
    };

    // build the router
    let router = Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        .route("/", get(root))
        .route("/blog", get(serve_blog_index))
        .route("/blog/{year}/{month}/{day}/{*path}", get(serve_blog_post))
        .with_state(app_state);

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

#[derive(Serialize)]
struct RootTemplate {
    parent: String,
}

impl Default for RootTemplate {
    fn default() -> Self {
        Self {
            parent: "base".to_string(),
        }
    }
}

/// Root index page
async fn root(State(state): State<AppState>, Key(key): Key) -> impl IntoResponse {
    RenderHtml(key, state.engine, RootTemplate::default())
}

/// List all articles in the blog
async fn serve_blog_index(State(state): State<AppState>, Key(key): Key) -> impl IntoResponse {
    // Map each one to their url format
    // /blog/{year}/{month}/{day}/{title}
    let posts: Vec<Value> = state
        .articles
        .iter()
        .map(|a| {
            let url = format!(
                "/blog/{}/{}/{}/{}",
                a.year(),
                a.month(),
                a.day(),
                a.serialize_title()
            );
            json!({
                "title": a.title().to_string(),
                "url": url
            })
        })
        .collect();

    tracing::debug!("Posts: {posts:?}");
    RenderHtml(
        key,
        state.engine,
        json!({
            "posts": posts,
            "parent": "base"
        }),
    )
}

/// Serve blog posts from build/blog/ directory
async fn serve_blog_post(
    State(state): State<AppState>,
    Path((year, month, day, path)): Path<(u32, u32, u32, String)>,
) -> impl IntoResponse {
    // Remove leading slash if present
    let path = path.strip_prefix('/').unwrap_or(&path);

    // Construct the file path with date-prefix structure
    let file_name = format!("{:04}-{:02}-{:02}-{}", year, month, day, path);

    // Find the article with the given file name
    let Some(article) = state.articles.iter().find(|a| a.filename() == file_name) else {
        return RenderHtml(
            "error",
            state.engine,
            json!({ "title": "Oops, there was an error", "parent": "base" }),
        );
    };

    // Render the article
    RenderHtml(
        "post",
        state.engine,
        json!({
            "content": article.render_html().unwrap(),
            "title": article.title(),
            "parent": "base"
        }),
    )
}
