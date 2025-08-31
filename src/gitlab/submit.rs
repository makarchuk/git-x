use crate::errors::TResult;
use crate::git::cmd::GitCommand;

#[derive(Debug, clap::Args)]
pub struct SubmitArgs {
    #[clap(short, long, required = true)]
    message: String,
}

pub fn execute_submit(
    git_context: crate::gitlab::GitContext,
    submit_args: &SubmitArgs,
) -> TResult<String> {
    _ = GitCommand::new(["add", "--all"])?.execute()?;
    let new_branch_name = git_context
        .config
        .repo_config
        .branch_name_template
        .render(&submit_args.message);
    _ = GitCommand::new(["checkout", "-b", &new_branch_name])?.execute()?;
    _ = GitCommand::new(["commit", "-m", &submit_args.message])?.execute()?;
    _ = GitCommand::new(["push", "--set-upstream", "origin", &new_branch_name])?.execute()?;
    let mr_url = git_context
        .gitlab_client
        .create_merge_request(&new_branch_name, &submit_args.message)?;
    Ok(format!("Merge Request Succesfully created!\n{}", mr_url))
}
