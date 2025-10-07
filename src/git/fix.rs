use crate::errors::*;
use crate::git::cmd::GitCommand;

pub fn execute_fix() -> TResult<String> {
    _ = GitCommand::new(["add", "--all"])?.execute()?;
    _ = GitCommand::new(["commit", "--amend", "--no-edit"])?.execute()?;
    _ = GitCommand::new(["push", "-f"])?.execute()?;
    Ok("Fixed the last commit successfully".into())
}
