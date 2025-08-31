use crate::errors::{Error, TResult};
use std::collections::HashMap;

#[derive(serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    hosts: HashMap<String, HostConfig>,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct HostConfig {
    pub token: SecretString,
    #[serde(default)]
    pub repos: HashMap<String, RepoConfig>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct RepoConfig {
    pub branch_name_template: BranchNameTemplate,
}

#[derive(Debug)]
pub struct FullRepoConfig {
    pub repo_config: RepoConfig,
    pub token: SecretString,
}

impl Config {
    pub fn get_repo_config(&self, base_url: &str, repo: &str) -> TResult<FullRepoConfig> {
        let host_config = self.hosts.get(base_url).ok_or(Error::Generic(format!(
            "No configuration found for host {}",
            base_url
        )))?;

        let repo_config = match host_config.repos.get(repo) {
            Some(v) => v,
            None => &RepoConfig {
                branch_name_template: BranchNameTemplate("$1".into()),
            },
        };

        Ok(FullRepoConfig {
            repo_config: repo_config.clone(),
            token: host_config.token.clone(),
        })
    }
}

#[derive(Clone, serde::Deserialize)]
pub struct SecretString(String);

impl core::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "****")
    }
}

impl SecretString {
    pub fn to_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct BranchNameTemplate(String);

impl BranchNameTemplate {
    pub fn render(&self, msg: &str) -> String {
        self.0.replace("$1", &self.sanitize_branch_name(msg))
    }

    fn sanitize_branch_name(&self, msg: &str) -> String {
        msg.replace(" ", "-").replace("/", "-").to_lowercase()
    }
}
