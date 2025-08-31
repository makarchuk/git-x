use crate::errors::*;
use crate::git::cmd::GitCommand;

pub fn main() -> TResult<String> {
    //works much faster, but not always present if repo created locally
    match GitCommand::new(["symbolic-ref", "refs/remotes/origin/HEAD"])?.execute() {
        Ok(output) => match output.strip_prefix("refs/remotes/origin/") {
            Some(branch) => return Ok(branch.trim_ascii().to_owned()),
            None => (),
        },
        Err(_) => (),
    };

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
