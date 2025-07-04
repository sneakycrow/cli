// TODO: Parse articles as static html
// TODO: Serve articles from /blog
pub mod errors;

use articles::Article;
use axum::{
    Router,
    extract::Path,
    http::{Response, StatusCode, header},
    response::IntoResponse,
    routing::get,
};
use errors::WebError;
use std::{io::ErrorKind, path::PathBuf};

// TODO: Replace with config loading
const ARTICLES_DIRECTORY: &str = "_posts/";
const DEFAULT_BUILD_DIR: &str = "build/";
const DEFAULT_POSTS_DIR: &str = "blog/";

/// Serve the website
pub async fn serve() -> Result<(), WebError> {
    // pre-render the articles
    let articles_path = PathBuf::from(ARTICLES_DIRECTORY);
    build_articles(articles_path)?;

    // build the router
    let router = Router::new()
        .route("/", get(root))
        .route("/blog", get(serve_blog_index))
        .route("/blog/{year}/{month}/{day}/{*path}", get(serve_blog_post));

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

/// Root index page
async fn root() -> &'static str {
    "Hello from Sneaky Crow"
}

/// Function for loading articles from a directory and saving them as html
fn build_articles(dir: PathBuf) -> Result<(), WebError> {
    // Make sure the target directory exists
    if !dir.exists() || !dir.is_dir() {
        return Err(WebError::IO(std::io::Error::new(
            ErrorKind::NotFound,
            "Directory not found",
        )));
    }

    let post_output = PathBuf::from(DEFAULT_BUILD_DIR).join(DEFAULT_POSTS_DIR);
    // Create the output directory if it doesn't exist
    if !post_output.exists() {
        std::fs::create_dir_all(&post_output)?;
    }

    // Load articles from the directory
    let articles = Article::from_dir(dir)?;

    // Render each article to the post output as html
    for article in articles {
        let file_name = format!("{}.html", article.filename());
        let content = article.render_html()?;

        let path = post_output.join(file_name);
        std::fs::write(path, &content)?;
    }

    Ok(())
}

/// List all articles in the blog
async fn serve_blog_index() -> impl IntoResponse {
    let blog_dir = PathBuf::from(DEFAULT_BUILD_DIR).join(DEFAULT_POSTS_DIR);

    // Check if the blog directory exists
    if !blog_dir.exists() || !blog_dir.is_dir() {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Blog directory not found".to_string())
            .unwrap();
    }

    // Read all HTML files in the blog directory
    match std::fs::read_dir(&blog_dir) {
        Ok(entries) => {
            let mut html_content = String::from(
                "<html><head><title>Blog Index</title></head><body><h1>Blog Posts</h1><ul>",
            );

            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "html") {
                        if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                            // Parse the filename to extract date and post name
                            let parts: Vec<&str> = file_name.splitn(4, '-').collect();
                            if parts.len() >= 4 {
                                let year = parts[0];
                                let month = parts[1];
                                let day = parts[2];
                                let post_name = parts[3];

                                let url = format!("/blog/{}/{}/{}/{}", year, month, day, post_name);
                                html_content.push_str(&format!(
                                    "<li><a href=\"{}\">{}</a></li>",
                                    url, file_name
                                ));
                            }
                        }
                    }
                }
            }

            html_content.push_str("</ul></body></html>");

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .body(html_content)
                .unwrap()
        }
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body("Error reading blog directory".to_string())
            .unwrap(),
    }
}

/// Serve blog posts from build/blog/ directory
async fn serve_blog_post(
    Path((year, month, day, path)): Path<(u32, u32, u32, String)>,
) -> impl IntoResponse {
    // Remove leading slash if present
    let path = path.strip_prefix('/').unwrap_or(&path);

    // Construct the file path with date-prefix structure
    let file_name = format!("{:04}-{:02}-{:02}-{}.html", year, month, day, path);
    let file_path = PathBuf::from(DEFAULT_BUILD_DIR)
        .join(DEFAULT_POSTS_DIR)
        .join(file_name);

    // Try to read the file
    match std::fs::read_to_string(&file_path) {
        Ok(content) => {
            // Return the HTML content with proper content type
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .body(content)
                .unwrap()
        }
        Err(_) => {
            // Return 404 if file doesn't exist
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("Post not found".to_string())
                .unwrap()
        }
    }
}
