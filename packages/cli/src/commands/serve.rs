use clap::Command;

/// Create the command to interact with articles
pub(crate) fn cli() -> Command {
    Command::new("serve").about("Serve the website")
}

/// Runs the serve subcommand
pub(crate) async fn run() {
    if let Err(e) = web::serve().await {
        tracing::error!("Error running serve command: {e}");
    }
}
