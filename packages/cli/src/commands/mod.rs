pub(crate) mod article;

use clap::Command;

pub(crate) struct Cli {
    subcommands: Vec<Command>,
}

impl Cli {
    /// Parses and runs the CLI with default parameters
    pub(crate) fn parse() {
        let cli = Self::default();
        let matches = cli.build().get_matches();
        match matches.subcommand() {
            Some(("article", sub_matches)) => article::run(sub_matches),
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
            subcommands: vec![article::cli()],
        }
    }
}
