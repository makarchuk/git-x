use crate::errors::TResult;
use crate::git::cmd::GitCommand;

pub fn execute_view(ctx: &crate::gitlab::GitContext) -> TResult<String> {
    let current_branch = GitCommand::new(["branch", "--show-current"])?
        .execute()?
        .trim()
        .to_owned();
    crate::log_debug!("Current branch: {}", current_branch);

    let links = ctx
        .gitlab_client
        .get_merge_requestse_by_branch(&current_branch)?;

    match links.len() {
        0 => Ok(format!("No mrs found for branch: {}", current_branch).to_string()),
        1 => match open::that(&links[0]) {
            Ok(()) => Ok(format!(
                "Opening merge request for branch {}: {}",
                current_branch, links[0]
            )),
            Err(e) => Ok(format!(
                "Merge request found for branch {}: {}\nFailed to open in browser: {}",
                current_branch, links[0], e
            )),
        },
        n => {
            let mut result = format!("Found {} mrs for branch {}:\n", n, current_branch);
            for link in links {
                result.push_str(&format!(" - {}\n", link));
            }
            Ok(result)
        }
    }
}
