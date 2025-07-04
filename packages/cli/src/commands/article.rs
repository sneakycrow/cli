use articles::Article;
use clap::{Command, arg};
use std::path::PathBuf;

const DEFAULT_ARTICLE_OUTPUT_DIR: &str = "_posts/";

/// Create the command to interact with articles
pub(crate) fn cli() -> Command {
    Command::new("article")
        .about("Interact with articles")
        .subcommand(
            Command::new("create")
                .about("Creates a new article file with the given data")
                .arg(arg!(output: -o --output <OUTPUT> "optional output of the file"))
                .arg(arg!(-t --title <TITLE> "title of the article")),
        )
}

/// Runs the article with given arg matches
pub(crate) fn run(matches: &clap::ArgMatches) {
    match matches.subcommand() {
        Some(("create", sub_matches)) => {
            // Get the title
            let title = sub_matches.get_one::<String>("title").cloned();

            // Get the output, or fallback to the default
            let output = sub_matches
                .get_one::<String>("output")
                .map(|o| PathBuf::from(o))
                .unwrap_or(PathBuf::from(DEFAULT_ARTICLE_OUTPUT_DIR));

            // Make sure the output directory exists
            std::fs::create_dir_all(output.clone()).expect("Could not create output directory");

            // Build the article and save it
            Article::builder()
                .maybe_title(title)
                .build()
                .save(&output)
                .expect("Could not save article");
        }
        _ => unreachable!(),
    }
}
