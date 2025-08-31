pub mod errors;
pub mod git;
pub mod gitlab;
pub mod global;

use std::process::exit;

use clap::{self, Parser};

fn main() {
    let cli = Cli::parse();
    match global::init_config(global::Config {
        debug: cli.global.debug,
        git_debug: cli.global.git_debug,
        api_debug: cli.global.api_debug,
    }) {
        Err(_) => {
            println!("Failed to initialize config! Should never happen!");
            exit(1);
        }
        _ => {}
    }

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
    #[command(flatten)]
    global: GlobalConfig,
    #[command(subcommand)]
    commands: Command,
}

#[derive(Debug, clap::Args)]
struct GlobalConfig {
    #[arg(long, default_value_t = false)]
    debug: bool,
    #[arg(long, default_value_t = false)]
    git_debug: bool,
    #[arg(long, default_value_t = false)]
    api_debug: bool,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    Main,
    MR(gitlab::MR),
}
