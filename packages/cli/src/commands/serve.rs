use clap::Command;

/// Create the command to interact with articles
pub(crate) fn cli() -> Command {
    Command::new("serve").about("Serve the website")
}

/// Runs the serve subcommand
pub(crate) async fn run() {
    web::serve().await
}
