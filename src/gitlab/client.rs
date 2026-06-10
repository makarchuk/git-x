use gitlab::api::Query;
use gitlab::api::projects::merge_requests as mr_api;
use gitlab::api::users as user_api;

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

#[derive(Debug, Default)]
pub struct MrCreateOptions {
    pub source_branch: String,
    pub target_branch: String,
    pub title: String,
    pub delete_source_branch: bool,
}

impl GitlabProjectClient {
    pub fn create_merge_request(&self, options: MrCreateOptions) -> TResult<MergeRequest> {
        let mr: CreateMergeRequestResponse = mr_api::CreateMergeRequest::builder()
            .project(&self.project)
            .source_branch(&options.source_branch)
            .target_branch(&options.target_branch)
            .title(&options.title)
            .remove_source_branch(options.delete_source_branch)
            .build()
            .with_comment("failed to build create merge request API call")?
            .query(&self.client)
            .with_comment("failed to create merge request")?;
        Ok(mr.mr)
    }

    pub fn get_merge_requestse_by_branch(&self, branch: &str) -> TResult<Vec<MergeRequest>> {
        let mrs: Vec<ListMergeRequestsResponseItem> = gitlab::api::paged(
            mr_api::MergeRequests::builder()
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
        let mr: MergeRequest = mr_api::MergeRequest::builder()
            .project(&self.project)
            .merge_request(mr)
            .build()
            .with_comment("failed to build get merge request API call")?
            .query(&self.client)
            .with_comment("failed to get merge request")?;
        Ok(mr)
    }

    pub fn get_merge_requests(&self, filter: GetMergeRequestsFilter) -> TResult<Vec<MergeRequest>> {
        let mut builder = mr_api::MergeRequests::builder();
        let mut request = builder.project(&self.project);

        if let Some(author) = filter.author {
            match author {
                MergeRequestFilterAuthor::Username(username) => {
                    request = request.author(username);
                }
                MergeRequestFilterAuthor::Me => {
                    request = request.scope(mr_api::MergeRequestScope::CreatedByMe);
                }
            }
        }
        if filter.opened_only {
            request = request.state(mr_api::MergeRequestState::Opened);
        }

        let mrs: Vec<ListMergeRequestsResponseItem> = gitlab::api::paged(
            request
                .build()
                .with_comment("failed to build get merge requests API call")?,
            gitlab::api::Pagination::Limit(200),
        )
        .query(&self.client)
        .with_comment("failed to get merge requests")?;
        Ok(mrs.into_iter().map(|mr| mr.mr).collect())
    }

    pub fn resolve_author_username(&self, query: &str) -> TResult<Option<String>> {
        let username_matches: Vec<UserSearchResponseItem> = gitlab::api::paged(
            user_api::Users::builder()
                .username(query)
                .build()
                .with_comment("failed to build get users API call")?,
            gitlab::api::Pagination::Limit(2),
        )
        .query(&self.client)
        .with_comment("failed to get user by username")?;

        if let Some(user) = username_matches
            .iter()
            .find(|user| user.username.eq_ignore_ascii_case(query))
        {
            return Ok(Some(user.username.clone()));
        }

        let search_matches: Vec<UserSearchResponseItem> = gitlab::api::paged(
            user_api::Users::builder()
                .search(query)
                .build()
                .with_comment("failed to build get users API call")?,
            gitlab::api::Pagination::Limit(20),
        )
        .query(&self.client)
        .with_comment("failed to search users")?;

        let exact_match = search_matches.iter().find(|user| user.matches_query(query));
        let user = exact_match.or_else(|| {
            if search_matches.len() == 1 {
                search_matches.first()
            } else {
                None
            }
        });
        Ok(user.map(|user| user.username.clone()))
    }
}

pub struct GetMergeRequestsFilter {
    pub author: Option<MergeRequestFilterAuthor>,
    pub opened_only: bool,
}

pub enum MergeRequestFilterAuthor {
    #[allow(dead_code)]
    Username(String),
    Me,
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
struct UserSearchResponseItem {
    username: String,
    email: Option<String>,
    public_email: Option<String>,
    commit_email: Option<String>,
}

impl UserSearchResponseItem {
    fn matches_query(&self, query: &str) -> bool {
        self.username.eq_ignore_ascii_case(query) || self.matches_email(query)
    }

    fn matches_email(&self, email: &str) -> bool {
        [
            self.email.as_ref(),
            self.public_email.as_ref(),
            self.commit_email.as_ref(),
        ]
        .into_iter()
        .flatten()
        .any(|value| value.eq_ignore_ascii_case(email))
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct MergeRequest {
    pub id: u64,
    pub iid: u64,
    pub title: String,
    #[allow(dead_code)]
    pub description: Option<String>,
    pub source_branch: String,
    pub state: String,
    pub web_url: String,
}
