pub(crate) mod commands;

pub(crate) use commands::Cli;

fn main() {
    Cli::parse();
}
