use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArticleError {
    #[error("Could not parse frontmatter")]
    FrontMatterParse,
    #[error("Could not save article to file")]
    SaveFile,
}
