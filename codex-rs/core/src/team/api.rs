use super::state::{
    TeamIntegrationHandoff, TeamIntegrationMode, TeamKind, TeamManagedResource,
    TeamManagedResourceKind, TeamManagedResourceStatus, TeamPhase, TeamTapeEntry, TeamTapeKind,
    TeamWorktreeState, load_team_state_bundle,
};
use codex_protocol::ThreadId;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamWorkflowThreadVisibility {
    NotTeam,
    PublicRoot,
    HiddenChild { root_thread_id: ThreadId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamWorkflowPublicTeamKind {
    Root,
    Child,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamWorkflowPublicPhase {
    Bootstrap,
    Design,
    Development,
    Review,
    Replan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamWorkflowPublicResourceKind {
    Worktree,
    TestEnvironment,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamWorkflowPublicResourceStatus {
    Active,
    Stale,
    Cleaned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamWorkflowPublicTapeKind {
    Bootstrap,
    WorktreeAssigned,
    Delegation,
    PeerSync,
    ArtifactHandoff,
    Resume,
    IntegrationReady,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamWorkflowPublicIntegrationMode {
    Merge,
    CherryPick,
    Patch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamWorkflowPublicResource {
    pub resource_id: String,
    pub kind: TeamWorkflowPublicResourceKind,
    pub path: Option<PathBuf>,
    pub status: TeamWorkflowPublicResourceStatus,
    pub cleanup_required: bool,
    pub last_verified_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamWorkflowPublicEnvironment {
    pub managed_resources: Vec<TeamWorkflowPublicResource>,
    pub stale_resources: Vec<TeamWorkflowPublicResource>,
    pub cleanup_notes: Vec<String>,
    pub last_cleanup_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamWorkflowPublicWorktree {
    pub branch_name: String,
    pub current_branch: Option<String>,
    pub checkout_path: PathBuf,
    pub source_checkout_path: Option<PathBuf>,
    pub repo_root: Option<PathBuf>,
    pub base_commit: Option<String>,
    pub head_commit: Option<String>,
    pub managed: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamWorkflowPublicIntegration {
    pub source_team_id: String,
    pub target_team_id: Option<String>,
    pub source_branch: Option<String>,
    pub source_checkout_path: PathBuf,
    pub target_checkout_path: Option<PathBuf>,
    pub base_commit: Option<String>,
    pub head_commit: Option<String>,
    pub patch_path: Option<PathBuf>,
    pub accepted_modes: Vec<TeamWorkflowPublicIntegrationMode>,
    pub review_ready: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamWorkflowPublicTapeEntry {
    pub entry_id: String,
    pub team_id: String,
    pub kind: TeamWorkflowPublicTapeKind,
    pub summary: String,
    pub counterpart_team_id: Option<String>,
    pub phase: Option<TeamWorkflowPublicPhase>,
    pub anchor: Option<String>,
    pub artifact_refs: Vec<PathBuf>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamWorkflowPublicTeam {
    pub team_id: String,
    pub thread_id: ThreadId,
    pub parent_team_id: Option<String>,
    pub depth: i32,
    pub kind: TeamWorkflowPublicTeamKind,
    pub role: String,
    pub nickname: Option<String>,
    pub current_phase: TeamWorkflowPublicPhase,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
    pub active_child_team_ids: Vec<String>,
    pub governance_doc_path: PathBuf,
    pub global_governance_path: PathBuf,
    pub produced_artifacts: Vec<String>,
    pub worktree: Option<TeamWorkflowPublicWorktree>,
    pub environment: TeamWorkflowPublicEnvironment,
    pub integration: Option<TeamWorkflowPublicIntegration>,
    pub recent_tape: Vec<TeamWorkflowPublicTapeEntry>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamWorkflowPublicSession {
    pub root_thread_id: ThreadId,
    pub root_team_id: String,
    pub root_role: String,
    pub current_phase: TeamWorkflowPublicPhase,
    pub max_depth: i32,
    pub active_team_count: usize,
    pub blocked_team_count: usize,
    pub stale_resource_count: usize,
    pub global_governance_path: PathBuf,
    pub team_state_index_path: PathBuf,
    pub teams: Vec<TeamWorkflowPublicTeam>,
    pub updated_at: String,
}

pub async fn team_workflow_thread_visibility(
    workspace_root: &Path,
    thread_id: ThreadId,
) -> io::Result<TeamWorkflowThreadVisibility> {
    let Some(bundle) = load_team_state_bundle(workspace_root, &thread_id.to_string()).await? else {
        return Ok(TeamWorkflowThreadVisibility::NotTeam);
    };
    if bundle.record.kind == TeamKind::Root {
        return Ok(TeamWorkflowThreadVisibility::PublicRoot);
    }
    let Some(root_bundle) = resolve_root_bundle(workspace_root, &bundle.record.team_id).await?
    else {
        return Ok(TeamWorkflowThreadVisibility::NotTeam);
    };
    Ok(TeamWorkflowThreadVisibility::HiddenChild {
        root_thread_id: root_bundle.record.thread_id,
    })
}

pub async fn load_public_team_workflow_session(
    workspace_root: &Path,
    thread_id: ThreadId,
    recent_tape_limit: usize,
) -> io::Result<Option<TeamWorkflowPublicSession>> {
    let Some(bundle) = load_team_state_bundle(workspace_root, &thread_id.to_string()).await? else {
        return Ok(None);
    };
    let Some(root_bundle) = resolve_root_bundle(workspace_root, &bundle.record.team_id).await?
    else {
        return Ok(None);
    };

    let mut teams = Vec::new();
    let mut pending = vec![root_bundle.record.team_id.clone()];
    let mut visited = HashSet::new();
    while let Some(team_id) = pending.pop() {
        if !visited.insert(team_id.clone()) {
            continue;
        }
        let Some(bundle) = load_team_state_bundle(workspace_root, &team_id).await? else {
            continue;
        };
        pending.extend(bundle.record.child_team_ids.iter().rev().cloned());
        teams.push(public_team_from_bundle(&bundle, recent_tape_limit).await?);
    }
    teams.sort_by(|left, right| {
        left.depth
            .cmp(&right.depth)
            .then_with(|| left.team_id.cmp(&right.team_id))
    });

    let blocked_team_count = teams
        .iter()
        .filter(|team| !team.blockers.is_empty())
        .count();
    let stale_resource_count = teams
        .iter()
        .map(|team| team.environment.stale_resources.len())
        .sum();
    Ok(Some(TeamWorkflowPublicSession {
        root_thread_id: root_bundle.record.thread_id,
        root_team_id: root_bundle.record.team_id.clone(),
        root_role: root_bundle.record.role.clone(),
        current_phase: map_phase(&root_bundle.status.current_phase),
        max_depth: root_bundle.record.max_depth,
        active_team_count: teams.len(),
        blocked_team_count,
        stale_resource_count,
        global_governance_path: root_bundle.paths.global_doc_path.clone(),
        team_state_index_path: root_bundle.paths.index_path.clone(),
        teams,
        updated_at: root_bundle.status.updated_at.clone(),
    }))
}

async fn resolve_root_bundle(
    workspace_root: &Path,
    team_id: &str,
) -> io::Result<Option<super::state::TeamStateBundle>> {
    let mut current = load_team_state_bundle(workspace_root, team_id).await?;
    while let Some(bundle) = current {
        if bundle.record.kind == TeamKind::Root {
            return Ok(Some(bundle));
        }
        let Some(parent_team_id) = bundle.record.parent_team_id.clone() else {
            return Ok(None);
        };
        current = load_team_state_bundle(workspace_root, &parent_team_id).await?;
    }
    Ok(None)
}

async fn public_team_from_bundle(
    bundle: &super::state::TeamStateBundle,
    recent_tape_limit: usize,
) -> io::Result<TeamWorkflowPublicTeam> {
    Ok(TeamWorkflowPublicTeam {
        team_id: bundle.record.team_id.clone(),
        thread_id: bundle.record.thread_id,
        parent_team_id: bundle.record.parent_team_id.clone(),
        depth: bundle.record.depth,
        kind: map_team_kind(bundle.record.kind.clone()),
        role: bundle.record.role.clone(),
        nickname: bundle.record.nickname.clone(),
        current_phase: map_phase(&bundle.status.current_phase),
        blockers: bundle.status.blockers.clone(),
        next_steps: bundle.status.next_steps.clone(),
        active_child_team_ids: bundle.status.active_child_teams.clone(),
        governance_doc_path: bundle.paths.team_doc_path.clone(),
        global_governance_path: bundle.paths.global_doc_path.clone(),
        produced_artifacts: bundle.handoff.produced_artifacts.clone(),
        worktree: bundle.record.worktree.as_ref().map(map_worktree),
        environment: map_environment(&bundle.status.environment),
        integration: bundle.handoff.integration.as_ref().map(map_integration),
        recent_tape: load_public_tape_entries(&bundle.paths.tape_path, recent_tape_limit).await?,
        updated_at: bundle.status.updated_at.clone(),
    })
}

async fn load_public_tape_entries(
    tape_path: &Path,
    recent_tape_limit: usize,
) -> io::Result<Vec<TeamWorkflowPublicTapeEntry>> {
    let contents = match fs::read_to_string(tape_path).await {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };
    let mut entries = contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str::<TeamTapeEntry>(line)
                .map(redact_tape_entry)
                .map_err(|err| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("failed to parse {}: {err}", tape_path.display()),
                    )
                })
        })
        .collect::<io::Result<Vec<_>>>()?;
    if recent_tape_limit > 0 && entries.len() > recent_tape_limit {
        entries = entries.split_off(entries.len() - recent_tape_limit);
    }
    Ok(entries)
}

fn redact_tape_entry(entry: TeamTapeEntry) -> TeamWorkflowPublicTapeEntry {
    TeamWorkflowPublicTapeEntry {
        entry_id: entry.entry_id,
        team_id: entry.team_id,
        kind: map_tape_kind(entry.kind),
        summary: redact_tape_summary(entry.kind),
        counterpart_team_id: entry.counterpart_team_id,
        phase: entry.phase.as_ref().map(map_phase),
        anchor: entry.anchor,
        artifact_refs: entry.artifact_refs,
        created_at: entry.created_at,
    }
}

fn redact_tape_summary(kind: TeamTapeKind) -> String {
    match kind {
        TeamTapeKind::Bootstrap => "Team bootstrap persisted.".to_string(),
        TeamTapeKind::WorktreeAssigned => "Checkout assignment updated.".to_string(),
        TeamTapeKind::Delegation => "Delegation checkpoint recorded.".to_string(),
        TeamTapeKind::PeerSync => "Sibling coordination checkpoint recorded.".to_string(),
        TeamTapeKind::ArtifactHandoff => "Artifact handoff recorded.".to_string(),
        TeamTapeKind::Resume => "Leader resumed from persisted artifacts.".to_string(),
        TeamTapeKind::IntegrationReady => "Integration-ready handoff recorded.".to_string(),
    }
}

fn map_team_kind(kind: TeamKind) -> TeamWorkflowPublicTeamKind {
    match kind {
        TeamKind::Root => TeamWorkflowPublicTeamKind::Root,
        TeamKind::Child => TeamWorkflowPublicTeamKind::Child,
    }
}

fn map_phase(phase: &TeamPhase) -> TeamWorkflowPublicPhase {
    match phase {
        TeamPhase::Bootstrap => TeamWorkflowPublicPhase::Bootstrap,
        TeamPhase::Design => TeamWorkflowPublicPhase::Design,
        TeamPhase::Development => TeamWorkflowPublicPhase::Development,
        TeamPhase::Review => TeamWorkflowPublicPhase::Review,
        TeamPhase::Replan => TeamWorkflowPublicPhase::Replan,
    }
}

fn map_worktree(worktree: &TeamWorktreeState) -> TeamWorkflowPublicWorktree {
    TeamWorkflowPublicWorktree {
        branch_name: worktree.branch_name.clone(),
        current_branch: worktree.current_branch.clone(),
        checkout_path: worktree.checkout_path.clone(),
        source_checkout_path: worktree.source_checkout_path.clone(),
        repo_root: worktree.repo_root.clone(),
        base_commit: worktree.base_commit.clone(),
        head_commit: worktree.head_commit.clone(),
        managed: worktree.managed,
        updated_at: worktree.updated_at.clone(),
    }
}

fn map_environment(
    environment: &super::state::TeamEnvironmentState,
) -> TeamWorkflowPublicEnvironment {
    TeamWorkflowPublicEnvironment {
        managed_resources: environment
            .managed_resources
            .iter()
            .map(map_resource)
            .collect(),
        stale_resources: environment
            .stale_resources
            .iter()
            .map(map_resource)
            .collect(),
        cleanup_notes: environment.cleanup_notes.clone(),
        last_cleanup_at: environment.last_cleanup_at.clone(),
    }
}

fn map_resource(resource: &TeamManagedResource) -> TeamWorkflowPublicResource {
    TeamWorkflowPublicResource {
        resource_id: resource.resource_id.clone(),
        kind: match resource.kind {
            TeamManagedResourceKind::Worktree => TeamWorkflowPublicResourceKind::Worktree,
            TeamManagedResourceKind::TestEnvironment => {
                TeamWorkflowPublicResourceKind::TestEnvironment
            }
            TeamManagedResourceKind::Other => TeamWorkflowPublicResourceKind::Other,
        },
        path: resource.path.clone(),
        status: match resource.status {
            TeamManagedResourceStatus::Active => TeamWorkflowPublicResourceStatus::Active,
            TeamManagedResourceStatus::Stale => TeamWorkflowPublicResourceStatus::Stale,
            TeamManagedResourceStatus::Cleaned => TeamWorkflowPublicResourceStatus::Cleaned,
        },
        cleanup_required: resource.cleanup_required,
        last_verified_at: resource.last_verified_at.clone(),
    }
}

fn map_integration(integration: &TeamIntegrationHandoff) -> TeamWorkflowPublicIntegration {
    TeamWorkflowPublicIntegration {
        source_team_id: integration.source_team_id.clone(),
        target_team_id: integration.target_team_id.clone(),
        source_branch: integration.source_branch.clone(),
        source_checkout_path: integration.source_checkout_path.clone(),
        target_checkout_path: integration.target_checkout_path.clone(),
        base_commit: integration.base_commit.clone(),
        head_commit: integration.head_commit.clone(),
        patch_path: integration.patch_path.clone(),
        accepted_modes: integration
            .accepted_modes
            .iter()
            .map(|mode| match mode {
                TeamIntegrationMode::Merge => TeamWorkflowPublicIntegrationMode::Merge,
                TeamIntegrationMode::CherryPick => TeamWorkflowPublicIntegrationMode::CherryPick,
                TeamIntegrationMode::Patch => TeamWorkflowPublicIntegrationMode::Patch,
            })
            .collect(),
        review_ready: integration.review_ready,
        updated_at: integration.updated_at.clone(),
    }
}

fn map_tape_kind(kind: TeamTapeKind) -> TeamWorkflowPublicTapeKind {
    match kind {
        TeamTapeKind::Bootstrap => TeamWorkflowPublicTapeKind::Bootstrap,
        TeamTapeKind::WorktreeAssigned => TeamWorkflowPublicTapeKind::WorktreeAssigned,
        TeamTapeKind::Delegation => TeamWorkflowPublicTapeKind::Delegation,
        TeamTapeKind::PeerSync => TeamWorkflowPublicTapeKind::PeerSync,
        TeamTapeKind::ArtifactHandoff => TeamWorkflowPublicTapeKind::ArtifactHandoff,
        TeamTapeKind::Resume => TeamWorkflowPublicTapeKind::Resume,
        TeamTapeKind::IntegrationReady => TeamWorkflowPublicTapeKind::IntegrationReady,
    }
}
