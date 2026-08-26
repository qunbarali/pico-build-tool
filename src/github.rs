use anyhow::{Result, anyhow};
use git2::Repository;
use log::info;
use std::path::Path;

pub async fn clone_repository(
    repo_url: &str,
    token: Option<&str>,
    branch: Option<&str>,
    output_dir: &str,
) -> Result<()> {
    info!("Cloning repository: {}", repo_url);
    
    let mut callbacks = git2::RemoteCallbacks::new();
    
    if let Some(token) = token {
        info!("Using GitHub personal access token for authentication");
        let token_clone = token.to_string();
        callbacks.credentials(move |_url, username_from_url, _allowed_types| {
            git2::Cred::userpass_plaintext(
                username_from_url.unwrap_or("git"),
                &token_clone,
            )
        });
    } else {
        info!("Attempting to clone without authentication");
    }
    
    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(callbacks);
    
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);
    
    if let Some(branch_name) = branch {
        builder.branch(branch_name);
    }
    
    match builder.clone(repo_url, Path::new(output_dir)) {
        Ok(_) => {
            info!("Repository cloned successfully to: {}", output_dir);
            Ok(())
        }
        Err(e) => Err(anyhow!("Failed to clone: {}", e))
    }
}

pub async fn verify_repository(repo_url: &str, token: Option<&str>) -> Result<bool> {
    let repo_info = parse_github_url(repo_url)?;
    let api_url = format!("https://api.github.com/repos/{}/{}", repo_info.owner, repo_info.repo);
    
    let client = reqwest::Client::new();
    let mut request = client.get(&api_url);
    
    if let Some(token) = token {
        request = request.header("Authorization", format!("token {}", token));
    }
    
    request = request.header("User-Agent", "pico-build-tool");
    
    match request.send().await {
        Ok(response) => Ok(response.status().as_u16() == 200),
        Err(e) => Err(anyhow!("Failed to verify: {}", e))
    }
}

#[derive(Debug)]
struct RepoInfo {
    owner: String,
    repo: String,
}

fn parse_github_url(url: &str) -> Result<RepoInfo> {
    let url = if url.ends_with(".git") {
        &url[..url.len() - 4]
    } else {
        url
    };
    
    let parts: Vec<&str> = if url.contains("github.com/") {
        url.split("github.com/").nth(1).unwrap_or("").split('/').collect()
    } else {
        return Err(anyhow!("Invalid GitHub URL"));
    };
    
    if parts.len() < 2 {
        return Err(anyhow!("Could not parse repo URL"));
    }
    
    Ok(RepoInfo {
        owner: parts[0].to_string(),
        repo: parts[1].to_string(),
    })
}
