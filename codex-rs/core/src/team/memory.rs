use super::config::TeamMemoryProviderConfig;
use super::config::TeamMemoryProviderMode;
use super::config::TeamTapeProviderConfig;
use super::state::TeamTapeEntry;
use crate::default_client::build_reqwest_client;
use serde::Serialize;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TeamMemoryProviderHealth {
    Ready,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub(crate) struct TeamMemoryProviderStatus {
    pub mode: TeamMemoryProviderMode,
    pub health: TeamMemoryProviderHealth,
}

impl TeamMemoryProviderStatus {
    pub(crate) fn local_ready() -> Self {
        Self {
            mode: TeamMemoryProviderMode::Local,
            health: TeamMemoryProviderHealth::Ready,
        }
    }

    pub(crate) fn for_config(config: &TeamMemoryProviderConfig) -> Self {
        Self {
            mode: config.mode,
            health: TeamMemoryProviderHealth::Ready,
        }
    }
}

#[derive(Debug, Serialize)]
struct TapeMemoryEnvelope<'a> {
    workspace_root: &'a Path,
    entry: &'a TeamTapeEntry,
}

pub(crate) async fn mirror_entry_to_provider(
    workspace_root: &Path,
    provider: &TeamMemoryProviderConfig,
    entry: &TeamTapeEntry,
) -> io::Result<TeamMemoryProviderStatus> {
    match provider.mode {
        TeamMemoryProviderMode::Local => Ok(TeamMemoryProviderStatus::local_ready()),
        TeamMemoryProviderMode::Tape => {
            let Some(config) = provider.tape.as_ref() else {
                return Ok(TeamMemoryProviderStatus {
                    mode: TeamMemoryProviderMode::Tape,
                    health: TeamMemoryProviderHealth::Degraded,
                });
            };
            let health = post_entry_to_tape(workspace_root, config, entry).await;
            Ok(TeamMemoryProviderStatus {
                mode: TeamMemoryProviderMode::Tape,
                health,
            })
        }
    }
}

async fn post_entry_to_tape(
    workspace_root: &Path,
    config: &TeamTapeProviderConfig,
    entry: &TeamTapeEntry,
) -> TeamMemoryProviderHealth {
    let client = build_reqwest_client();
    let mut request = client
        .post(config.endpoint.as_str())
        .json(&TapeMemoryEnvelope {
            workspace_root,
            entry,
        });
    if let Some(env_name) = config.api_key_env.as_deref()
        && let Ok(value) = std::env::var(env_name)
        && !value.trim().is_empty()
    {
        request = request.bearer_auth(value);
    }
    if let Some(project) = config.project.as_deref()
        && !project.trim().is_empty()
    {
        request = request.header("x-codex-team-project", project);
    }
    match request.send().await {
        Ok(response) if response.status().is_success() => TeamMemoryProviderHealth::Ready,
        Ok(_) | Err(_) => TeamMemoryProviderHealth::Degraded,
    }
}
