pub mod builder;
pub mod errors;

pub use builder::ArticleBuilder;
use chrono::{DateTime, NaiveDateTime, TimeZone};
use chrono_tz::{Tz, US::Pacific};
use errors::ArticleError;
pub use serde::Serialize;

#[derive(Serialize)]
struct Frontmatter<'a> {
    title: &'a str,
    author: Option<&'a str>,
    date: &'a str,
    tags: Option<&'a [String]>,
}

#[derive(Serialize)]
pub struct Article {
    title: String,
    author: String,
    date: DateTime<Tz>,
    content: String,
}

impl TryFrom<Article> for String {
    type Error = ArticleError;

    fn try_from(value: Article) -> Result<Self, Self::Error> {
        let frontmatter = Frontmatter {
            title: &value.title,
            author: Some(&value.author),
            date: &value.date.to_rfc3339(),
            tags: None,
        };

        let frontmatter_yaml = serde_yaml::to_string(&frontmatter).map_err(|e| {
            tracing::error!("Failed to serialize frontmatter: {e}");
            ArticleError::FrontMatterParse
        })?;

        Ok(format!("---\n{}\n---\n{}", frontmatter_yaml, value.content))
    }
}

impl TryFrom<String> for Article {
    type Error = ArticleError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        // Split the file into three parts delimited by `---`
        // First part: empty string before the first delimiter
        // Second part: the frontmatter
        // Third part: the content
        let parts: Vec<&str> = value.splitn(3, "---").collect();

        // If the frontmatter is less than 3 parts we have an unexpected structure
        if parts.len() < 3 {
            return Err(ArticleError::FrontMatterParse);
        }

        // Next, parse the content
        let frontmatter_content = parts[1].trim();
        let frontmatter: serde_yaml::Value =
            serde_yaml::from_str(frontmatter_content).map_err(|e| {
                tracing::error!("Failed to parse frontmatter: {e}");
                ArticleError::FrontMatterParse
            })?;

        // Extract the metadata
        let title = Self::extract_field("title", &frontmatter)?;
        let author = Self::extract_field("author", &frontmatter)?;
        let date = Self::extract_date(&frontmatter)?;

        // Create the article
        Ok(Article {
            title: title.to_string(),
            author: author.to_string(),
            date,
            content: parts[2].trim().to_string(),
        })
    }
}

impl Article {
    /// Creates a new builder for an article
    pub fn builder() -> ArticleBuilder<String> {
        ArticleBuilder::default()
    }

    /// Saves the article to a json file
    pub fn save_json(&self) -> Result<(), std::io::Error> {
        println!("Saving article to {}.json", self.title);
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(format!("{}.json", self.title), json)?;
        Ok(())
    }

    /// Utility function for extracting a single field from frontmatter
    pub fn extract_field<'a>(
        field: &str,
        frontmatter: &'a serde_yaml::Value,
    ) -> Result<&'a str, ArticleError> {
        frontmatter[field]
            .as_str()
            .ok_or(ArticleError::FrontMatterParse)
    }

    /// Utility function for extracting and parsing the DateTime
    pub fn extract_date(frontmatter: &serde_yaml::Value) -> Result<DateTime<Tz>, ArticleError> {
        // Get the date String
        let date = Self::extract_field("date", frontmatter)?;

        // Parse to a Naive Date
        let dt = NaiveDateTime::parse_from_str(date, "%Y-%m-%d").map_err(|e| {
            tracing::error!("Failed to parse date: {e}");
            ArticleError::FrontMatterParse
        })?;

        // Convert to Pacific Time
        let pt = Pacific
            .from_local_datetime(&dt)
            .single()
            .ok_or("Ambiguous or invalid local time")
            .map_err(|e| {
                tracing::error!("Failed to convert local time to UTC: {e}");
                ArticleError::FrontMatterParse
            })?;

        Ok(pt)
    }
}
