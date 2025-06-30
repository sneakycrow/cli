use crate::Article;

const DEFAULT_TITLE: &str = "title-me";

#[derive(Default)]
pub struct ArticleBuilder<S>
where
    S: ToString,
{
    title: Option<S>,
}

impl<S> ArticleBuilder<S>
where
    S: ToString,
{
    /// Builds the article with the provided data
    pub fn build(self) -> Article {
        let title = self
            .title
            .map(|t| t.to_string())
            .unwrap_or_else(|| DEFAULT_TITLE.to_string());

        Article { title }
    }

    /// Adds a title to the article.
    pub fn title(mut self, title: S) -> Self {
        self.title = Some(title);
        self
    }

    /// Adds an optional title to the article.
    pub fn maybe_title(mut self, title: Option<S>) -> Self {
        self.title = title;
        self
    }
}
