use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArticleError {
    #[error("Could not parse frontmatter")]
    FrontMatterParse,
    #[error("Could not parse content")]
    ContentParse,
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),
}
