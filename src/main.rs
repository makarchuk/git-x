pub mod git;

use clap::{self, Parser};

fn main() {
    let cli = Cli::parse();

    match &cli.commands {
        Command::Main => {
            let output = git::main();
            println!("{}", output);
        }
    }
}

#[derive(Debug, clap::Parser)] // requires `derive` feature
#[command(name = "git-x")]
#[command(about = "git extensions toolbox", long_about = None)]
struct Cli {
    #[command(subcommand)]
    commands: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    Main,
}
