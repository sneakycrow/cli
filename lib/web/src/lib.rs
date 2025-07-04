// TODO: Parse articles as static html
// TODO: Serve articles from /blog

use axum::{Router, routing::get};

/// Serve the website
pub async fn serve() {
    // build the router
    let router = Router::new().route("/", get(root));

    // run the router
    let port = 3000;
    let host = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&host)
        .await
        .expect("Could not bind the listener");

    tracing::info!("Listening on http://{}", host);
    axum::serve(listener, router)
        .await
        .expect("Could not serve the listener")
}

/// Root index page
async fn root() -> &'static str {
    "Hello from Sneaky Crow"
}
