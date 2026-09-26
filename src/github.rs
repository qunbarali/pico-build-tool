use anyhow::{anyhow, Context, Result};
use git2::{build::RepoBuilder, Cred, FetchOptions, RemoteCallbacks};
use log::info;
use reqwest::Client;
use std::path::Path;
use tokio::task;

pub async fn clone_repository(
    repo_url: &str,
    token: Option<&str>,
    branch: Option<&str>,
    output_dir: &str,
) -> Result<()> {
    let (owner, repo) = parse_github_url(repo_url)?;
    let output = Path::new(output_dir);
    anyhow::ensure!(!output.exists(), "output directory already exists: {}", output.display());

    let token = token.map(str::to_owned).or_else(|| std::env::var("GITHUB_TOKEN").ok());
    let url = format!("https://github.com/{owner}/{repo}.git");
    let output = output.to_path_buf();
    let branch = branch.map(str::to_owned);

    task::spawn_blocking(move || -> Result<()> {
        let mut callbacks = RemoteCallbacks::new();
        if let Some(secret) = token {
            callbacks.credentials(move |_url, username, _allowed| {
                Cred::userpass_plaintext(username.unwrap_or("git"), &secret)
            });
        }

        let mut fetch = FetchOptions::new();
        fetch.remote_callbacks(callbacks);

        let mut builder = RepoBuilder::new();
        builder.fetch_options(fetch);
        if let Some(branch) = branch.as_deref() {
            builder.branch(branch);
        }

        builder.clone(&url, &output)
            .with_context(|| format!("failed to clone {url}"))?;
        Ok(())
    }).await??;

    info!("Cloned {} to {}", repo_url, output_dir);
    Ok(())
}

pub async fn verify_repository(repo_url: &str, token: Option<&str>) -> Result<bool> {
    let (owner, repo) = parse_github_url(repo_url)?;
    let client = Client::builder().user_agent("pico-build-tool/1.1").build()?;
    let mut request = client.get(format!("https://api.github.com/repos/{owner}/{repo}"));
    let env_token = std::env::var("GITHUB_TOKEN").ok();
    if let Some(secret) = token.or(env_token.as_deref()) {
        request = request.bearer_auth(secret);
    }
    Ok(request.send().await?.status().is_success())
}

fn parse_github_url(input: &str) -> Result<(String, String)> {
    let raw = input.trim();
    let without_suffix = raw.strip_suffix(".git").unwrap_or(raw);
    let parsed = url::Url::parse(without_suffix)
        .map_err(|_| anyhow!("invalid GitHub URL"))?;
    anyhow::ensure!(parsed.scheme() == "https", "GitHub URL must use https");
    anyhow::ensure!(parsed.host_str() == Some("github.com"), "only github.com URLs are supported");

    let mut segments = parsed.path_segments()
        .ok_or_else(|| anyhow!("GitHub URL has no path"))?
        .filter(|s| !s.is_empty());
    let owner = segments.next().ok_or_else(|| anyhow!("missing GitHub owner"))?;
    let repo = segments.next().ok_or_else(|| anyhow!("missing GitHub repository"))?;
    anyhow::ensure!(segments.next().is_none(), "GitHub URL contains unexpected path segments");
    anyhow::ensure!(parsed.query().is_none() && parsed.fragment().is_none(), "query strings and fragments are not allowed");

    Ok((owner.to_string(), repo.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_real_github_https_urls() {
        assert_eq!(parse_github_url("https://github.com/a/b.git").unwrap(), ("a".into(), "b".into()));
        assert!(parse_github_url("http://github.com/a/b").is_err());
        assert!(parse_github_url("https://evil.example/github.com/a/b").is_err());
        assert!(parse_github_url("https://github.com/a/b/issues").is_err());
    }
}
