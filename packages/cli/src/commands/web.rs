use clap::Command;

/// Create the command to interact with articles
pub(crate) fn cli() -> Command {
    Command::new("web")
        .about("fn for personal website")
        .subcommand(Command::new("serve").about("Serves the website locally"))
        .subcommand(Command::new("build").about("Builds the website"))
}

/// Runs the serve subcommand
pub(crate) async fn run(matches: &clap::ArgMatches) {
    match matches.subcommand() {
        Some(("serve", _)) => {
            if let Err(e) = web::serve().await {
                tracing::error!("Error running serve command: {e}");
            }
        }
        Some(("build", _)) => {
            if let Err(e) = web::build(&web::AppState::default()) {
                tracing::error!("Error running build command: {e}");
            }
        }
        _ => unreachable!(),
    }
}
