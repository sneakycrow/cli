pub(crate) mod about;
pub(crate) mod article;
pub(crate) mod web;

use clap::Command;
use context::{DEFAULT_CONFIG_FILE, SneakyContext};

pub(crate) struct Cli {
    subcommands: Vec<Command>,
    context: SneakyContext,
}

impl Cli {
    /// Parses and runs the CLI with default parameters
    pub(crate) async fn parse(&self) {
        let matches = self.build().get_matches();
        match matches.subcommand() {
            Some(("article", sub_matches)) => article::run(sub_matches),
            Some(("web", sub_matches)) => web::run(sub_matches).await,
            Some(("about", sub_matches)) => about::run(sub_matches, &self.context).await,
            _ => unreachable!(),
        }
    }

    /// Builds the root cli
    pub(crate) fn build(&self) -> Command {
        // Build root command
        let mut root_cmd = Command::new("sc")
            .name("sc")
            .about("sneakycrow's personal CLI tool")
            .subcommand_required(true)
            .arg_required_else_help(true);

        // Add subcommand for each defined
        for subcommand in &self.subcommands {
            root_cmd = root_cmd.subcommand(subcommand);
        }

        root_cmd
    }
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            subcommands: vec![article::cli(), web::cli(), about::cli()],
            context: SneakyContext::from_file(DEFAULT_CONFIG_FILE).unwrap_or_default(),
        }
    }
}
