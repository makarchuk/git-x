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
        let client = gitlab::GitlabBuilder::new(base_url, private_token.to_str())
            .cert_insecure()
            .build()
            .with_comment("failed to build gitlab client")?;

        Ok(GitlabProjectClient {
            project: project,
            client: client,
        })
    }
}

impl GitlabProjectClient {
    pub fn create_merge_request(&self, branch: &str, title: &str) -> TResult<MergeRequest> {
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
        Ok(mr.mr)
    }

    pub fn get_merge_requestse_by_branch(&self, branch: &str) -> TResult<Vec<MergeRequest>> {
        let mrs: Vec<ListMergeRequestsResponseItem> = gitlab::api::paged(
            gitlab::api::projects::merge_requests::MergeRequests::builder()
                .project(&self.project)
                .source_branch(branch)
                .build()
                .with_comment("failed to build get merge requests API call")?,
            gitlab::api::Pagination::Limit(200),
        )
        .query(&self.client)
        .with_comment("failed to get merge requests")?;
        //todo: paginate
        Ok(mrs.into_iter().map(|mr| mr.mr).collect())
    }

    pub fn get_merge_request(&self, mr: u64) -> TResult<MergeRequest> {
        let mr: MergeRequest = gitlab::api::projects::merge_requests::MergeRequest::builder()
            .project(&self.project)
            .merge_request(mr)
            .build()
            .with_comment("failed to build get merge request API call")?
            .query(&self.client)
            .with_comment("failed to get merge request")?;
        Ok(mr)
    }
}

//Very much incomplete structure. Consult the docs if you need additional fields available
// https://docs.gitlab.com/api/merge_requests/#create-mr
#[derive(serde::Deserialize, Debug)]
struct CreateMergeRequestResponse {
    #[serde(flatten)]
    mr: MergeRequest,
}

//Very much incomplete structure. Consult the docs if you need additional fields available
// https://docs.gitlab.com/api/merge_requests/#list-project-merge-requests
#[derive(serde::Deserialize, Debug)]
struct ListMergeRequestsResponseItem {
    #[serde(flatten)]
    mr: MergeRequest,
}

#[derive(serde::Deserialize, Debug)]
pub struct MergeRequest {
    pub id: u64,
    pub iid: u64,
    pub title: String,
    pub description: Option<String>,
    pub source_branch: String,
    pub state: String,
    pub web_url: String,
}
