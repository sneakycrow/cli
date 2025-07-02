use crate::Article;
use chrono::DateTime;
use chrono_tz::Tz;

const DEFAULT_TITLE: &str = "title-me";
const DEFAULT_AUTHOR: &str = "sneakycrow";

#[derive(Default)]
pub struct ArticleBuilder<S>
where
    S: ToString,
{
    title: Option<S>,
    content: Option<String>,
    date: Option<DateTime<Tz>>,
    author: Option<String>,
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
            .unwrap_or(DEFAULT_TITLE.to_string());

        let content = self.content.unwrap_or_default();

        let pacific_time = self
            .date
            .unwrap_or(chrono::Utc::now().with_timezone(&chrono_tz::US::Pacific));

        let author = self.author.unwrap_or(DEFAULT_AUTHOR.to_string());

        Article {
            title,
            content,
            date: pacific_time,
            author,
        }
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

    /// Adds content to the article.
    pub fn content(mut self, content: String) -> Self {
        self.content = Some(content);
        self
    }

    /// Adds optional content to the article.
    pub fn maybe_content(mut self, content: Option<String>) -> Self {
        self.content = content;
        self
    }
}
