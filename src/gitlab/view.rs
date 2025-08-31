use crate::errors::TResult;
use crate::git::cmd::GitCommand;

pub fn execute_view(ctx: &crate::gitlab::GitContext) -> TResult<String> {
    let current_branch = GitCommand::new(["branch", "--show-current"])?
        .execute()?
        .trim()
        .to_owned();
    crate::log_debug!("Current branch: {}", current_branch);

    let mrs = ctx
        .gitlab_client
        .get_merge_requestse_by_branch(&current_branch)?;

    match mrs.len() {
        0 => Ok(format!("No mrs found for branch: {}", current_branch).to_string()),
        1 => match open::that(&mrs[0].web_url) {
            Ok(()) => Ok(format!(
                "Opening Merge Request !{} `{}`: {}",
                mrs[0].id, mrs[0].title, mrs[0].web_url
            )),
            Err(e) => Ok(format!(
                "Merge request found !{} `{}`: {}\nFailed to open in browser: {}",
                mrs[0].id, mrs[0].title, mrs[0].web_url, e
            )),
        },
        n => {
            let mut result = format!("Found {} mrs for branch {}:\n", n, current_branch);
            for mr in mrs {
                result.push_str(&format!(" - !{} `{}`: {}\n", mr.id, mr.title, mr.web_url));
            }
            Ok(result)
        }
    }
}
