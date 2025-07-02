use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArticleError {
    #[error("Could not parse frontmatter")]
    FrontMatterParse,
}
