use gitlab::api::Query;

use crate::{
    errors::{TResult, ToGeneric},
    gitlab::config,
};

#[derive(Debug)]
pub struct GitlabProjectClient {
    // pub base_url: String,
    pub project: String,
    // pub private_token: config::SecretString,
    client: gitlab::Gitlab,
}

impl GitlabProjectClient {
    pub fn new(
        base_url: String,
        project: String,
        private_token: config::SecretString,
    ) -> TResult<Self> {
        let client = gitlab::Gitlab::new(base_url, private_token.to_str())
            .with_comment("failed to initialize gitlab client")?;
        Ok(GitlabProjectClient {
            project: project,
            client: client,
        })
    }
}

impl GitlabProjectClient {
    pub fn create_merge_request(&self, branch: &str, title: &str) -> TResult<String> {
        let mr: CreateMergeRequestResponse =
            gitlab::api::projects::merge_requests::CreateMergeRequest::builder()
                .project(&self.project)
                .source_branch(branch)
                //TODO: pass target branch here
                .target_branch("trunk")
                .title(title)
                .build()
                .with_comment("failed to build create merge request API call")?
                .query(&self.client)
                .with_comment("failed to create merge request")?;
        Ok(mr.web_url)
    }
}

//Very much incomplete structure. Consult the docs if you need additional fields available
// https://docs.gitlab.com/api/merge_requests/#create-mr
#[derive(serde::Deserialize, Debug)]
struct CreateMergeRequestResponse {
    web_url: String,
}
