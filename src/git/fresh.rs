use crate::errors::*;
use crate::git::cmd::GitCommand;
use crate::git::main;

pub fn execute_fresh() -> TResult<String> {
    let main_branch = main::execute_main()?;
    _ = GitCommand::new(["checkout", &main_branch])?.execute()?;
    _ = GitCommand::new(["pull", "origin", &main_branch])?.execute()?;
    Ok(format!(
        "Checked out and updated to latest `{}`",
        main_branch
    ))
}
