use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArticleError {
    #[error("Could not parse frontmatter: {0}")]
    FrontMatterParse(String),
    #[error("Could not parse content")]
    ContentParse,
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),
}
