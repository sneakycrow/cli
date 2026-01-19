use articles::errors::ArticleError;
use handlebars::RenderError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WebError {
    #[error("Failed loading articles: {0}")]
    ArticlesLoad(#[from] ArticleError),
    #[error("I/O Failure: {0}")]
    IO(#[from] std::io::Error),
    #[error("Handlebars render error: {0}")]
    Render(#[from] RenderError),
    #[error("Context error: {0}")]
    Context(#[from] context::SneakyContextError),
}
