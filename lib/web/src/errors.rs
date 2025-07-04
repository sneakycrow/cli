use articles::errors::ArticleError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WebError {
    #[error("Failed loading articles: {0}")]
    ArticlesLoad(#[from] ArticleError),
    #[error("I/O Failure: {0}")]
    IO(#[from] std::io::Error),
}
