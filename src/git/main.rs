use crate::errors::*;
use crate::git::cmd::GitCommand;
use std::process;

pub fn main() -> Result<String> {
    let stdout = GitCommand::new(["remote", "show", "origin"])?.execute()?;

    Ok(stdout
        .lines()
        .map(|line| line.trim_ascii())
        .filter(|line| line.starts_with("HEAD branch"))
        .map(|line| line.strip_prefix("HEAD branch: "))
        .next()
        .unwrap() // .expect("failed to get default branch")
        .unwrap() // .expect("failed to get default branch")
        .to_owned())
}
