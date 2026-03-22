use super::config::TeamMemoryProviderConfig;
use super::config::TeamMemoryProviderMode;
use super::config::TeamTapeProviderConfig;
use super::redaction::public_team_ref;
use super::redaction::sanitize_summary_for_export;
use super::state::TeamKind;
use super::state::TeamTapeEntry;
use super::state::TeamTapeKind;
use super::state::load_team_state_bundle;
use crate::default_client::build_reqwest_client;
use serde::Serialize;
use std::io;
use std::path::Path;
use std::path::PathBuf;

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
struct TapeMemoryEnvelope {
    scope: &'static str,
    entry: SanitizedTapeMemoryEntry,
}

#[derive(Debug, Serialize)]
struct SanitizedTapeMemoryEntry {
    team_ref: String,
    kind: TeamTapeKind,
    summary: String,
    counterpart_team_ref: Option<String>,
    phase: Option<String>,
    anchor: Option<String>,
    artifact_refs: Vec<PathBuf>,
    peer_intent: Option<String>,
    created_at: String,
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
    let sanitized = match sanitize_tape_entry(workspace_root, entry).await {
        Ok(entry) => entry,
        Err(_) => return TeamMemoryProviderHealth::Degraded,
    };
    let client = build_reqwest_client();
    let mut request = client
        .post(config.endpoint.as_str())
        .json(&TapeMemoryEnvelope {
            scope: "codex-team-workflow",
            entry: sanitized,
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

async fn sanitize_tape_entry(
    workspace_root: &Path,
    entry: &TeamTapeEntry,
) -> io::Result<SanitizedTapeMemoryEntry> {
    let team_ref = sanitize_team_ref(workspace_root, &entry.team_id).await;
    let counterpart_team_ref = match entry.counterpart_team_id.as_deref() {
        Some(team_id) => Some(sanitize_team_ref(workspace_root, team_id).await),
        None => None,
    };
    let peer_intent = entry.peer_message.as_ref().map(|peer| match peer.intent {
        super::state::TeamA2aIntent::Align => "align".to_string(),
        super::state::TeamA2aIntent::Request => "request".to_string(),
        super::state::TeamA2aIntent::Answer => "answer".to_string(),
        super::state::TeamA2aIntent::Blocker => "blocker".to_string(),
        super::state::TeamA2aIntent::HandoffReady => "handoff_ready".to_string(),
    });
    let phase = entry.phase.as_ref().map(|phase| match phase {
        super::state::TeamPhase::Bootstrap => "bootstrap".to_string(),
        super::state::TeamPhase::Design => "design".to_string(),
        super::state::TeamPhase::Development => "development".to_string(),
        super::state::TeamPhase::Review => "review".to_string(),
        super::state::TeamPhase::Replan => "replan".to_string(),
    });
    Ok(SanitizedTapeMemoryEntry {
        team_ref,
        kind: entry.kind,
        summary: sanitize_summary_for_export(&entry.summary, workspace_root),
        counterpart_team_ref,
        phase,
        anchor: entry.anchor.clone(),
        artifact_refs: entry.artifact_refs.clone(),
        peer_intent,
        created_at: entry.created_at.clone(),
    })
}

async fn sanitize_team_ref(workspace_root: &Path, team_id: &str) -> String {
    match load_team_state_bundle(workspace_root, team_id).await {
        Ok(Some(bundle)) => public_team_ref(
            &bundle.record.team_id,
            &bundle.record.role,
            bundle.record.depth,
            bundle.record.kind.clone(),
        ),
        _ => public_team_ref(team_id, "team", /*depth*/ 0, TeamKind::Child),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::team::runtime::maybe_initialize_for_thread;
    use codex_protocol::ThreadId;
    use codex_protocol::protocol::SessionSource;
    use codex_protocol::protocol::SubAgentSource;

    async fn write_workflow(path: &Path) {
        let codex_dir = path.join(".codex");
        tokio::fs::create_dir_all(&codex_dir)
            .await
            .expect("create .codex dir");
        tokio::fs::write(codex_dir.join("team-workflow.yaml"), "version: 1\n")
            .await
            .expect("write workflow");
    }

    #[tokio::test]
    async fn sanitize_tape_export_redacts_workspace_root_and_hidden_team_ids() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        write_workflow(temp_dir.path()).await;
        let root_thread_id = ThreadId::new();
        let child_thread_id = ThreadId::new();

        maybe_initialize_for_thread(temp_dir.path(), root_thread_id, &SessionSource::Exec, None)
            .await
            .expect("initialize root team");
        maybe_initialize_for_thread(
            temp_dir.path(),
            child_thread_id,
            &SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: Some("Ada".to_string()),
                agent_role: Some("development-lead".to_string()),
            }),
            None,
        )
        .await
        .expect("initialize child team");

        let sanitized = sanitize_tape_entry(
            temp_dir.path(),
            &TeamTapeEntry {
                entry_id: "entry-1".to_string(),
                team_id: child_thread_id.to_string(),
                kind: TeamTapeKind::PeerSync,
                summary: format!(
                    "share bounded context from {}\\artifacts\\handoff.md",
                    temp_dir.path().display()
                ),
                counterpart_team_id: Some(root_thread_id.to_string()),
                phase: Some(super::super::state::TeamPhase::Development),
                anchor: Some("delivery".to_string()),
                artifact_refs: vec![PathBuf::from("artifacts/handoff.md")],
                peer_message: Some(super::super::state::TeamA2aEnvelope {
                    protocol: "codex-a2a".to_string(),
                    version: 1,
                    sender_public_id: "team-d1-development-lead-abcd1234".to_string(),
                    recipient_public_id: "root-scheduler".to_string(),
                    relationship: super::super::state::TeamA2aRelationship::Sibling,
                    phase: super::super::state::TeamPhase::Development,
                    intent: super::super::state::TeamA2aIntent::Align,
                    summary: "bounded".to_string(),
                    artifact_refs: vec![PathBuf::from("artifacts/handoff.md")],
                    reply_needed: true,
                }),
                created_at: "2026-03-18T00:00:00Z".to_string(),
            },
        )
        .await
        .expect("sanitize entry");

        assert!(sanitized.team_ref.starts_with("team-d1-development-lead-"));
        assert!(!sanitized.team_ref.contains(&child_thread_id.to_string()));
        assert_eq!(
            sanitized.counterpart_team_ref,
            Some("root-scheduler".to_string())
        );
        assert_eq!(sanitized.peer_intent, Some("align".to_string()));
        let json = serde_json::to_value(sanitized).expect("serialize");
        assert!(json.get("workspaceRoot").is_none());
        assert!(json.to_string().contains("root-scheduler"));
        assert!(json.to_string().contains("workspace-root"));
        assert!(
            !json
                .to_string()
                .contains(&temp_dir.path().display().to_string())
        );
    }
}
