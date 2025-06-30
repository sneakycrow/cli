use articles::Article;
use clap::{Command, arg};

/// Create the command to interact with articles
pub(crate) fn cli() -> Command {
    Command::new("article")
        .about("Interact with articles")
        .subcommand(
            Command::new("create")
                .about("Creates a new article file with the given data")
                .arg(arg!(title: -t --title <TITLE>)),
        )
}

/// Runs the article with given arg matches
pub(crate) fn run(matches: &clap::ArgMatches) {
    match matches.subcommand() {
        Some(("create", sub_matches)) => {
            let title = sub_matches
                .get_one::<String>("title")
                .map(|s| s.to_string());

            let _article = Article::builder()
                .maybe_title(title)
                .build()
                .save_json()
                .expect("Could not save article");
        }
        _ => unreachable!(),
    }
}
