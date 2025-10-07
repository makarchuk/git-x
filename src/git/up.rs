use crate::errors::*;
use crate::git::cmd::GitCommand;

pub fn execute_up() -> TResult<String> {
    _ = GitCommand::new(["pull", "origin", "trunk", "--no-edit", "--no-ff"])?.execute()?;
    Ok("Updated to new trunk succesfully".into())
}
