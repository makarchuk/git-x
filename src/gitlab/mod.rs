mod client;
mod config;
mod submit;

use crate::errors::*;
use crate::git::cmd::GitCommand;

use gix_url;

#[derive(Debug)]
struct GitContext {
    pub config: config::FullRepoConfig,
    pub gitlab_client: client::GitlabProjectClient,
}

#[derive(Debug, clap::Args)]
pub struct MR {
    #[clap(subcommand)]
    command: Subcommand,
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    Submit(submit::SubmitArgs),
}

pub fn mr(mr: &MR) -> TResult<String> {
    let remote_url = get_remote_url()?;
    let git_context = get_execution_context(&remote_url)?;
    dbg!(&git_context);

    match &mr.command {
        Subcommand::Submit(submit_args) => submit::execute_submit(git_context, submit_args),
    }
}

fn get_remote_url() -> TResult<String> {
    let remote_addr = GitCommand::new(["remote", "get-url", "origin"])?.execute()?;
    Ok(remote_addr)
}

fn get_execution_context(remote_url: &str) -> TResult<GitContext> {
    let url = gix_url::Url::from_bytes(remote_url.as_bytes().into()).to_generic()?;
    let base_url = url
        .host()
        .ok_or(Error::Generic(format!(
            "Failed to get host from URL {}",
            url
        )))?
        .to_string();

    let project = url
        .path
        .clone()
        .to_string()
        .trim_ascii()
        .trim_end_matches(".git")
        .to_owned();

    let home_dir =
        std::env::home_dir().ok_or(Error::Generic("Failed to get home directory".into()))?;
    let config_path = home_dir.join(".config/gitx/credentials.json");
    let config_file = std::fs::File::open(&config_path)
        .with_comment(&format!("Failed to open {:?}", config_path))?;

    let config: config::Config =
        serde_json::from_reader(config_file).with_comment("failed to parse credentials")?;

    let repo_config = config.get_repo_config(&base_url, &project)?;

    let gitlab_client =
        client::GitlabProjectClient::new(base_url, project, repo_config.token.clone())?;

    Ok(GitContext {
        config: repo_config,
        gitlab_client,
    })
}
