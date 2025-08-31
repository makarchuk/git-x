pub mod errors;
pub mod git;

use std::process::exit;

use clap::{self, Parser};

fn main() {
    let cli = Cli::parse();

    match &cli.commands {
        Command::Main => match git::main() {
            Ok(output) => println!("{}", output),
            Err(err) => {
                err.print();
                exit(1);
            }
        },
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
