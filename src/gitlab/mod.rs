mod client;
mod submit;
pub use client::*;
pub use submit::*;

use crate::errors::*;
use crate::git::GitCommand;
use gix_url;

#[derive(Debug, clap::Args)]
pub struct MR {
    #[clap(subcommand)]
    command: Subcommand,
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    Submit(Submit),
}

#[derive(Debug, clap::Args)]
struct Submit {
    #[clap(short, long)]
    message: String,
}

pub fn mr(mr: &MR) -> TResult<String> {
    match &mr.command {
        Subcommand::Submit(_) => {
            dbg!(mr);
            let remote_url = get_remote_url()?;
            println!("Remote URL: {}", remote_url);
            let client = build_client(&remote_url)?;
            Ok("".into())
        }
    }
}

fn get_remote_url() -> TResult<String> {
    let remote_addr = GitCommand::new(["remote", "get-url", "origin"])?.execute()?;
    Ok(remote_addr)
}

fn build_client(remote_url: &str) -> TResult<GitlabClient> {
    let url = gix_url::Url::from_bytes(remote_url.as_bytes().into()).to_generic()?;
    dbg!(url);
    unimplemented!()
}
