use git2::Repository;
use serde::{Deserialize, Serialize};

pub mod lab;

const DEFAULT_TOKEN: &str = "Your token here";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub gitlab: GitlabConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gitlab: GitlabConfig {
                url: "https://gitlab.com".to_owned(),
                token: DEFAULT_TOKEN.to_owned(),
            },
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitlabConfig {
    pub url: String,
    pub token: String,
}

#[tokio::main]
async fn main() {
    let branch = get_current_branch_name();
    if branch.is_none() {
        eprintln!("logfetcher can only be run from a git repository. Exiting.");
        return;
    }

    let storage_location = &quickcfg::get_location("logfetcher")
        .await
        .expect("Unable to get storage dir");
    let config: Config = quickcfg::load(storage_location).await;
    if config.gitlab.token == DEFAULT_TOKEN {
        eprintln!("The config file is stored at: {storage_location}");
    }

    let current_folder = std::env::current_dir().expect("Unabel to get current folder");
    let current_folder = current_folder
        .file_name()
        .expect("Unable to get folder name");

    let Some(failed_jobs_from_mrs) = lab::get_merge_requests(
        &config.gitlab.url,
        &config.gitlab.token,
        &current_folder.to_string_lossy(),
        branch.as_deref(),
    )
    .await
    else {
        return;
    };
    let Some(logs) = lab::get_logs(
        failed_jobs_from_mrs,
        &config.gitlab.url,
        &config.gitlab.token,
    )
    .await
    else {
        eprintln!("Unable to load logs");
        return;
    };
    let Some(log) = logs.first() else {
        eprintln!("Unable to get log");
        return;
    };
    println!("{}", log);
}

fn get_current_branch_name() -> Option<String> {
    let repo = Repository::open(".").ok()?;
    let head = repo.head().ok()?;
    let head = repo.head().ok()?;

    head.shorthand().map(|n| n.to_string())
}
