use clap::Command;
use context::SneakyContext;

/// Create the command to interact with articles
pub(crate) fn cli() -> Command {
    Command::new("about").about("about the author of sneaky crow")
}

/// Runs the serve subcommand
pub(crate) async fn run(_matches: &clap::ArgMatches, ctx: &SneakyContext) {
    println!("A cli tool for interacting with sneaky crow data");
    println!("by {} <{}>", ctx.me.name, ctx.me.email)
}
