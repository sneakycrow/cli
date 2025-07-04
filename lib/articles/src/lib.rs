pub mod builder;
pub mod errors;
pub use builder::ArticleBuilder;
use chrono::{DateTime, NaiveDateTime, TimeZone};
use chrono_tz::{Tz, US::Pacific};
use errors::ArticleError;
use markdown_ppp::{
    ast::Document,
    html_printer::{self, render_html},
    parser::{MarkdownParserState, parse_markdown},
};
pub use serde::Serialize;
use std::{fs, path::PathBuf};

#[derive(Serialize)]
struct Frontmatter<'a> {
    title: &'a str,
    author: Option<&'a str>,
    date: &'a str,
}

#[derive(Serialize, Debug)]
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
        };

        let frontmatter_yaml = serde_yaml::to_string(&frontmatter).map_err(|e| {
            tracing::error!("Failed to serialize frontmatter: {e}");
            ArticleError::FrontMatterParse
        })?;

        Ok(format!("---\n{}---\n{}", frontmatter_yaml, value.content))
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

    /// Render the content of the article to HTML
    pub fn render_html(&self) -> Result<String, ArticleError> {
        self.render_ast()
            .map(|doc| render_html(&doc, html_printer::config::Config::default()))
    }

    /// Loads a list of articles from a directory
    pub fn from_dir(dir: PathBuf) -> Result<Vec<Article>, ArticleError> {
        let mut articles = Vec::new();
        let dir_entries = fs::read_dir(dir)?;
        for entry in dir_entries {
            let entry = entry.map_err(ArticleError::IO)?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                let file_content = fs::read_to_string(&path)?;
                let article = Article::try_from(file_content)?;

                articles.push(article);
            }
        }

        Ok(articles)
    }

    /// Renders the content of the article into an AST
    pub fn render_ast(&self) -> Result<Document, ArticleError> {
        let state = MarkdownParserState::default();

        parse_markdown(state, &self.content).map_err(|e| {
            tracing::error!("Failed to parse markdown: {e}");
            ArticleError::ContentParse
        })
    }

    /// Saves the article to a file
    pub fn save(self, output_dir: PathBuf) -> Result<(), ArticleError> {
        // Make sure the output directory is a directory and exists
        if !output_dir.exists() || !output_dir.is_dir() {
            return Err(ArticleError::IO(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Directory specified is invalid: {}", output_dir.display()),
            )));
        }

        // Transform the article date and title into a file name
        // {YYYY-MM-dd}-{title}.md
        let file_name = format!(
            "{}-{}.md",
            self.date.date_naive().to_string(),
            self.serialize_title()
        );

        // Construct the output path and validate it doesn't already exist
        let output_path = output_dir.join(file_name.clone());
        if output_path.exists() {
            return Err(ArticleError::IO(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Output file already exists",
            )));
        }

        // Parse the article into a string for the file
        let content = String::try_from(self)?;

        // Write the file
        std::fs::write(&output_path, content)?;

        tracing::debug!("Article written to {}", output_path.display());
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

    /// Utility function for serializing the title into safe filename
    pub fn serialize_title(&self) -> String {
        self.title.to_lowercase().replace(' ', "-").replace(',', "")
    }
}
