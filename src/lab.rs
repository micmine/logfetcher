use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::error::Error;

pub async fn get_merge_requests(
    url: &str,
    token: &str,
    repo: &str,
    branch: Option<&str>,
) -> Option<Vec<MergeRequest>> {
    let res = fetch(url, token).await.ok()?;

    if let Ok(data) = serde_json::from_str::<TopData>(&res) {
        let requests: Vec<_> = data
            .data
            .current_user
            .authored_merge_requests
            .nodes
            .into_iter()
            .filter(|m| &m.project.name == repo)
            .filter(|m| &m.state == "opened")
            .filter(|m| {
                if let Some(pipe) = &m.head_pipeline {
                    return should_include_pipeline(pipe);
                }
                return false;
            })
            .filter(|m| {
                if let Some(branch) = &branch {
                    return m.source_branch == *branch;
                }
                return true;
            })
            .collect();
        return Some(requests);
    } else {
        return None;
    };
}

fn should_include_pipeline(pipe: &HeadPipeline) -> bool {
    pipe.jobs.nodes.iter().count() > 0
}

async fn fetch(url: &str, token: &str) -> Result<String, Box<dyn Error>> {
    let graphql: String = format!("{url}/api/graphql");
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let query = "{\"query\": \"query {currentUser { authoredMergeRequests { nodes { projectId project { id name } state sourceBranch headPipeline { active complete commit { sha } jobs(statuses: [FAILED]) { nodes { id active createdAt } } } } } }}\"}";
    let res = client
        .post(graphql)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(query)
        .send()
        .await?
        .text()
        .await?;
    Ok(res)
}
#[derive(Debug, Deserialize)]
pub struct Commit {
    pub sha: String,
}

#[derive(Debug, Deserialize)]
pub struct Job {
    pub id: String,
    pub active: bool,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Deserialize)]
pub struct JobNodes {
    pub nodes: Vec<Job>,
}

#[derive(Debug, Deserialize)]
pub struct HeadPipeline {
    pub active: bool,
    pub commit: Commit,
    pub complete: bool,
    pub jobs: JobNodes,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct MergeRequest {
    #[serde(rename = "headPipeline")]
    pub head_pipeline: Option<HeadPipeline>,
    pub project: Project,
    #[serde(rename = "sourceBranch")]
    pub source_branch: String,
    pub state: String,
    #[serde(rename = "projectId")]
    pub project_id: usize,
}

#[derive(Debug, Deserialize)]
struct MergeRequstNodes {
    pub nodes: Vec<MergeRequest>,
}

#[derive(Debug, Deserialize)]
struct CurrentUser {
    #[serde(rename = "authoredMergeRequests")]
    pub authored_merge_requests: MergeRequstNodes,
}

#[derive(Debug, Deserialize)]
struct Data {
    #[serde(rename = "currentUser")]
    pub current_user: CurrentUser,
}

#[derive(Debug, Deserialize)]
struct TopData {
    pub data: Data,
}

pub async fn get_logs(
    failed_jobs_from_mrs: Vec<MergeRequest>,
    prefix: &str,
    token: &str,
) -> Option<Vec<String>> {
    let links: Vec<String> = failed_jobs_from_mrs
        .into_iter()
        .filter(|m| m.head_pipeline.is_some())
        .map(|m| (m.project_id, m.head_pipeline.unwrap()))
        .map(|(proid, pipeline)| {
            pipeline.jobs.nodes.into_iter().map(move |job| {
                return format!(
                    "{prefix}/api/v4/projects/{}/jobs/{}/trace",
                    &proid,
                    job.id.replace("gid://gitlab/Ci::Build/", "")
                );
            })
        })
        .flatten()
        .collect();
    let mut logs = Vec::with_capacity(links.len());

    for link in links {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .ok()?;
        let res = client
            .get(link)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .ok()?
            .text()
            .await
            .ok()?;
        logs.push(res);
    }
    Some(logs)
}
