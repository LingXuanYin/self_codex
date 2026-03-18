use super::config::CrossLevelHandoffPolicy;
use super::config::GLOBAL_AGENT_DOC_FILENAME;
use super::config::IterationRole;
use super::config::SameLevelContextProtocol;
use super::config::TEAM_AGENT_DOC_FILENAME;
use super::config::TEAM_ARTIFACTS_DIRNAME;
use super::config::TEAM_DIRNAME;
use super::config::TEAM_STATE_DIRNAME;
use super::config::TeamWorkflowConfig;
use super::config::resolve_team_home_root;
use super::memory::TeamMemoryProviderStatus;
use super::memory::mirror_entry_to_provider;
use super::redaction::sanitize_summary_text;
use super::redaction::sanitize_workspace_paths;
use chrono::Utc;
use codex_protocol::ThreadId;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

pub(crate) const TEAM_INDEX_FILENAME: &str = "index.json";
pub(crate) const TEAM_METADATA_FILENAME: &str = "team.json";
pub(crate) const TEAM_STATUS_FILENAME: &str = "status.json";
pub(crate) const TEAM_HANDOFF_FILENAME: &str = "handoff.json";
pub(crate) const TEAM_RECOVERY_FILENAME: &str = "recovery.json";
pub(crate) const TEAM_AUDIT_FILENAME: &str = "audit.json";
pub(crate) const TEAM_TAPE_FILENAME: &str = "team-tape.jsonl";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TeamStatePaths {
    pub team_root: PathBuf,
    pub team_dir: PathBuf,
    pub artifacts_dir: PathBuf,
    pub team_metadata_path: PathBuf,
    pub status_path: PathBuf,
    pub handoff_path: PathBuf,
    pub recovery_path: PathBuf,
    pub audit_path: PathBuf,
    pub tape_path: PathBuf,
    pub team_doc_path: PathBuf,
    pub global_doc_path: PathBuf,
    pub index_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TeamKind {
    Root,
    Child,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TeamStateRecord {
    pub team_id: String,
    pub thread_id: ThreadId,
    pub team_template_id: Option<String>,
    pub parent_team_id: Option<String>,
    pub parent_thread_id: Option<ThreadId>,
    pub child_team_ids: Vec<String>,
    pub depth: i32,
    pub kind: TeamKind,
    pub role: String,
    pub nickname: Option<String>,
    pub workspace_root: PathBuf,
    pub workflow_path: PathBuf,
    pub rollout_path: Option<PathBuf>,
    pub artifacts_dir: PathBuf,
    pub status_path: PathBuf,
    pub handoff_path: PathBuf,
    pub team_doc_path: PathBuf,
    pub global_doc_path: PathBuf,
    pub worktree: Option<TeamWorktreeState>,
    pub max_depth: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TeamWorktreeState {
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
#[serde(rename_all = "snake_case")]
pub(crate) enum TeamLifecycle {
    Active,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TeamPhase {
    Bootstrap,
    Design,
    Development,
    Review,
    Replan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TeamRoleState {
    Pending,
    Active,
    Complete,
    NeedsRework,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TeamRoleAssignment {
    pub role: IterationRole,
    pub state: TeamRoleState,
    pub owner_team_id: Option<String>,
    pub owner_thread_id: Option<ThreadId>,
    pub owner_role: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TeamCycleSnapshot {
    pub cycle_id: u32,
    pub phase: TeamPhase,
    pub roles: Vec<TeamRoleAssignment>,
    pub replan_reason: Option<String>,
    pub last_transition_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TeamStatusSnapshot {
    pub lifecycle: TeamLifecycle,
    pub current_phase: TeamPhase,
    pub compact_safe: bool,
    pub required_roles: Vec<IterationRole>,
    pub cycle: TeamCycleSnapshot,
    pub same_level_context_protocol: SameLevelContextProtocol,
    pub cross_level_handoff: CrossLevelHandoffPolicy,
    pub memory_provider: TeamMemoryProviderStatus,
    pub governance_docs: Vec<PathBuf>,
    pub active_child_teams: Vec<String>,
    pub worktree: Option<TeamWorktreeState>,
    #[serde(default)]
    pub environment: TeamEnvironmentState,
    pub recovery_path: PathBuf,
    pub audit_path: PathBuf,
    #[serde(default)]
    pub tape_path: PathBuf,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TeamHandoffMetadata {
    pub summary: String,
    pub artifact_root: PathBuf,
    pub produced_artifacts: Vec<String>,
    pub integration: Option<TeamIntegrationHandoff>,
    pub governance_docs: Vec<PathBuf>,
    pub next_steps: Vec<String>,
    pub blockers: Vec<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TeamIntegrationMode {
    Merge,
    CherryPick,
    Patch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TeamIntegrationHandoff {
    pub source_team_id: String,
    pub target_team_id: Option<String>,
    pub source_branch: Option<String>,
    pub source_checkout_path: PathBuf,
    pub target_checkout_path: Option<PathBuf>,
    pub base_commit: Option<String>,
    pub head_commit: Option<String>,
    pub patch_path: Option<PathBuf>,
    pub accepted_modes: Vec<TeamIntegrationMode>,
    pub review_ready: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TeamRecoverySnapshot {
    pub compact_safe: bool,
    pub restore_from_artifacts: bool,
    pub status_path: PathBuf,
    pub handoff_path: PathBuf,
    pub governance_docs: Vec<PathBuf>,
    pub active_child_teams: Vec<String>,
    #[serde(default)]
    pub environment: TeamEnvironmentState,
    #[serde(default)]
    pub tape_path: PathBuf,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
    pub last_resumed_at: Option<String>,
    pub last_compact_checkpoint_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TeamAuditKind {
    Delegation,
    PeerSync,
    ArtifactHandoff,
    Resume,
    Correction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TeamAuditEntry {
    pub kind: TeamAuditKind,
    pub counterpart_team_id: Option<String>,
    pub counterpart_thread_id: Option<ThreadId>,
    pub summary: String,
    pub artifact_refs: Vec<PathBuf>,
    pub detected_instruction_drift: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TeamAuditLog {
    pub entries: Vec<TeamAuditEntry>,
    pub synthesized_skills: Vec<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TeamManagedResourceKind {
    Worktree,
    TestEnvironment,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TeamManagedResourceStatus {
    Active,
    Stale,
    Cleaned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TeamManagedResource {
    pub resource_id: String,
    pub kind: TeamManagedResourceKind,
    pub path: Option<PathBuf>,
    pub status: TeamManagedResourceStatus,
    pub cleanup_required: bool,
    pub last_verified_at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TeamEnvironmentState {
    pub managed_resources: Vec<TeamManagedResource>,
    pub stale_resources: Vec<TeamManagedResource>,
    pub cleanup_notes: Vec<String>,
    pub last_cleanup_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TeamTapeKind {
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
pub(crate) enum TeamA2aRelationship {
    SameTeam,
    Sibling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TeamA2aIntent {
    Align,
    Request,
    Answer,
    Blocker,
    HandoffReady,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TeamA2aEnvelope {
    pub protocol: String,
    pub version: u32,
    pub sender_public_id: String,
    pub recipient_public_id: String,
    pub relationship: TeamA2aRelationship,
    pub phase: TeamPhase,
    pub intent: TeamA2aIntent,
    pub summary: String,
    pub artifact_refs: Vec<PathBuf>,
    pub reply_needed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TeamTapeEntry {
    pub entry_id: String,
    pub team_id: String,
    pub kind: TeamTapeKind,
    pub summary: String,
    pub counterpart_team_id: Option<String>,
    pub phase: Option<TeamPhase>,
    pub anchor: Option<String>,
    pub artifact_refs: Vec<PathBuf>,
    #[serde(default)]
    pub peer_message: Option<TeamA2aEnvelope>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TeamStateWriteRequest {
    pub team_id: String,
    pub thread_id: ThreadId,
    pub parent_thread_id: Option<ThreadId>,
    pub depth: i32,
    pub kind: TeamKind,
    pub role: String,
    pub nickname: Option<String>,
    pub workspace_root: PathBuf,
    pub workflow_path: PathBuf,
    pub rollout_path: Option<PathBuf>,
    pub workflow: TeamWorkflowConfig,
}

#[derive(Debug, Clone)]
pub(crate) struct TeamStateBundle {
    pub paths: TeamStatePaths,
    pub record: TeamStateRecord,
    pub status: TeamStatusSnapshot,
    pub handoff: TeamHandoffMetadata,
    pub recovery: TeamRecoverySnapshot,
    pub audit: TeamAuditLog,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct TeamRegistry {
    teams: BTreeMap<String, TeamRegistryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TeamRegistryEntry {
    team_id: String,
    kind: TeamKind,
    depth: i32,
    role: String,
    parent_team_id: Option<String>,
    team_doc_path: PathBuf,
    status_path: PathBuf,
    handoff_path: PathBuf,
    updated_at: String,
}

pub(crate) fn team_state_paths(
    workspace_root: &Path,
    team_id: &str,
    artifact_directory: &str,
) -> TeamStatePaths {
    let team_home_root = resolve_team_home_root(workspace_root);
    let team_root = team_home_root.join(TEAM_DIRNAME).join(TEAM_STATE_DIRNAME);
    let team_dir = team_root.join(team_id);
    TeamStatePaths {
        artifacts_dir: team_dir.join(artifact_directory),
        team_metadata_path: team_dir.join(TEAM_METADATA_FILENAME),
        status_path: team_dir.join(TEAM_STATUS_FILENAME),
        handoff_path: team_dir.join(TEAM_HANDOFF_FILENAME),
        recovery_path: team_dir.join(TEAM_RECOVERY_FILENAME),
        audit_path: team_dir.join(TEAM_AUDIT_FILENAME),
        tape_path: team_dir.join(TEAM_TAPE_FILENAME),
        team_doc_path: team_dir.join(TEAM_AGENT_DOC_FILENAME),
        global_doc_path: team_home_root
            .join(TEAM_DIRNAME)
            .join(GLOBAL_AGENT_DOC_FILENAME),
        index_path: team_root.join(TEAM_INDEX_FILENAME),
        team_root,
        team_dir,
    }
}

pub(crate) async fn persist_team_state(
    request: TeamStateWriteRequest,
) -> io::Result<(TeamStatePaths, TeamStateRecord)> {
    let TeamStateWriteRequest {
        team_id,
        thread_id,
        parent_thread_id,
        depth,
        kind,
        role,
        nickname,
        workspace_root,
        workflow_path,
        rollout_path,
        workflow,
    } = request;
    let paths = team_state_paths(
        &workspace_root,
        &team_id,
        workflow.artifact_policy.artifact_directory.as_str(),
    );
    fs::create_dir_all(&paths.team_dir).await?;
    fs::create_dir_all(&paths.artifacts_dir).await?;

    let existing = read_json_if_exists::<TeamStateRecord>(&paths.team_metadata_path).await?;
    let existing_status = read_json_if_exists::<TeamStatusSnapshot>(&paths.status_path).await?;
    let existing_handoff = read_json_if_exists::<TeamHandoffMetadata>(&paths.handoff_path).await?;
    let existing_recovery =
        read_json_if_exists::<TeamRecoverySnapshot>(&paths.recovery_path).await?;
    let existing_audit = read_json_if_exists::<TeamAuditLog>(&paths.audit_path).await?;
    let now = Utc::now().to_rfc3339();
    let team_template_id = match workflow
        .team_templates
        .iter()
        .find(|template| template_matches_role(template, &role))
    {
        Some(template) => Some(template.id.clone()),
        None => existing
            .as_ref()
            .and_then(|record| record.team_template_id.clone()),
    };
    let record = TeamStateRecord {
        team_id: team_id.clone(),
        thread_id,
        team_template_id,
        parent_team_id: parent_thread_id.map(|thread_id| thread_id.to_string()),
        parent_thread_id,
        child_team_ids: existing
            .as_ref()
            .map(|record| record.child_team_ids.clone())
            .unwrap_or_default(),
        depth,
        kind: kind.clone(),
        role: role.clone(),
        nickname,
        workspace_root: workspace_root.clone(),
        workflow_path,
        rollout_path,
        artifacts_dir: paths.artifacts_dir.clone(),
        status_path: paths.status_path.clone(),
        handoff_path: paths.handoff_path.clone(),
        team_doc_path: paths.team_doc_path.clone(),
        global_doc_path: paths.global_doc_path.clone(),
        worktree: existing.as_ref().and_then(|record| record.worktree.clone()),
        max_depth: workflow.max_depth,
        created_at: existing
            .as_ref()
            .map(|record| record.created_at.clone())
            .unwrap_or_else(|| now.clone()),
        updated_at: now.clone(),
    };
    write_json_pretty(&paths.team_metadata_path, &record).await?;

    let mut status = existing_status.unwrap_or_else(|| TeamStatusSnapshot {
        lifecycle: TeamLifecycle::Active,
        current_phase: TeamPhase::Bootstrap,
        compact_safe: workflow.workflow_loop.persist_before_compact,
        required_roles: workflow.workflow_loop.required_roles.clone(),
        cycle: default_cycle_snapshot(&workflow.workflow_loop.required_roles, &now),
        same_level_context_protocol: workflow.artifact_policy.same_level_context_protocol,
        cross_level_handoff: workflow.artifact_policy.cross_level_handoff,
        memory_provider: TeamMemoryProviderStatus::for_config(&workflow.memory_provider),
        governance_docs: vec![paths.global_doc_path.clone(), paths.team_doc_path.clone()],
        active_child_teams: record.child_team_ids.clone(),
        worktree: record.worktree.clone(),
        environment: TeamEnvironmentState::default(),
        recovery_path: paths.recovery_path.clone(),
        audit_path: paths.audit_path.clone(),
        tape_path: paths.tape_path.clone(),
        blockers: Vec::new(),
        next_steps: vec![
            "Capture design, implementation, and review outputs in persisted artifacts."
                .to_string(),
            "Update governance docs before compact, replan, or team handoff.".to_string(),
        ],
        updated_at: now.clone(),
    });
    status.compact_safe = workflow.workflow_loop.persist_before_compact;
    status.required_roles = workflow.workflow_loop.required_roles.clone();
    status.same_level_context_protocol = workflow.artifact_policy.same_level_context_protocol;
    status.cross_level_handoff = workflow.artifact_policy.cross_level_handoff;
    status.memory_provider = TeamMemoryProviderStatus::for_config(&workflow.memory_provider);
    status.governance_docs = vec![paths.global_doc_path.clone(), paths.team_doc_path.clone()];
    status.active_child_teams = record.child_team_ids.clone();
    status.worktree = record.worktree.clone();
    status.recovery_path = paths.recovery_path.clone();
    status.audit_path = paths.audit_path.clone();
    status.tape_path = paths.tape_path.clone();
    status.current_phase = status.cycle.phase.clone();
    seed_local_role_owner(
        &mut status.cycle,
        &record.team_id,
        record.thread_id,
        &record.role,
        &now,
    );
    status.updated_at = now.clone();
    write_json_pretty(&paths.status_path, &status).await?;

    let mut handoff = existing_handoff.unwrap_or_else(|| TeamHandoffMetadata {
        summary: "Bootstrap team state persisted for compact-safe recovery.".to_string(),
        artifact_root: paths.artifacts_dir.clone(),
        produced_artifacts: Vec::new(),
        integration: None,
        governance_docs: vec![paths.global_doc_path.clone(), paths.team_doc_path.clone()],
        next_steps: vec![
            "Use OpenSpec-aligned artifacts for cross-level handoffs.".to_string(),
            "Recover from status.json and AGENT_TEAM.md before relying on hidden transcript context."
                .to_string(),
        ],
        blockers: Vec::new(),
        updated_at: now.clone(),
    });
    handoff.artifact_root = paths.artifacts_dir.clone();
    handoff.governance_docs = vec![paths.global_doc_path.clone(), paths.team_doc_path.clone()];
    handoff.updated_at = now.clone();
    write_json_pretty(&paths.handoff_path, &handoff).await?;

    let mut recovery = existing_recovery.unwrap_or_else(|| TeamRecoverySnapshot {
        compact_safe: workflow.workflow_loop.persist_before_compact,
        restore_from_artifacts: workflow.workflow_loop.resume_from_artifacts,
        status_path: paths.status_path.clone(),
        handoff_path: paths.handoff_path.clone(),
        governance_docs: vec![paths.global_doc_path.clone(), paths.team_doc_path.clone()],
        active_child_teams: record.child_team_ids.clone(),
        environment: TeamEnvironmentState::default(),
        tape_path: paths.tape_path.clone(),
        blockers: status.blockers.clone(),
        next_steps: status.next_steps.clone(),
        last_resumed_at: None,
        last_compact_checkpoint_at: None,
        updated_at: now.clone(),
    });
    recovery.compact_safe = workflow.workflow_loop.persist_before_compact;
    recovery.restore_from_artifacts = workflow.workflow_loop.resume_from_artifacts;
    recovery.status_path = paths.status_path.clone();
    recovery.handoff_path = paths.handoff_path.clone();
    recovery.governance_docs = vec![paths.global_doc_path.clone(), paths.team_doc_path.clone()];
    recovery.active_child_teams = record.child_team_ids.clone();
    recovery.tape_path = paths.tape_path.clone();
    recovery.blockers = status.blockers.clone();
    recovery.next_steps = status.next_steps.clone();
    recovery.updated_at = now.clone();
    write_json_pretty(&paths.recovery_path, &recovery).await?;

    let mut audit = existing_audit.unwrap_or_else(|| TeamAuditLog {
        entries: Vec::new(),
        synthesized_skills: Vec::new(),
        updated_at: now.clone(),
    });
    audit.synthesized_skills = synthesize_internal_skills(&audit.entries);
    audit.updated_at = now.clone();
    write_json_pretty(&paths.audit_path, &audit).await?;
    ensure_tape_file(&paths.tape_path).await?;

    let mut registry = read_json_if_exists::<TeamRegistry>(&paths.index_path)
        .await?
        .unwrap_or_default();
    registry.teams.insert(
        team_id.clone(),
        TeamRegistryEntry {
            team_id,
            kind,
            depth,
            role: role.clone(),
            parent_team_id: record.parent_team_id.clone(),
            team_doc_path: paths.team_doc_path.clone(),
            status_path: paths.status_path.clone(),
            handoff_path: paths.handoff_path.clone(),
            updated_at: now,
        },
    );
    write_json_pretty(&paths.index_path, &registry).await?;

    if let Some(parent_team_id) = record.parent_team_id.as_deref() {
        update_parent_team_links(
            &resolve_team_home_root(&workspace_root),
            parent_team_id,
            &record.team_id,
            record.thread_id,
            &role,
        )
        .await?;
    }

    Ok((paths, record))
}

pub(crate) async fn load_team_state_bundle(
    workspace_root: &Path,
    team_id: &str,
) -> io::Result<Option<TeamStateBundle>> {
    let team_home_root = resolve_team_home_root(workspace_root);
    let metadata_path = team_home_root
        .join(TEAM_DIRNAME)
        .join(TEAM_STATE_DIRNAME)
        .join(team_id)
        .join(TEAM_METADATA_FILENAME);
    let Some(record) = read_json_if_exists::<TeamStateRecord>(&metadata_path).await? else {
        return Ok(None);
    };
    let paths = team_state_paths(
        &team_home_root,
        team_id,
        record
            .artifacts_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(TEAM_ARTIFACTS_DIRNAME),
    );
    let status = read_json_if_exists::<TeamStatusSnapshot>(&paths.status_path)
        .await?
        .ok_or_else(|| missing_state_file(&paths.status_path))?;
    let handoff = read_json_if_exists::<TeamHandoffMetadata>(&paths.handoff_path)
        .await?
        .ok_or_else(|| missing_state_file(&paths.handoff_path))?;
    let recovery = read_json_if_exists::<TeamRecoverySnapshot>(&paths.recovery_path)
        .await?
        .ok_or_else(|| missing_state_file(&paths.recovery_path))?;
    let audit = read_json_if_exists::<TeamAuditLog>(&paths.audit_path)
        .await?
        .ok_or_else(|| missing_state_file(&paths.audit_path))?;
    Ok(Some(TeamStateBundle {
        paths,
        record,
        status,
        handoff,
        recovery,
        audit,
    }))
}

pub(crate) async fn write_team_state_bundle(bundle: &TeamStateBundle) -> io::Result<()> {
    write_json_pretty(&bundle.paths.team_metadata_path, &bundle.record).await?;
    write_json_pretty(&bundle.paths.status_path, &bundle.status).await?;
    write_json_pretty(&bundle.paths.handoff_path, &bundle.handoff).await?;
    write_json_pretty(&bundle.paths.recovery_path, &bundle.recovery).await?;
    write_json_pretty(&bundle.paths.audit_path, &bundle.audit).await
}

pub(crate) async fn load_team_worktree(
    workspace_root: &Path,
    team_id: &str,
) -> io::Result<Option<TeamWorktreeState>> {
    Ok(load_team_state_bundle(workspace_root, team_id)
        .await?
        .and_then(|bundle| bundle.record.worktree))
}

pub(crate) async fn update_team_worktree(
    workspace_root: &Path,
    team_id: &str,
    worktree: TeamWorktreeState,
) -> io::Result<Option<TeamStateBundle>> {
    let Some(mut bundle) = load_team_state_bundle(workspace_root, team_id).await? else {
        return Ok(None);
    };
    bundle.record.worktree = Some(worktree.clone());
    bundle.status.worktree = Some(worktree);
    let now = Utc::now().to_rfc3339();
    bundle.record.updated_at = now.clone();
    bundle.status.updated_at = now;
    write_team_state_bundle(&bundle).await?;
    Ok(Some(bundle))
}

pub(crate) async fn update_team_environment_state(
    workspace_root: &Path,
    team_id: &str,
    environment: TeamEnvironmentState,
) -> io::Result<Option<TeamStateBundle>> {
    let Some(mut bundle) = load_team_state_bundle(workspace_root, team_id).await? else {
        return Ok(None);
    };
    bundle.status.environment = environment.clone();
    bundle.recovery.environment = environment;
    let now = Utc::now().to_rfc3339();
    bundle.status.updated_at = now.clone();
    bundle.recovery.updated_at = now;
    write_team_state_bundle(&bundle).await?;
    Ok(Some(bundle))
}

pub(crate) async fn append_team_tape_entry(
    workspace_root: &Path,
    team_id: &str,
    kind: TeamTapeKind,
    summary: impl Into<String>,
    counterpart_team_id: Option<String>,
    artifact_refs: Vec<PathBuf>,
    anchor: Option<String>,
    peer_message: Option<TeamA2aEnvelope>,
) -> io::Result<TeamTapeEntry> {
    let Some(bundle) = load_team_state_bundle(workspace_root, team_id).await? else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("missing team state for tape append: {team_id}"),
        ));
    };
    ensure_tape_file(&bundle.paths.tape_path).await?;
    let summary = summary.into();
    let entry = TeamTapeEntry {
        entry_id: Uuid::new_v4().to_string(),
        team_id: bundle.record.team_id.clone(),
        kind,
        summary: sanitize_summary_text(&summary),
        counterpart_team_id,
        phase: Some(bundle.status.current_phase.clone()),
        anchor,
        artifact_refs: sanitize_workspace_paths(
            &artifact_refs,
            &bundle.record.workspace_root,
            "redacted-artifact",
        ),
        peer_message,
        created_at: Utc::now().to_rfc3339(),
    };
    let line = serde_json::to_string(&entry).map_err(io::Error::other)?;
    let mut existing = fs::read_to_string(&bundle.paths.tape_path)
        .await
        .unwrap_or_default();
    if !existing.is_empty() && !existing.ends_with('\n') {
        existing.push('\n');
    }
    existing.push_str(&line);
    existing.push('\n');
    fs::write(&bundle.paths.tape_path, existing).await?;
    if let Some(workflow) =
        super::config::load_workflow_from_workspace(&bundle.record.workspace_root).await?
    {
        let mut status_bundle = bundle.clone();
        status_bundle.status.memory_provider = mirror_entry_to_provider(
            &status_bundle.record.workspace_root,
            &workflow.memory_provider,
            &entry,
        )
        .await?;
        status_bundle.status.updated_at = Utc::now().to_rfc3339();
        write_team_state_bundle(&status_bundle).await?;
    }
    Ok(entry)
}

async fn ensure_tape_file(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    if fs::try_exists(path).await? {
        return Ok(());
    }
    fs::write(path, "").await
}

pub(crate) fn infer_iteration_role(role: &str) -> Option<IterationRole> {
    let normalized = role.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if normalized.contains("review") || normalized.contains("audit") || normalized.contains("qa") {
        return Some(IterationRole::Review);
    }
    if normalized.contains("design")
        || normalized.contains("architect")
        || normalized.contains("planner")
    {
        return Some(IterationRole::Design);
    }
    if normalized.contains("dev")
        || normalized.contains("worker")
        || normalized.contains("build")
        || normalized.contains("implement")
        || normalized.contains("coder")
    {
        return Some(IterationRole::Development);
    }
    None
}

pub(crate) fn apply_role_assignment(
    cycle: &mut TeamCycleSnapshot,
    team_id: &str,
    thread_id: ThreadId,
    owner_role: &str,
    now: &str,
) {
    let Some(iteration_role) = infer_iteration_role(owner_role) else {
        return;
    };
    if let Some(assignment) = cycle
        .roles
        .iter_mut()
        .find(|entry| entry.role == iteration_role)
    {
        assignment.state = TeamRoleState::Active;
        assignment.owner_team_id = Some(team_id.to_string());
        assignment.owner_thread_id = Some(thread_id);
        assignment.owner_role = Some(owner_role.to_string());
        assignment.updated_at = now.to_string();
    }
    cycle.phase = phase_for_iteration_role(iteration_role);
    cycle.last_transition_at = now.to_string();
}

pub(crate) fn mark_cycle_replan(cycle: &mut TeamCycleSnapshot, reason: String, now: &str) {
    cycle.phase = TeamPhase::Replan;
    cycle.replan_reason = Some(reason);
    cycle.last_transition_at = now.to_string();
    for assignment in &mut cycle.roles {
        if matches!(
            assignment.role,
            IterationRole::Design | IterationRole::Development
        ) {
            assignment.state = TeamRoleState::NeedsRework;
            assignment.updated_at = now.to_string();
        }
    }
}

pub(crate) fn mark_cycle_artifact_handoff(
    cycle: &mut TeamCycleSnapshot,
    sender_role: Option<&str>,
    summary: &str,
    now: &str,
) {
    if let Some(role_name) = sender_role
        && let Some(iteration_role) = infer_iteration_role(role_name)
    {
        if let Some(assignment) = cycle
            .roles
            .iter_mut()
            .find(|entry| entry.role == iteration_role)
        {
            assignment.state = TeamRoleState::Complete;
            assignment.updated_at = now.to_string();
        }
        cycle.phase = match iteration_role {
            IterationRole::Design => TeamPhase::Development,
            IterationRole::Development => TeamPhase::Review,
            IterationRole::Review => TeamPhase::Review,
        };
        cycle.last_transition_at = now.to_string();
    }
    if indicates_replan(summary) {
        mark_cycle_replan(cycle, summary.to_string(), now);
    }
}

pub(crate) fn indicates_replan(summary: &str) -> bool {
    let normalized = summary.to_ascii_lowercase();
    [
        "replan",
        "needs rework",
        "send back",
        "correct",
        "fix",
        "drift",
        "blocked",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

async fn read_json_if_exists<T>(path: &Path) -> io::Result<Option<T>>
where
    T: for<'de> Deserialize<'de>,
{
    let contents = match fs::read_to_string(path).await {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };
    let value = serde_json::from_str(&contents).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to parse {}: {err}", path.display()),
        )
    })?;
    Ok(Some(value))
}

async fn write_json_pretty<T>(path: &Path, value: &T) -> io::Result<()>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let contents = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    fs::write(path, contents).await
}

fn default_cycle_snapshot(required_roles: &[IterationRole], now: &str) -> TeamCycleSnapshot {
    TeamCycleSnapshot {
        cycle_id: 1,
        phase: TeamPhase::Bootstrap,
        roles: required_roles
            .iter()
            .copied()
            .map(|role| TeamRoleAssignment {
                role,
                state: TeamRoleState::Pending,
                owner_team_id: None,
                owner_thread_id: None,
                owner_role: None,
                updated_at: now.to_string(),
            })
            .collect(),
        replan_reason: None,
        last_transition_at: now.to_string(),
    }
}

fn seed_local_role_owner(
    cycle: &mut TeamCycleSnapshot,
    team_id: &str,
    thread_id: ThreadId,
    owner_role: &str,
    now: &str,
) {
    let Some(iteration_role) = infer_iteration_role(owner_role) else {
        return;
    };
    if let Some(assignment) = cycle
        .roles
        .iter_mut()
        .find(|entry| entry.role == iteration_role)
        && assignment.owner_team_id.is_none()
    {
        assignment.owner_team_id = Some(team_id.to_string());
        assignment.owner_thread_id = Some(thread_id);
        assignment.owner_role = Some(owner_role.to_string());
        if matches!(assignment.state, TeamRoleState::Pending) {
            assignment.state = TeamRoleState::Active;
            cycle.phase = phase_for_iteration_role(iteration_role);
            cycle.last_transition_at = now.to_string();
        }
        assignment.updated_at = now.to_string();
    }
}

fn phase_for_iteration_role(role: IterationRole) -> TeamPhase {
    match role {
        IterationRole::Design => TeamPhase::Design,
        IterationRole::Development => TeamPhase::Development,
        IterationRole::Review => TeamPhase::Review,
    }
}

fn template_matches_role(template: &super::config::TeamTemplateConfig, role: &str) -> bool {
    let normalized_role = role.trim().to_ascii_lowercase();
    if normalized_role.is_empty() {
        return false;
    }
    template.id.trim().eq_ignore_ascii_case(&normalized_role)
        || template
            .leader_role
            .as_deref()
            .map(str::trim)
            .is_some_and(|leader_role| leader_role.eq_ignore_ascii_case(&normalized_role))
}

fn synthesize_internal_skills(entries: &[TeamAuditEntry]) -> Vec<String> {
    let correction_count = entries
        .iter()
        .filter(|entry| entry.detected_instruction_drift)
        .count();
    let peer_sync_count = entries
        .iter()
        .filter(|entry| entry.kind == TeamAuditKind::PeerSync)
        .count();
    let handoff_count = entries
        .iter()
        .filter(|entry| entry.kind == TeamAuditKind::ArtifactHandoff)
        .count();

    let mut skills = Vec::new();
    if correction_count > 0 {
        skills.push(
            "Restate scope boundaries and delivery rules before delegating when prior work drifted."
                .to_string(),
        );
    }
    if peer_sync_count > 0 {
        skills.push(
            "Capture interface assumptions and blockers in sibling peer syncs instead of relying on memory."
                .to_string(),
        );
    }
    if handoff_count > 0 {
        skills.push(
            "Package cross-level work as explicit artifact bundles with status and governance references."
                .to_string(),
        );
    }
    skills
}

async fn update_parent_team_links(
    workspace_root: &Path,
    parent_team_id: &str,
    child_team_id: &str,
    child_thread_id: ThreadId,
    child_role: &str,
) -> io::Result<()> {
    let Some(mut parent) = load_team_state_bundle(workspace_root, parent_team_id).await? else {
        return Ok(());
    };
    if !parent
        .record
        .child_team_ids
        .iter()
        .any(|team_id| team_id == child_team_id)
    {
        parent.record.child_team_ids.push(child_team_id.to_string());
        parent.record.child_team_ids.sort();
    }
    parent.status.active_child_teams = parent.record.child_team_ids.clone();
    let now = Utc::now().to_rfc3339();
    apply_role_assignment(
        &mut parent.status.cycle,
        child_team_id,
        child_thread_id,
        child_role,
        &now,
    );
    parent.status.current_phase = parent.status.cycle.phase.clone();
    parent.status.updated_at = now.clone();
    parent.recovery.active_child_teams = parent.record.child_team_ids.clone();
    parent.recovery.updated_at = now.clone();
    parent.audit.entries.push(TeamAuditEntry {
        kind: TeamAuditKind::Delegation,
        counterpart_team_id: Some(child_team_id.to_string()),
        counterpart_thread_id: None,
        summary: format!("Registered child team `{child_team_id}` with role `{child_role}`."),
        artifact_refs: Vec::new(),
        detected_instruction_drift: false,
        created_at: now.clone(),
    });
    parent.audit.synthesized_skills = synthesize_internal_skills(&parent.audit.entries);
    parent.audit.updated_at = now;
    write_team_state_bundle(&parent).await
}

fn missing_state_file(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("missing team state file {}", path.display()),
    )
}
