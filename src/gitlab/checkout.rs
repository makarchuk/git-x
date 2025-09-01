use crate::{errors::*, git::cmd::GitCommand};

#[derive(Debug, Clone, clap::Args)]
pub struct CheckoutArgs {
    pub mr: u64,
}

pub fn execute_checkout(
    git_context: &crate::gitlab::GitContext,
    checkout_args: &CheckoutArgs,
) -> TResult<String> {
    let mr = git_context
        .gitlab_client
        .get_merge_request(checkout_args.mr)?;

    GitCommand::new([
        "fetch",
        "origin",
        &format!("{}:{}", &mr.source_branch, &mr.source_branch),
    ])?
    .execute()?;

    GitCommand::new(["checkout", &mr.source_branch])?.execute()?;

    Ok(format!(
        "Checked out to branch `{}` for MR !{} `{}`\nView Merge Request in Browser: {}",
        mr.source_branch, mr.iid, mr.title, mr.web_url
    ))
}
