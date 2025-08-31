pub mod errors;
pub mod git;
pub mod gitlab;

use std::process::exit;

use clap::{self, Parser};

fn main() {
    let cli = Cli::parse();

    match &cli.commands {
        Command::Main => match git::main::main() {
            Ok(output) => println!("{}", output),
            Err(err) => {
                println!("Failed: {}", err.print());
                exit(1);
            }
        },
        Command::MR(mr) => match gitlab::mr(mr) {
            Ok(output) => println!("Ok: {}", output),
            Err(err) => {
                println!("Failed: {}", err.print());
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
    MR(gitlab::MR),
}
