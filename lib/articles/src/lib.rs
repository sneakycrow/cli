pub mod builder;

pub use builder::ArticleBuilder;
pub use serde::Serialize;

#[derive(Serialize)]
pub struct Article {
    title: String,
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
}
