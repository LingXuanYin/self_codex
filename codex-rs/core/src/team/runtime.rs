use super::config::{
    ExecutionMode, GLOBAL_AGENT_DOC_FILENAME, GovernanceTrigger, IterationRole,
    TEAM_AGENT_DOC_FILENAME, TEAM_DIRNAME, TEAM_STATE_DIRNAME, TeamWorkflowConfig,
    load_workflow_from_workspace, resolve_team_home_root, workflow_path,
};
use super::redaction::{
    public_team_ref, public_worktree_label, sanitize_summary_text, sanitize_user_input_summary,
    sanitize_workspace_path, vertical_receiver_label,
};
use super::state::{
    TeamAuditEntry, TeamAuditKind, TeamEnvironmentState, TeamIntegrationHandoff,
    TeamIntegrationMode, TeamKind, TeamManagedResource, TeamManagedResourceKind,
    TeamManagedResourceStatus, TeamPhase, TeamStateBundle, TeamStatePaths, TeamStateRecord,
    TeamStateWriteRequest, TeamTapeEntry, TeamTapeKind, TeamWorktreeState, append_team_tape_entry,
    apply_role_assignment, indicates_replan, load_team_state_bundle, load_team_worktree,
    mark_cycle_artifact_handoff, mark_cycle_replan, persist_team_state,
    update_team_environment_state, update_team_worktree, write_team_state_bundle,
};
use crate::git_info::current_branch_name;
use crate::git_info::get_head_commit_hash;
use crate::git_info::resolve_root_git_project_for_trust;
use chrono::Utc;
use codex_protocol::ThreadId;
use codex_protocol::protocol::{SessionSource, SubAgentSource};
use codex_protocol::user_input::UserInput;
use std::{
    borrow::Cow,
    io,
    path::{Path, PathBuf},
};
use tokio::fs;
use tokio::process::Command;

const GENERATED_SECTION_START: &str = "<!-- codex-team-runtime:start -->";
const GENERATED_SECTION_END: &str = "<!-- codex-team-runtime:end -->";
const GOVERNANCE_PROMPTS_DIR: &str = "team-governance/prompts";
const TEAM_OPS_DIRNAME: &str = "team-ops";

const TEAM_SKILL_DELEGATION: &str = "team-delegation";
const TEAM_SKILL_REVIEW_LOOP: &str = "team-review-return-loop";
const TEAM_SKILL_COMPACT: &str = "team-compact-continuation";
const TEAM_SKILL_GOVERNANCE: &str = "team-governance-updates";
const TEAM_SKILL_HANDOFF: &str = "team-sanitized-handoff";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TeamRelationship {
    None,
    SameTeam,
    Sibling,
    Vertical,
    SeparateBoundary,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedTeamMessage {
    pub relationship: TeamRelationship,
    pub items: Vec<UserInput>,
    pub summary: String,
    pub artifact_refs: Vec<PathBuf>,
    pub integration_handoff: Option<TeamIntegrationHandoff>,
}

pub(crate) async fn maybe_initialize_for_thread(
    workspace_root: &Path,
    thread_id: ThreadId,
    session_source: &SessionSource,
    rollout_path: Option<&Path>,
) -> io::Result<()> {
    let team_home_root = resolve_team_home_root(workspace_root);
    let Some(workflow) = load_workflow_from_workspace(&team_home_root).await? else {
        return Ok(());
    };
    let Some(scope) = TeamSessionScope::from_session_source(session_source) else {
        return Ok(());
    };
    let (paths, record) = persist_team_state(TeamStateWriteRequest {
        team_id: thread_id.to_string(),
        thread_id,
        parent_thread_id: scope.parent_thread_id(),
        depth: scope.depth(),
        kind: scope.team_kind(),
        role: scope.role(&workflow),
        nickname: scope.nickname(),
        workspace_root: team_home_root.clone(),
        workflow_path: workflow_path(&team_home_root),
        rollout_path: rollout_path.map(Path::to_path_buf),
        workflow: workflow.clone(),
    })
    .await?;
    let initial_bundle = load_team_state_bundle(&team_home_root, &record.team_id)
        .await?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "team bundle missing"))?;
    record_bootstrap_tape_entry(&initial_bundle).await?;
    ensure_team_worktree_assignment(&team_home_root, &record.team_id, workspace_root).await?;
    let bundle = load_team_state_bundle(&team_home_root, &record.team_id)
        .await?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "team bundle missing"))?;
    ensure_governance_assets(&team_home_root, &workflow).await?;
    if matches!(scope, TeamSessionScope::Root) {
        ensure_global_agent_doc(&paths, &bundle, &workflow).await?;
    }
    ensure_team_agent_doc(&paths, &bundle, &workflow).await?;
    sync_operator_surface(&bundle).await?;
    if let Some(parent_team_id) = record.parent_team_id.as_deref() {
        refresh_team_documents(&team_home_root, parent_team_id).await?;
    }
    Ok(())
}

pub(crate) async fn prepare_child_team_spawn(
    workspace_root: &Path,
    parent_thread_id: ThreadId,
    items: Vec<UserInput>,
) -> io::Result<PreparedTeamMessage> {
    prepare_vertical_handoff(
        workspace_root,
        &parent_thread_id.to_string(),
        None,
        items,
        "spawn",
    )
    .await
}

pub(crate) async fn prepare_team_message(
    workspace_root: &Path,
    sender_thread_id: ThreadId,
    receiver_thread_id: ThreadId,
    items: Vec<UserInput>,
) -> io::Result<PreparedTeamMessage> {
    let Some(sender) =
        load_team_state_bundle(workspace_root, &sender_thread_id.to_string()).await?
    else {
        return Ok(PreparedTeamMessage {
            relationship: TeamRelationship::None,
            summary: summarize_input(&items),
            items,
            artifact_refs: Vec::new(),
            integration_handoff: None,
        });
    };
    let Some(receiver) =
        load_team_state_bundle(workspace_root, &receiver_thread_id.to_string()).await?
    else {
        return Ok(PreparedTeamMessage {
            relationship: TeamRelationship::None,
            summary: summarize_input(&items),
            items,
            artifact_refs: Vec::new(),
            integration_handoff: None,
        });
    };
    match determine_relationship(&sender.record, &receiver.record) {
        TeamRelationship::Sibling | TeamRelationship::SameTeam | TeamRelationship::None => {
            Ok(PreparedTeamMessage {
                relationship: determine_relationship(&sender.record, &receiver.record),
                summary: summarize_input(&items),
                items,
                artifact_refs: Vec::new(),
                integration_handoff: None,
            })
        }
        TeamRelationship::Vertical | TeamRelationship::SeparateBoundary => {
            prepare_vertical_handoff(
                workspace_root,
                &sender.record.team_id,
                Some(&receiver.record.team_id),
                items,
                "handoff",
            )
            .await
        }
    }
}

pub(crate) async fn record_child_team_spawn(
    workspace_root: &Path,
    parent_thread_id: ThreadId,
    child_thread_id: ThreadId,
    child_role: Option<&str>,
    prepared: &PreparedTeamMessage,
) -> io::Result<()> {
    let Some(mut parent) =
        load_team_state_bundle(workspace_root, &parent_thread_id.to_string()).await?
    else {
        return Ok(());
    };
    let Some(mut child) =
        load_team_state_bundle(workspace_root, &child_thread_id.to_string()).await?
    else {
        return Ok(());
    };
    let now = Utc::now().to_rfc3339();
    parent.audit.entries.push(TeamAuditEntry {
        kind: TeamAuditKind::Delegation,
        counterpart_team_id: Some(child.record.team_id.clone()),
        counterpart_thread_id: Some(child_thread_id),
        summary: prepared.summary.clone(),
        artifact_refs: prepared.artifact_refs.clone(),
        detected_instruction_drift: false,
        created_at: now.clone(),
    });
    parent.audit.synthesized_skills = synthesize_skills(&parent.audit.entries);
    parent.audit.updated_at = now.clone();
    apply_role_assignment(
        &mut parent.status.cycle,
        &child.record.team_id,
        child_thread_id,
        child_role.unwrap_or(child.record.role.as_str()),
        &now,
    );
    parent.status.current_phase = parent.status.cycle.phase.clone();
    parent.status.updated_at = now.clone();
    parent.handoff.produced_artifacts = merge_artifacts(
        &parent.handoff.produced_artifacts,
        &prepared.artifact_refs,
        &parent.record.workspace_root,
    );
    if prepared.integration_handoff.is_some() {
        parent.handoff.integration = prepared.integration_handoff.clone();
    }
    parent.handoff.updated_at = now.clone();
    child.handoff.produced_artifacts = merge_artifacts(
        &child.handoff.produced_artifacts,
        &prepared.artifact_refs,
        &child.record.workspace_root,
    );
    if prepared.integration_handoff.is_some() {
        child.handoff.integration = prepared.integration_handoff.clone();
    }
    child.handoff.updated_at = now.clone();
    write_team_state_bundle(&parent).await?;
    write_team_state_bundle(&child).await?;
    sync_operator_surface(&parent).await?;
    sync_operator_surface(&child).await?;
    append_team_tape_entry(
        &parent.record.workspace_root,
        &parent.record.team_id,
        TeamTapeKind::Delegation,
        prepared.summary.clone(),
        Some(child.record.team_id.clone()),
        prepared.artifact_refs.clone(),
        Some("delegation".to_string()),
    )
    .await?;
    if prepared.integration_handoff.is_some() {
        append_team_tape_entry(
            &child.record.workspace_root,
            &child.record.team_id,
            TeamTapeKind::IntegrationReady,
            "Child team bootstrap produced a reviewable integration contract.".to_string(),
            Some(parent.record.team_id.clone()),
            prepared.artifact_refs.clone(),
            Some("integration-ready".to_string()),
        )
        .await?;
    }
    refresh_team_documents(workspace_root, &parent.record.team_id).await?;
    refresh_team_documents(workspace_root, &child.record.team_id).await
}

pub(crate) async fn record_team_message_delivery(
    workspace_root: &Path,
    sender_thread_id: ThreadId,
    receiver_thread_id: ThreadId,
    prepared: &PreparedTeamMessage,
) -> io::Result<()> {
    let Some(mut sender) =
        load_team_state_bundle(workspace_root, &sender_thread_id.to_string()).await?
    else {
        return Ok(());
    };
    let Some(mut receiver) =
        load_team_state_bundle(workspace_root, &receiver_thread_id.to_string()).await?
    else {
        return Ok(());
    };
    let now = Utc::now().to_rfc3339();
    let drift = indicates_replan(&prepared.summary);
    let kind = if drift {
        TeamAuditKind::Correction
    } else if matches!(
        prepared.relationship,
        TeamRelationship::Sibling | TeamRelationship::SameTeam
    ) {
        TeamAuditKind::PeerSync
    } else {
        TeamAuditKind::ArtifactHandoff
    };
    {
        let other = &receiver.record;
        let bundle = &mut sender;
        bundle.audit.entries.push(TeamAuditEntry {
            kind: kind.clone(),
            counterpart_team_id: Some(other.team_id.clone()),
            counterpart_thread_id: Some(other.thread_id),
            summary: prepared.summary.clone(),
            artifact_refs: prepared.artifact_refs.clone(),
            detected_instruction_drift: drift,
            created_at: now.clone(),
        });
        bundle.audit.synthesized_skills = synthesize_skills(&bundle.audit.entries);
        bundle.audit.updated_at = now.clone();
        if matches!(
            prepared.relationship,
            TeamRelationship::Sibling | TeamRelationship::SameTeam
        ) {
            apply_role_assignment(
                &mut bundle.status.cycle,
                &other.team_id,
                other.thread_id,
                other.role.as_str(),
                &now,
            );
        } else if drift && bundle.record.role.to_ascii_lowercase().contains("review") {
            mark_cycle_replan(&mut bundle.status.cycle, prepared.summary.clone(), &now);
            if !bundle
                .status
                .blockers
                .iter()
                .any(|entry| entry == &prepared.summary)
            {
                bundle.status.blockers.push(prepared.summary.clone());
            }
        } else {
            mark_cycle_artifact_handoff(
                &mut bundle.status.cycle,
                Some(bundle.record.role.as_str()),
                &prepared.summary,
                &now,
            );
        }
        bundle.status.current_phase = bundle.status.cycle.phase.clone();
        bundle.status.updated_at = now.clone();
        bundle.handoff.produced_artifacts = merge_artifacts(
            &bundle.handoff.produced_artifacts,
            &prepared.artifact_refs,
            &bundle.record.workspace_root,
        );
        if prepared.integration_handoff.is_some() {
            bundle.handoff.integration = prepared.integration_handoff.clone();
        }
        bundle.handoff.summary = prepared.summary.clone();
        bundle.handoff.updated_at = now.clone();
        bundle.recovery.blockers = bundle.status.blockers.clone();
        bundle.recovery.next_steps = bundle.status.next_steps.clone();
        bundle.recovery.updated_at = now.clone();
    }
    {
        let other = &sender.record;
        let bundle = &mut receiver;
        bundle.audit.entries.push(TeamAuditEntry {
            kind,
            counterpart_team_id: Some(other.team_id.clone()),
            counterpart_thread_id: Some(other.thread_id),
            summary: prepared.summary.clone(),
            artifact_refs: prepared.artifact_refs.clone(),
            detected_instruction_drift: drift,
            created_at: now.clone(),
        });
        bundle.audit.synthesized_skills = synthesize_skills(&bundle.audit.entries);
        bundle.audit.updated_at = now.clone();
        if matches!(
            prepared.relationship,
            TeamRelationship::Sibling | TeamRelationship::SameTeam
        ) {
            apply_role_assignment(
                &mut bundle.status.cycle,
                &other.team_id,
                other.thread_id,
                other.role.as_str(),
                &now,
            );
        } else if drift && bundle.record.role.to_ascii_lowercase().contains("review") {
            mark_cycle_replan(&mut bundle.status.cycle, prepared.summary.clone(), &now);
            if !bundle
                .status
                .blockers
                .iter()
                .any(|entry| entry == &prepared.summary)
            {
                bundle.status.blockers.push(prepared.summary.clone());
            }
        } else {
            mark_cycle_artifact_handoff(
                &mut bundle.status.cycle,
                Some(bundle.record.role.as_str()),
                &prepared.summary,
                &now,
            );
        }
        bundle.status.current_phase = bundle.status.cycle.phase.clone();
        bundle.status.updated_at = now.clone();
        bundle.handoff.produced_artifacts = merge_artifacts(
            &bundle.handoff.produced_artifacts,
            &prepared.artifact_refs,
            &bundle.record.workspace_root,
        );
        if prepared.integration_handoff.is_some() {
            bundle.handoff.integration = prepared.integration_handoff.clone();
        }
        bundle.handoff.summary = prepared.summary.clone();
        bundle.handoff.updated_at = now.clone();
        bundle.recovery.blockers = bundle.status.blockers.clone();
        bundle.recovery.next_steps = bundle.status.next_steps.clone();
        bundle.recovery.updated_at = now.clone();
    }
    write_team_state_bundle(&sender).await?;
    write_team_state_bundle(&receiver).await?;
    sync_operator_surface(&sender).await?;
    sync_operator_surface(&receiver).await?;
    let tape_kind = if matches!(
        prepared.relationship,
        TeamRelationship::Sibling | TeamRelationship::SameTeam
    ) {
        TeamTapeKind::PeerSync
    } else if prepared.integration_handoff.is_some() {
        TeamTapeKind::IntegrationReady
    } else {
        TeamTapeKind::ArtifactHandoff
    };
    append_team_tape_entry(
        &sender.record.workspace_root,
        &sender.record.team_id,
        tape_kind,
        prepared.summary.clone(),
        Some(receiver.record.team_id.clone()),
        prepared.artifact_refs.clone(),
        Some("delivery".to_string()),
    )
    .await?;
    append_team_tape_entry(
        &receiver.record.workspace_root,
        &receiver.record.team_id,
        tape_kind,
        prepared.summary.clone(),
        Some(sender.record.team_id.clone()),
        prepared.artifact_refs.clone(),
        Some("delivery".to_string()),
    )
    .await?;
    refresh_team_documents(workspace_root, &sender.record.team_id).await?;
    refresh_team_documents(workspace_root, &receiver.record.team_id).await
}

pub(crate) async fn record_team_resume(workspace_root: &Path, team_id: &str) -> io::Result<()> {
    let Some(mut bundle) = load_team_state_bundle(workspace_root, team_id).await? else {
        return Ok(());
    };
    let now = Utc::now().to_rfc3339();
    bundle.recovery.last_resumed_at = Some(now.clone());
    bundle.recovery.updated_at = now.clone();
    bundle.audit.entries.push(TeamAuditEntry {
        kind: TeamAuditKind::Resume,
        counterpart_team_id: None,
        counterpart_thread_id: Some(bundle.record.thread_id),
        summary: "Leader resumed from persisted artifacts.".to_string(),
        artifact_refs: vec![
            bundle.paths.status_path.clone(),
            bundle.paths.handoff_path.clone(),
            bundle.paths.team_doc_path.clone(),
        ],
        detected_instruction_drift: false,
        created_at: now.clone(),
    });
    bundle.audit.synthesized_skills = synthesize_skills(&bundle.audit.entries);
    bundle.audit.updated_at = now;
    write_team_state_bundle(&bundle).await?;
    sync_operator_surface(&bundle).await?;
    append_team_tape_entry(
        &bundle.record.workspace_root,
        &bundle.record.team_id,
        TeamTapeKind::Resume,
        "Leader resumed from persisted artifacts.".to_string(),
        None,
        vec![
            bundle.paths.status_path.clone(),
            bundle.paths.handoff_path.clone(),
            bundle.paths.team_doc_path.clone(),
        ],
        Some("resume".to_string()),
    )
    .await?;
    refresh_team_documents(workspace_root, team_id).await
}

pub(crate) async fn refresh_team_documents(workspace_root: &Path, team_id: &str) -> io::Result<()> {
    let Some(workflow) = load_workflow_from_workspace(workspace_root).await? else {
        return Ok(());
    };
    let Some(bundle) = load_team_state_bundle(workspace_root, team_id).await? else {
        return Ok(());
    };
    ensure_governance_assets(workspace_root, &workflow).await?;
    if bundle.record.kind == TeamKind::Root {
        ensure_global_agent_doc(&bundle.paths, &bundle, &workflow).await?;
    }
    ensure_team_agent_doc(&bundle.paths, &bundle, &workflow).await?;
    sync_operator_surface(&bundle).await
}

async fn record_bootstrap_tape_entry(bundle: &TeamStateBundle) -> io::Result<()> {
    if fs::metadata(&bundle.paths.tape_path)
        .await
        .map(|metadata| metadata.len() > 0)
        .unwrap_or(false)
    {
        return Ok(());
    }
    let summary = format!(
        "Initialized team at depth {} with role `{}`.",
        bundle.record.depth, bundle.record.role
    );
    append_team_tape_entry(
        &bundle.record.workspace_root,
        &bundle.record.team_id,
        TeamTapeKind::Bootstrap,
        summary,
        bundle.record.parent_team_id.clone(),
        vec![
            bundle.paths.status_path.clone(),
            bundle.paths.handoff_path.clone(),
            bundle.paths.team_doc_path.clone(),
        ],
        Some("bootstrap".to_string()),
    )
    .await?;
    Ok(())
}

async fn record_worktree_tape_entry(bundle: &TeamStateBundle) -> io::Result<()> {
    let Some(worktree) = bundle.record.worktree.as_ref() else {
        return Ok(());
    };
    let summary = format!(
        "Assigned checkout `{}` with branch namespace `{}`.",
        public_worktree_label(
            &public_team_ref(
                &bundle.record.team_id,
                &bundle.record.role,
                bundle.record.depth,
                bundle.record.kind.clone(),
            ),
            worktree.managed
        )
        .display(),
        worktree.branch_name
    );
    append_team_tape_entry(
        &bundle.record.workspace_root,
        &bundle.record.team_id,
        TeamTapeKind::WorktreeAssigned,
        summary,
        bundle.record.parent_team_id.clone(),
        vec![
            bundle.paths.status_path.clone(),
            bundle.paths.team_doc_path.clone(),
        ],
        Some("worktree".to_string()),
    )
    .await?;
    Ok(())
}

pub(crate) async fn assigned_team_cwd(
    workspace_root: &Path,
    team_id: &str,
) -> io::Result<Option<PathBuf>> {
    Ok(load_team_worktree(workspace_root, team_id)
        .await?
        .map(|worktree| worktree.checkout_path))
}

async fn sync_team_environment_state(
    workspace_root: &Path,
    bundle: &TeamStateBundle,
) -> io::Result<Option<TeamStateBundle>> {
    let Some(worktree) = bundle.record.worktree.as_ref() else {
        return Ok(Some(bundle.clone()));
    };
    let worktree_exists = fs::try_exists(&worktree.checkout_path)
        .await
        .unwrap_or(false);
    let now = Utc::now().to_rfc3339();
    let resource = TeamManagedResource {
        resource_id: worktree.branch_name.clone(),
        kind: TeamManagedResourceKind::Worktree,
        path: Some(worktree.checkout_path.clone()),
        status: if worktree_exists {
            TeamManagedResourceStatus::Active
        } else {
            TeamManagedResourceStatus::Stale
        },
        cleanup_required: worktree.managed,
        last_verified_at: now.clone(),
    };
    let mut environment = TeamEnvironmentState {
        managed_resources: vec![resource.clone()],
        stale_resources: Vec::new(),
        cleanup_notes: Vec::new(),
        last_cleanup_at: None,
    };
    if worktree.managed {
        environment.cleanup_notes.push(
            "Leader must remove or repurpose the linked worktree after acceptance or supersession."
                .to_string(),
        );
    }
    if !worktree_exists {
        environment.stale_resources.push(resource);
        environment.cleanup_notes.push(
            "Managed worktree path is missing; cleanup and reconciliation are required."
                .to_string(),
        );
    }
    update_team_environment_state(workspace_root, &bundle.record.team_id, environment).await
}

async fn ensure_team_worktree_assignment(
    team_home_root: &Path,
    team_id: &str,
    session_cwd: &Path,
) -> io::Result<()> {
    let Some(bundle) = load_team_state_bundle(team_home_root, team_id).await? else {
        return Ok(());
    };

    let worktree = match bundle.record.kind {
        TeamKind::Root => capture_root_worktree(&bundle, session_cwd).await?,
        TeamKind::Child => allocate_child_worktree(&bundle, session_cwd).await?,
    };
    let Some(bundle) = update_team_worktree(team_home_root, team_id, worktree).await? else {
        return Ok(());
    };
    let Some(bundle) = sync_team_environment_state(team_home_root, &bundle).await? else {
        return Ok(());
    };
    record_worktree_tape_entry(&bundle).await?;
    if bundle.record.kind == TeamKind::Root {
        refresh_team_documents(team_home_root, team_id).await?;
    }
    Ok(())
}

async fn capture_root_worktree(
    bundle: &TeamStateBundle,
    session_cwd: &Path,
) -> io::Result<TeamWorktreeState> {
    let root_team_id =
        resolve_root_team_id(&bundle.record.workspace_root, &bundle.record.team_id).await?;
    let branch_name = planned_branch_name(&root_team_id, &bundle.record);
    let checkout_path = session_cwd.to_path_buf();
    let current_branch = current_branch_name(&checkout_path).await;
    let head_commit = get_head_commit_hash(&checkout_path).await;
    let repo_root = resolve_root_git_project_for_trust(&checkout_path);

    Ok(TeamWorktreeState {
        branch_name,
        current_branch,
        checkout_path,
        source_checkout_path: None,
        repo_root,
        base_commit: head_commit.clone(),
        head_commit,
        managed: false,
        updated_at: Utc::now().to_rfc3339(),
    })
}

async fn allocate_child_worktree(
    bundle: &TeamStateBundle,
    session_cwd: &Path,
) -> io::Result<TeamWorktreeState> {
    let root_team_id =
        resolve_root_team_id(&bundle.record.workspace_root, &bundle.record.team_id).await?;
    let branch_name = planned_branch_name(&root_team_id, &bundle.record);
    let checkout_path = bundle.paths.team_dir.join("worktree");

    if bundle.record.worktree.as_ref().is_some_and(|worktree| {
        worktree.checkout_path == checkout_path && checkout_path.join(".git").exists()
    }) {
        return capture_existing_worktree(bundle.record.worktree.as_ref(), &branch_name).await;
    }

    let parent_checkout_path = parent_checkout_path(bundle)
        .await?
        .unwrap_or_else(|| session_cwd.to_path_buf());
    let repo_root = resolve_root_git_project_for_trust(&parent_checkout_path);
    let base_commit = get_head_commit_hash(&parent_checkout_path).await;

    if repo_root.is_some() && !checkout_path.join(".git").exists() {
        create_git_worktree(
            &parent_checkout_path,
            &checkout_path,
            &branch_name,
            base_commit.as_deref(),
        )
        .await?;
    }

    let current_branch = current_branch_name(&checkout_path).await;
    let head_commit = get_head_commit_hash(&checkout_path)
        .await
        .or_else(|| base_commit.clone());

    Ok(TeamWorktreeState {
        branch_name,
        current_branch,
        checkout_path,
        source_checkout_path: Some(parent_checkout_path),
        repo_root,
        base_commit,
        head_commit,
        managed: true,
        updated_at: Utc::now().to_rfc3339(),
    })
}

async fn capture_existing_worktree(
    existing: Option<&TeamWorktreeState>,
    branch_name: &str,
) -> io::Result<TeamWorktreeState> {
    let existing = existing.cloned().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "team worktree metadata missing for existing checkout",
        )
    })?;
    let current_branch = current_branch_name(&existing.checkout_path).await;
    let head_commit = get_head_commit_hash(&existing.checkout_path).await;
    Ok(TeamWorktreeState {
        branch_name: branch_name.to_string(),
        current_branch,
        head_commit,
        updated_at: Utc::now().to_rfc3339(),
        ..existing
    })
}

async fn parent_checkout_path(bundle: &TeamStateBundle) -> io::Result<Option<PathBuf>> {
    let Some(parent_team_id) = bundle.record.parent_team_id.as_deref() else {
        return Ok(None);
    };
    let Some(parent) =
        load_team_state_bundle(&bundle.record.workspace_root, parent_team_id).await?
    else {
        return Ok(None);
    };
    Ok(parent
        .record
        .worktree
        .as_ref()
        .map(|worktree| worktree.checkout_path.clone()))
}

async fn resolve_root_team_id(workspace_root: &Path, team_id: &str) -> io::Result<String> {
    let mut current_id = team_id.to_string();
    loop {
        let Some(bundle) = load_team_state_bundle(workspace_root, &current_id).await? else {
            return Ok(current_id);
        };
        let Some(parent_id) = bundle.record.parent_team_id else {
            return Ok(bundle.record.team_id);
        };
        current_id = parent_id;
    }
}

fn planned_branch_name(root_team_id: &str, record: &TeamStateRecord) -> String {
    match record.kind {
        TeamKind::Root => format!("team/{root_team_id}/root"),
        TeamKind::Child => format!("team/{root_team_id}/{}", record.team_id),
    }
}

async fn create_git_worktree(
    source_cwd: &Path,
    checkout_path: &Path,
    branch_name: &str,
    base_commit: Option<&str>,
) -> io::Result<()> {
    if let Some(parent) = checkout_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    if checkout_path.exists() && !checkout_path.join(".git").exists() {
        let mut entries = fs::read_dir(checkout_path).await?;
        if entries.next_entry().await?.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "managed worktree path already exists and is not a git checkout: {}",
                    checkout_path.display()
                ),
            ));
        }
        fs::remove_dir_all(checkout_path).await?;
    }

    let mut args = vec![
        "worktree".to_string(),
        "add".to_string(),
        "-b".to_string(),
        branch_name.to_string(),
        checkout_path.display().to_string(),
    ];
    if let Some(base_commit) = base_commit {
        args.push(base_commit.to_string());
    }
    if run_git(source_cwd, &args).await.is_err() {
        run_git(
            source_cwd,
            &[
                "worktree".to_string(),
                "add".to_string(),
                checkout_path.display().to_string(),
                branch_name.to_string(),
            ],
        )
        .await?;
    }
    Ok(())
}

async fn run_git(cwd: &Path, args: &[String]) -> io::Result<()> {
    let output = Command::new("git")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(args)
        .current_dir(cwd)
        .output()
        .await?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(io::Error::other(format!(
        "git {} failed: {}",
        args.join(" "),
        stderr.trim()
    )))
}

#[derive(Debug, Clone)]
enum TeamSessionScope {
    Root,
    Child {
        parent_thread_id: ThreadId,
        depth: i32,
        agent_role: Option<String>,
        agent_nickname: Option<String>,
    },
}

impl TeamSessionScope {
    fn from_session_source(session_source: &SessionSource) -> Option<Self> {
        match session_source {
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth,
                agent_role,
                agent_nickname,
            }) => Some(Self::Child {
                parent_thread_id: *parent_thread_id,
                depth: *depth,
                agent_role: agent_role.clone(),
                agent_nickname: agent_nickname.clone(),
            }),
            SessionSource::SubAgent(_) => None,
            SessionSource::Cli
            | SessionSource::VSCode
            | SessionSource::Exec
            | SessionSource::Mcp
            | SessionSource::Unknown => Some(Self::Root),
        }
    }
    fn parent_thread_id(&self) -> Option<ThreadId> {
        match self {
            Self::Root => None,
            Self::Child {
                parent_thread_id, ..
            } => Some(*parent_thread_id),
        }
    }
    fn depth(&self) -> i32 {
        match self {
            Self::Root => 0,
            Self::Child { depth, .. } => *depth,
        }
    }
    fn nickname(&self) -> Option<String> {
        match self {
            Self::Root => None,
            Self::Child { agent_nickname, .. } => agent_nickname.clone(),
        }
    }
    fn team_kind(&self) -> TeamKind {
        match self {
            Self::Root => TeamKind::Root,
            Self::Child { .. } => TeamKind::Child,
        }
    }
    fn role(&self, workflow: &TeamWorkflowConfig) -> String {
        match self {
            Self::Root => workflow.root_scheduler.role.clone(),
            Self::Child { agent_role, .. } => agent_role
                .as_deref()
                .map(str::trim)
                .filter(|role| !role.is_empty())
                .unwrap_or("team-leader")
                .to_string(),
        }
    }
}

async fn ensure_governance_assets(
    workspace_root: &Path,
    workflow: &TeamWorkflowConfig,
) -> io::Result<()> {
    let codex_root = resolve_team_home_root(workspace_root).join(".codex");
    for (relative_path, contents) in governance_prompt_assets() {
        let path = codex_root.join(relative_path);
        let generated = wrap_generated_asset(&contents);
        upsert_generated_markdown(&path, generated.clone(), generated).await?;
    }
    for (skill_name, contents) in governance_skill_assets(workflow) {
        let path = codex_root.join("skills").join(skill_name).join("SKILL.md");
        let generated = wrap_generated_asset(&contents);
        upsert_generated_markdown(&path, generated.clone(), generated).await?;
    }
    Ok(())
}

fn governance_prompt_assets() -> Vec<(&'static str, String)> {
    vec![
        (
            "team-governance/prompts/scheduler.md",
            r#"# Scheduler Decision Model

- Own the team charter, root plan, and final user interaction surface.
- Maintain the triad loop: design, development, review must all participate in each substantive cycle.
- Choose direct execution only for blocking work that is cheaper to finish locally than to delegate.
- Delegate bounded work with explicit ownership, expected artifacts, and recovery checkpoints.
- Escalate or replan when review returns drift, boundary uncertainty, or missing architecture.
- Keep `AGENT.md` current when global governance, escalation rules, or delivery policy changes.
"#
            .to_string(),
        ),
        (
            "team-governance/prompts/leader.md",
            r#"# Team Leader Decision Model

- Translate parent intent into bounded team work without leaking hidden context upward.
- Preserve version discipline: maintain branch/worktree awareness, checkpoint progress, and clean stale resources.
- Use sibling syncs for alignment and vertical handoffs for artifacts only.
- Update `AGENT_TEAM.md` before the next delegation round whenever review changes local rules or reusable skills.
- Summarize blockers, next actions, and declared outputs instead of forwarding raw transcripts.
"#
            .to_string(),
        ),
        (
            "team-governance/prompts/worker.md",
            r#"# Worker Decision Model

- Execute the assigned bounded task and keep changes inside the agreed ownership boundary.
- Persist progress in artifacts and status files before compacting or yielding.
- Raise blockers early with concrete evidence, not speculative context dumps.
- Hand work back through sanitized artifact bundles that declare outputs, blockers, and next action.
"#
            .to_string(),
        ),
        (
            "team-governance/prompts/designer.md",
            r#"# Designer Decision Model

- Define system boundaries, interfaces, constraints, and module responsibilities before implementation fan-out.
- Make dependencies, integration contracts, and review checkpoints explicit.
- Re-open design when implementation or review reveals drift, missing boundaries, or coordination risk.
"#
            .to_string(),
        ),
        (
            "team-governance/prompts/developer.md",
            r#"# Developer Decision Model

- Implement the planned slice with minimal, reviewable changes and explicit validation.
- Keep the assigned environment healthy, clean stale resources, and surface integration risks early.
- Deliver code plus artifacts that a reviewer or parent can consume without hidden transcript context.
"#
            .to_string(),
        ),
        (
            "team-governance/prompts/reviewer.md",
            r#"# Reviewer Decision Model

- Review against requirements, architecture boundaries, regression risk, and process adherence.
- Return work to design or development when scope drift, unsafe exposure, or weak validation is found.
- Capture reusable team skills when recurring issues or strong patterns emerge.
"#
            .to_string(),
        ),
    ]
}

fn governance_skill_assets(workflow: &TeamWorkflowConfig) -> Vec<(&'static str, String)> {
    let roles = workflow
        .workflow_loop
        .required_roles
        .iter()
        .map(|role| format!("`{}`", format_iteration_role(*role)))
        .collect::<Vec<_>>()
        .join(", ");
    vec![
        (
            TEAM_SKILL_DELEGATION,
            r#"---
name: team-delegation
description: Delegate bounded work with explicit ownership, artifact requirements, and return conditions.
---

- Delegate only when the worker can complete the slice without hidden parent context.
- Include scope, owner, expected artifacts, validation target, and compact checkpoint in every delegation.
- Tell workers to preserve others' edits and avoid reverting unrelated work.
"#
            .to_string(),
        ),
        (
            TEAM_SKILL_REVIEW_LOOP,
            format!(
                r#"---
name: team-review-return-loop
description: Run the design-development-review loop with explicit return-to-design or return-to-development conditions.
---

- Every substantive cycle must cover {}.
- When review finds drift, boundary mismatch, or missing validation, return work with concrete findings and update `AGENT_TEAM.md` before the next delegation round.
- Do not bypass review just because the branch builds.
"#
                ,
                roles
            ),
        ),
        (
            TEAM_SKILL_COMPACT,
            r#"---
name: team-compact-continuation
description: Preserve team progress across compaction or interruption.
---

- Before compact, persist blockers, next steps, artifacts, and governance updates.
- Resume from `status.json`, `handoff.json`, `team-tape.jsonl`, `AGENT.md`, and `AGENT_TEAM.md` instead of hidden transcript memory.
"#
            .to_string(),
        ),
        (
            TEAM_SKILL_GOVERNANCE,
            r#"---
name: team-governance-updates
description: Update global and team governance documents when process rules change.
---

- The root scheduler owns `AGENT.md`.
- Each team leader owns `AGENT_TEAM.md` for local scope, retrospectives, reusable skills, and recovery notes.
- Update governance docs on team creation, replan, review handoff, compact, or leader resume.
"#
            .to_string(),
        ),
        (
            TEAM_SKILL_HANDOFF,
            r#"---
name: team-sanitized-handoff
description: Author vertical handoffs using structured summaries and safe artifact references only.
---

- Cross-level handoffs may include summaries, blockers, next actions, governance deltas, and safe artifact references.
- Do not include raw transcript dumps, unsafe absolute paths, or hidden child identifiers in vertical artifacts.
"#
            .to_string(),
        ),
    ]
}

fn role_prompt_relative_path(record: &TeamStateRecord) -> &'static str {
    if record.kind == TeamKind::Root {
        return "team-governance/prompts/scheduler.md";
    }
    let normalized = record.role.to_ascii_lowercase();
    if normalized.contains("review") {
        "team-governance/prompts/reviewer.md"
    } else if normalized.contains("design") || normalized.contains("architect") {
        "team-governance/prompts/designer.md"
    } else if normalized.contains("dev")
        || normalized.contains("implement")
        || normalized.contains("build")
    {
        "team-governance/prompts/developer.md"
    } else if normalized.contains("worker") {
        "team-governance/prompts/worker.md"
    } else {
        "team-governance/prompts/leader.md"
    }
}

fn operator_surface_root(workspace_root: &Path) -> PathBuf {
    resolve_team_home_root(workspace_root)
        .join(".codex")
        .join(TEAM_OPS_DIRNAME)
}

fn operator_team_root(record: &TeamStateRecord) -> PathBuf {
    operator_surface_root(&record.workspace_root)
        .join("teams")
        .join(public_team_ref(
            &record.team_id,
            &record.role,
            record.depth,
            record.kind.clone(),
        ))
}

fn operator_visible_path(bundle: &TeamStateBundle, actual_path: &Path) -> PathBuf {
    let record = &bundle.record;
    let operator_root = operator_surface_root(&record.workspace_root);
    let team_root = operator_team_root(record);
    if actual_path == record.global_doc_path {
        return operator_root.join("AGENT.md");
    }
    if actual_path == record.team_doc_path {
        return team_root.join("AGENT_TEAM.md");
    }
    if actual_path == record.status_path {
        return team_root.join("status.json");
    }
    if actual_path == record.handoff_path {
        return team_root.join("handoff.json");
    }
    if actual_path == bundle.paths.tape_path {
        return team_root.join("team-tape.jsonl");
    }
    if let Ok(relative) = actual_path.strip_prefix(&record.artifacts_dir) {
        return team_root.join("artifacts").join(relative);
    }
    team_root
        .join("artifacts")
        .join(actual_path.file_name().unwrap_or_default())
}

fn resolve_workspace_path(workspace_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    }
}

async fn owning_bundle_for_path(
    workspace_root: &Path,
    actual_path: &Path,
) -> io::Result<Option<TeamStateBundle>> {
    let team_state_root = resolve_team_home_root(workspace_root)
        .join(TEAM_DIRNAME)
        .join(TEAM_STATE_DIRNAME);
    let relative = match actual_path.strip_prefix(&team_state_root) {
        Ok(relative) => relative,
        Err(_) => return Ok(None),
    };
    let Some(team_id) = relative
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
    else {
        return Ok(None);
    };
    load_team_state_bundle(workspace_root, team_id).await
}

async fn operator_visible_path_for_workspace(
    bundle: &TeamStateBundle,
    path: &Path,
) -> io::Result<PathBuf> {
    let actual_path = resolve_workspace_path(&bundle.record.workspace_root, path);
    if let Some(owner_bundle) =
        owning_bundle_for_path(&bundle.record.workspace_root, &actual_path).await?
    {
        return Ok(operator_visible_path(&owner_bundle, &actual_path));
    }
    Ok(operator_visible_path(bundle, &actual_path))
}

async fn mirror_operator_file(bundle: &TeamStateBundle, actual_path: &Path) -> io::Result<()> {
    let actual_path = resolve_workspace_path(&bundle.record.workspace_root, actual_path);
    if !fs::try_exists(&actual_path).await.unwrap_or(false) {
        return Ok(());
    }
    let mirror_path = operator_visible_path_for_workspace(bundle, &actual_path).await?;
    if let Some(parent) = mirror_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let contents = fs::read(actual_path).await?;
    fs::write(mirror_path, contents).await
}

async fn mirror_operator_index(bundle: &TeamStateBundle) -> io::Result<()> {
    if bundle.record.kind != TeamKind::Root
        || !fs::try_exists(&bundle.paths.index_path)
            .await
            .unwrap_or(false)
    {
        return Ok(());
    }
    let mirror_path = operator_surface_root(&bundle.record.workspace_root).join("index.json");
    if let Some(parent) = mirror_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let contents = fs::read(&bundle.paths.index_path).await?;
    fs::write(mirror_path, contents).await
}

async fn mirror_handoff_artifacts(bundle: &TeamStateBundle) -> io::Result<()> {
    for artifact in &bundle.handoff.produced_artifacts {
        mirror_operator_file(bundle, Path::new(artifact)).await?;
    }
    if let Some(integration) = bundle.handoff.integration.as_ref()
        && let Some(patch_path) = integration.patch_path.as_ref()
    {
        mirror_operator_file(bundle, patch_path).await?;
    }
    Ok(())
}

async fn mirror_tape_artifacts(bundle: &TeamStateBundle) -> io::Result<()> {
    let contents = match fs::read_to_string(&bundle.paths.tape_path).await {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let entry = match serde_json::from_str::<TeamTapeEntry>(line) {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        for artifact_ref in entry.artifact_refs {
            mirror_operator_file(bundle, &artifact_ref).await?;
        }
    }
    Ok(())
}

async fn sync_operator_surface(bundle: &TeamStateBundle) -> io::Result<()> {
    mirror_operator_index(bundle).await?;
    mirror_operator_file(bundle, &bundle.paths.status_path).await?;
    mirror_operator_file(bundle, &bundle.paths.handoff_path).await?;
    mirror_operator_file(bundle, &bundle.paths.tape_path).await?;
    mirror_operator_file(bundle, &bundle.paths.team_doc_path).await?;
    mirror_handoff_artifacts(bundle).await?;
    mirror_tape_artifacts(bundle).await?;
    if bundle.record.kind == TeamKind::Root {
        mirror_operator_file(bundle, &bundle.paths.global_doc_path).await?;
    }
    Ok(())
}

async fn ensure_global_agent_doc(
    paths: &TeamStatePaths,
    bundle: &TeamStateBundle,
    workflow: &TeamWorkflowConfig,
) -> io::Result<()> {
    upsert_generated_markdown(
        &paths.global_doc_path,
        format!(
            "# {GLOBAL_AGENT_DOC_FILENAME}\n\n{}",
            render_global_runtime_section(paths, bundle, workflow)
        ),
        render_global_runtime_section(paths, bundle, workflow),
    )
    .await
}

async fn ensure_team_agent_doc(
    paths: &TeamStatePaths,
    bundle: &TeamStateBundle,
    workflow: &TeamWorkflowConfig,
) -> io::Result<()> {
    let record = &bundle.record;
    let parent_team = record.parent_team_id.as_deref().unwrap_or("root");
    let generated = render_team_runtime_section(bundle, workflow);
    let scaffold = format!(
        "# {TEAM_AGENT_DOC_FILENAME}\n\nThis consensus document is maintained by `{}`.\n\n## Team Scope\n- Team id: `{}`\n- Parent team: `{}`\n- Thread id: `{}`\n- Role: `{}`\n- Depth: `{}`\n\n{}",
        record.team_id,
        record.team_id,
        parent_team,
        record.thread_id,
        record.role,
        record.depth,
        generated
    );
    upsert_generated_markdown(&paths.team_doc_path, scaffold, generated).await
}

async fn upsert_generated_markdown(
    path: &Path,
    scaffold: String,
    generated: String,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let next = match fs::read_to_string(path).await {
        Ok(existing) => merge_generated_section(existing, generated),
        Err(err) if err.kind() == io::ErrorKind::NotFound => scaffold,
        Err(err) => return Err(err),
    };
    fs::write(path, next).await
}

fn merge_generated_section(existing: String, generated: String) -> String {
    let Some(start_idx) = existing.find(GENERATED_SECTION_START) else {
        return format!("{}\n\n{}\n", existing.trim_end(), generated);
    };
    let Some(end_rel) = existing[start_idx..].find(GENERATED_SECTION_END) else {
        return format!("{}\n\n{}\n", existing.trim_end(), generated);
    };
    let end_idx = start_idx + end_rel + GENERATED_SECTION_END.len();
    format!(
        "{}{}{}",
        &existing[..start_idx],
        generated,
        &existing[end_idx..]
    )
}

fn wrap_generated_asset(contents: &str) -> String {
    format!(
        "{GENERATED_SECTION_START}\n{}\n{GENERATED_SECTION_END}\n",
        contents.trim()
    )
}

fn render_global_runtime_section(
    paths: &TeamStatePaths,
    bundle: &TeamStateBundle,
    workflow: &TeamWorkflowConfig,
) -> String {
    let record = &bundle.record;
    let root_checkout = bundle
        .record
        .worktree
        .as_ref()
        .map(|worktree| display_path(&worktree.checkout_path, &record.workspace_root).into_owned())
        .unwrap_or_else(|| "unassigned".to_string());
    let root_branch = bundle
        .record
        .worktree
        .as_ref()
        .map(|worktree| worktree.branch_name.clone())
        .unwrap_or_else(|| "unassigned".to_string());
    let modes = workflow
        .decision_policy
        .allowed_modes
        .iter()
        .map(|mode| format!("`{}`", format_execution_mode(*mode)))
        .collect::<Vec<_>>()
        .join(", ");
    let roles = workflow
        .workflow_loop
        .required_roles
        .iter()
        .map(|role| format!("`{}`", format_iteration_role(*role)))
        .collect::<Vec<_>>()
        .join(", ");
    let triggers = workflow
        .governance
        .update_triggers
        .iter()
        .map(|trigger| format!("`{}`", format_governance_trigger(*trigger)))
        .collect::<Vec<_>>()
        .join(", ");
    let prompt_root = Path::new(".codex").join(GOVERNANCE_PROMPTS_DIR);
    format!(
        "{GENERATED_SECTION_START}\n## Runtime Checkpoint\n- Root team: `{}`\n- Workflow: `{}`\n- Team state index: `{}`\n- Status snapshot: `{}`\n- Handoff metadata: `{}`\n- Team tape: `{}`\n- Governance prompt root: `{}`\n- Scheduler prompt: `.codex/{}`\n- Shared team skills: `.codex/skills/{}/SKILL.md`, `.codex/skills/{}/SKILL.md`, `.codex/skills/{}/SKILL.md`\n- Root checkout: `{}`\n- Root branch namespace: `{}`\n- Active child teams: `{}`\n- Current phase: `{}`\n- Allowed execution modes: {}\n- Required iteration roles: {}\n- Maximum nested team depth: `{}`\n- Governance update triggers: {}\n- Last updated: `{}`\n{GENERATED_SECTION_END}",
        record.team_id,
        display_path(&record.workflow_path, &record.workspace_root),
        display_path(&paths.index_path, &record.workspace_root),
        display_path(&record.status_path, &record.workspace_root),
        display_path(&record.handoff_path, &record.workspace_root),
        display_path(&bundle.paths.tape_path, &record.workspace_root),
        prompt_root.display(),
        role_prompt_relative_path(record),
        TEAM_SKILL_DELEGATION,
        TEAM_SKILL_REVIEW_LOOP,
        TEAM_SKILL_GOVERNANCE,
        root_checkout,
        root_branch,
        record.child_team_ids.len(),
        format_team_phase(&bundle.status.current_phase),
        modes,
        roles,
        workflow.max_depth,
        triggers,
        Utc::now().to_rfc3339()
    )
}

fn render_team_runtime_section(bundle: &TeamStateBundle, workflow: &TeamWorkflowConfig) -> String {
    let record = &bundle.record;
    let worktree_checkout = bundle
        .record
        .worktree
        .as_ref()
        .map(|worktree| display_path(&worktree.checkout_path, &record.workspace_root).into_owned())
        .unwrap_or_else(|| "unassigned".to_string());
    let worktree_branch = bundle
        .record
        .worktree
        .as_ref()
        .map(|worktree| worktree.branch_name.clone())
        .unwrap_or_else(|| "unassigned".to_string());
    let current_branch = bundle
        .record
        .worktree
        .as_ref()
        .and_then(|worktree| worktree.current_branch.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let managed_worktree = bundle
        .record
        .worktree
        .as_ref()
        .map(|worktree| worktree.managed)
        .unwrap_or(false);
    let active_resource_count = bundle.status.environment.managed_resources.len();
    let stale_resource_count = bundle.status.environment.stale_resources.len();
    let roles = workflow
        .workflow_loop
        .required_roles
        .iter()
        .map(|role| format!("`{}`", format_iteration_role(*role)))
        .collect::<Vec<_>>()
        .join(", ");
    let skills = if bundle.audit.synthesized_skills.is_empty() {
        "none".to_string()
    } else {
        bundle.audit.synthesized_skills.join(" | ")
    };
    let role_prompt = role_prompt_relative_path(record);
    format!(
        "{GENERATED_SECTION_START}\n## Runtime Checkpoint\n- Artifacts directory: `{}`\n- Status snapshot: `{}`\n- Handoff metadata: `{}`\n- Recovery snapshot: `{}`\n- Audit log: `{}`\n- Team tape: `{}`\n- Role decision model: `.codex/{}`\n- Reusable team skills: `.codex/skills/{}/SKILL.md`, `.codex/skills/{}/SKILL.md`, `.codex/skills/{}/SKILL.md`\n- Checkout path: `{}`\n- Branch namespace: `{}`\n- Current branch: `{}`\n- Managed linked worktree: `{}`\n- Active managed resources: `{}`\n- Stale resources: `{}`\n- Last rollout: `{}`\n- Same-level protocol: `a2a`\n- Cross-level contract: `openspec-artifacts`\n- Required cycle roles: {}\n- Current cycle phase: `{}`\n- Replan reason: `{}`\n- Active child teams: `{}`\n- Recovered blockers: `{}`\n- Next steps: `{}`\n- Synthesized skills: `{}`\n- Last updated: `{}`\n{GENERATED_SECTION_END}",
        display_path(&record.artifacts_dir, &record.workspace_root),
        display_path(&record.status_path, &record.workspace_root),
        display_path(&record.handoff_path, &record.workspace_root),
        display_path(&bundle.paths.recovery_path, &record.workspace_root),
        display_path(&bundle.paths.audit_path, &record.workspace_root),
        display_path(&bundle.paths.tape_path, &record.workspace_root),
        role_prompt,
        TEAM_SKILL_DELEGATION,
        TEAM_SKILL_COMPACT,
        TEAM_SKILL_HANDOFF,
        worktree_checkout,
        worktree_branch,
        current_branch,
        managed_worktree,
        active_resource_count,
        stale_resource_count,
        record
            .rollout_path
            .as_ref()
            .map(|path| display_path(path, &record.workspace_root).into_owned())
            .unwrap_or_else(|| "not persisted yet".to_string()),
        roles,
        format_team_phase(&bundle.status.current_phase),
        bundle
            .status
            .cycle
            .replan_reason
            .as_deref()
            .unwrap_or("none"),
        record.child_team_ids.len(),
        bundle.status.blockers.join(" | "),
        bundle.status.next_steps.join(" | "),
        skills,
        Utc::now().to_rfc3339()
    )
}

fn determine_relationship(
    sender: &TeamStateRecord,
    receiver: &TeamStateRecord,
) -> TeamRelationship {
    if sender.team_id == receiver.team_id {
        return TeamRelationship::SameTeam;
    }
    if sender.parent_team_id == receiver.parent_team_id && sender.depth == receiver.depth {
        return TeamRelationship::Sibling;
    }
    if sender.parent_team_id.as_deref() == Some(receiver.team_id.as_str())
        || receiver.parent_team_id.as_deref() == Some(sender.team_id.as_str())
    {
        return TeamRelationship::Vertical;
    }
    if sender.depth != receiver.depth || sender.parent_team_id != receiver.parent_team_id {
        return TeamRelationship::SeparateBoundary;
    }
    TeamRelationship::None
}

async fn prepare_vertical_handoff(
    workspace_root: &Path,
    sender_team_id: &str,
    receiver_team_id: Option<&str>,
    items: Vec<UserInput>,
    prefix: &str,
) -> io::Result<PreparedTeamMessage> {
    let Some(sender) = load_team_state_bundle(workspace_root, sender_team_id).await? else {
        return Ok(PreparedTeamMessage {
            relationship: TeamRelationship::None,
            summary: summarize_input(&items),
            items,
            artifact_refs: Vec::new(),
            integration_handoff: None,
        });
    };
    let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    let receiver_bundle = match receiver_team_id {
        Some(team_id) => load_team_state_bundle(workspace_root, team_id).await?,
        None => None,
    };
    let receiver_suffix =
        vertical_receiver_label(sender.record.parent_team_id.as_deref(), receiver_team_id);
    let artifact_path = sender
        .paths
        .artifacts_dir
        .join(format!("{prefix}-{timestamp}-to-{receiver_suffix}.md"));
    if let Some(parent) = artifact_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let summary = sanitize_user_input_summary(&items);
    let integration_handoff =
        build_integration_handoff(&sender, receiver_bundle.as_ref(), &timestamp.to_string())
            .await?;
    let mut artifact_refs = vec![
        artifact_path,
        sender.paths.status_path.clone(),
        sender.paths.handoff_path.clone(),
        sender.paths.tape_path.clone(),
        sender.paths.team_doc_path.clone(),
        sender.paths.global_doc_path.clone(),
    ];
    if let Some(integration) = integration_handoff.as_ref()
        && let Some(patch_path) = integration.patch_path.as_ref()
    {
        artifact_refs.push(patch_path.clone());
    }
    let manifest = render_vertical_manifest(
        &sender,
        artifact_refs.as_slice(),
        integration_handoff.as_ref(),
        workspace_root,
    );
    fs::write(
        &artifact_refs[0],
        build_vertical_handoff_markdown(
            &sender,
            &summary,
            artifact_refs.as_slice(),
            integration_handoff.as_ref(),
            workspace_root,
        ),
    )
    .await?;
    for artifact_ref in &artifact_refs {
        mirror_operator_file(&sender, artifact_ref).await?;
    }
    Ok(PreparedTeamMessage {
        relationship: TeamRelationship::Vertical,
        summary,
        items: vec![UserInput::Text {
            text: manifest,
            text_elements: Vec::new(),
        }],
        artifact_refs,
        integration_handoff,
    })
}

async fn build_integration_handoff(
    sender: &TeamStateBundle,
    receiver: Option<&TeamStateBundle>,
    timestamp: &str,
) -> io::Result<Option<TeamIntegrationHandoff>> {
    let Some(worktree) = sender.record.worktree.as_ref() else {
        return Ok(None);
    };
    if !worktree.managed {
        return Ok(None);
    }
    let head_commit = worktree
        .head_commit
        .clone()
        .or_else(|| {
            sender
                .handoff
                .integration
                .as_ref()
                .and_then(|handoff| handoff.head_commit.clone())
        })
        .or_else(|| Some(String::new()))
        .filter(|commit| !commit.is_empty());
    let base_commit = worktree.base_commit.clone();
    let patch_path = if worktree.managed {
        Some(write_integration_patch(sender, worktree, timestamp).await?)
    } else {
        None
    };
    let target_checkout_path = receiver.map(|bundle| {
        let target_public_id = public_team_ref(
            &bundle.record.team_id,
            &bundle.record.role,
            bundle.record.depth,
            bundle.record.kind.clone(),
        );
        public_worktree_label(
            &target_public_id,
            bundle
                .record
                .worktree
                .as_ref()
                .map(|state| state.managed)
                .unwrap_or(false),
        )
    });
    let source_public_id = public_team_ref(
        &sender.record.team_id,
        &sender.record.role,
        sender.record.depth,
        sender.record.kind.clone(),
    );

    Ok(Some(TeamIntegrationHandoff {
        source_team_id: source_public_id.clone(),
        target_team_id: receiver.map(|bundle| {
            public_team_ref(
                &bundle.record.team_id,
                &bundle.record.role,
                bundle.record.depth,
                bundle.record.kind.clone(),
            )
        }),
        source_branch: Some(worktree.branch_name.clone()),
        source_checkout_path: public_worktree_label(&source_public_id, worktree.managed),
        target_checkout_path,
        base_commit,
        head_commit,
        patch_path: patch_path.as_ref().map(|path| {
            sanitize_workspace_path(path, &sender.record.workspace_root, "integration.patch")
        }),
        accepted_modes: vec![
            TeamIntegrationMode::Merge,
            TeamIntegrationMode::CherryPick,
            TeamIntegrationMode::Patch,
        ],
        review_ready: true,
        updated_at: Utc::now().to_rfc3339(),
    }))
}

async fn write_integration_patch(
    sender: &TeamStateBundle,
    worktree: &TeamWorktreeState,
    timestamp: &str,
) -> io::Result<PathBuf> {
    let patch_path = sender
        .paths
        .artifacts_dir
        .join(format!("integration-{timestamp}.patch"));
    let diff_args = if let Some(base_commit) = worktree.base_commit.as_deref() {
        vec![
            "diff".to_string(),
            "--binary".to_string(),
            base_commit.to_string(),
            "HEAD".to_string(),
        ]
    } else {
        vec!["diff".to_string(), "--binary".to_string()]
    };
    let output = Command::new("git")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(&diff_args)
        .current_dir(&worktree.checkout_path)
        .output()
        .await?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "git {} failed: {}",
            diff_args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    fs::write(&patch_path, output.stdout).await?;
    Ok(patch_path)
}

fn render_integration_manifest(integration: &TeamIntegrationHandoff) -> String {
    let accepted_modes = integration
        .accepted_modes
        .iter()
        .map(|mode| match mode {
            TeamIntegrationMode::Merge => "merge",
            TeamIntegrationMode::CherryPick => "cherry-pick",
            TeamIntegrationMode::Patch => "patch",
        })
        .collect::<Vec<_>>()
        .join(",");
    let patch_line = integration
        .patch_path
        .as_ref()
        .map(|path| format!("\npatch: {}", path.display()))
        .unwrap_or_default();
    let source_branch = integration.source_branch.as_deref().unwrap_or("unknown");
    let head_commit = integration.head_commit.as_deref().unwrap_or("unknown");
    let base_commit = integration.base_commit.as_deref().unwrap_or("unknown");
    format!(
        "\nintegration_modes: {accepted_modes}\nsource_branch: {source_branch}\nbase_commit: {base_commit}\nhead_commit: {head_commit}{patch_line}"
    )
}

fn render_vertical_manifest(
    sender: &TeamStateBundle,
    artifact_refs: &[PathBuf],
    integration: Option<&TeamIntegrationHandoff>,
    _workspace_root: &Path,
) -> String {
    let artifact = artifact_refs
        .first()
        .map(|path| operator_visible_path(sender, path))
        .unwrap_or_else(|| PathBuf::from("handoff.md"));
    let status = operator_visible_path(sender, &sender.paths.status_path);
    let handoff = operator_visible_path(sender, &sender.paths.handoff_path);
    let tape = operator_visible_path(sender, &sender.paths.tape_path);
    let governance = operator_visible_path(sender, &sender.paths.team_doc_path);
    let global_governance = operator_visible_path(sender, &sender.paths.global_doc_path);
    let integration_manifest = integration
        .as_ref()
        .map(|handoff| render_integration_manifest(handoff))
        .unwrap_or_default();
    format!(
        "protocol: codex-team-artifacts\nrelationship: vertical\nartifact: {}\nstatus: {}\nhandoff: {}\ntape: {}\ngovernance: {}\nglobal_governance: {}\nnext_action: Review persisted artifacts, then continue the next bounded step.{}",
        artifact.display(),
        status.display(),
        handoff.display(),
        tape.display(),
        governance.display(),
        global_governance.display(),
        integration_manifest
    )
}

fn build_vertical_handoff_markdown(
    sender: &TeamStateBundle,
    summary: &str,
    artifact_refs: &[PathBuf],
    integration: Option<&TeamIntegrationHandoff>,
    _workspace_root: &Path,
) -> String {
    let declared_outputs = artifact_refs
        .iter()
        .map(|path| operator_visible_path(sender, path))
        .map(|path| format!("- `{}`", path.display()))
        .collect::<Vec<_>>()
        .join("\n");
    let integration_section = integration
        .map(|handoff| {
            format!(
                "\n## Integration Contract\n- Source team ref: `{}`\n- Target team ref: `{}`\n- Source branch: `{}`\n- Accepted modes: `{}`\n- Head commit: `{}`\n",
                handoff.source_team_id,
                handoff.target_team_id.as_deref().unwrap_or("pending-child"),
                handoff.source_branch.as_deref().unwrap_or("unknown"),
                handoff
                    .accepted_modes
                    .iter()
                    .map(|mode| match mode {
                        TeamIntegrationMode::Merge => "merge",
                        TeamIntegrationMode::CherryPick => "cherry-pick",
                        TeamIntegrationMode::Patch => "patch",
                    })
                    .collect::<Vec<_>>()
                    .join(","),
                handoff.head_commit.as_deref().unwrap_or("unknown"),
            )
        })
        .unwrap_or_default();
    format!(
        "# Codex Vertical Handoff\n\n## Summary\n{}\n\n## Declared Outputs\n{}\n\n## Next Action\n- Review persisted status, governance docs, and artifacts before continuing.\n\n## Blockers\n- None recorded in this handoff.\n\n## Governance Deltas\n- Update `AGENT_TEAM.md` before the next delegation round if review changed local rules.\n{}",
        sanitize_summary_text(summary),
        declared_outputs,
        integration_section
    )
}

fn summarize_input(items: &[UserInput]) -> String {
    let parts = items
        .iter()
        .map(|item| match item {
            UserInput::Text { text, .. } => text.trim().to_string(),
            UserInput::Mention { name, path } => format!("mention:{name} ({path})"),
            UserInput::Skill { name, path } => format!("skill:{name} ({})", path.display()),
            UserInput::LocalImage { path } => format!("local_image:{}", path.display()),
            UserInput::Image { .. } => "image".to_string(),
            _ => "input".to_string(),
        })
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        "artifact handoff".to_string()
    } else {
        parts.join("\n")
    }
}

fn render_input(items: &[UserInput]) -> String {
    items
        .iter()
        .map(|item| match item {
            UserInput::Text { text, .. } => format!("- text: {}", text.trim()),
            UserInput::Mention { name, path } => format!("- mention `{name}` -> `{path}`"),
            UserInput::Skill { name, path } => format!("- skill `{name}` -> `{}`", path.display()),
            UserInput::LocalImage { path } => format!("- local image `{}`", path.display()),
            UserInput::Image { .. } => "- image".to_string(),
            _ => "- input".to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn merge_artifacts(existing: &[String], next: &[PathBuf], workspace_root: &Path) -> Vec<String> {
    let mut merged = existing.to_vec();
    for path in next {
        let item = display_path(path, workspace_root).to_string();
        if !merged.iter().any(|entry| entry == &item) {
            merged.push(item);
        }
    }
    merged.sort();
    merged
}

fn synthesize_skills(entries: &[TeamAuditEntry]) -> Vec<String> {
    let correction = entries
        .iter()
        .filter(|entry| entry.detected_instruction_drift)
        .count();
    let peer = entries
        .iter()
        .filter(|entry| entry.kind == TeamAuditKind::PeerSync)
        .count();
    let handoff = entries
        .iter()
        .filter(|entry| entry.kind == TeamAuditKind::ArtifactHandoff)
        .count();
    let mut skills = Vec::new();
    if correction > 0 {
        skills.push("Restate boundaries before delegation after review finds drift.".to_string());
    }
    if peer > 0 {
        skills.push("Use sibling peer syncs to externalize assumptions and blockers.".to_string());
    }
    if handoff > 0 {
        skills.push("Use artifact bundles for every parent-child handoff.".to_string());
    }
    skills
}

fn display_path<'a>(path: &'a Path, workspace_root: &'a Path) -> Cow<'a, str> {
    match path.strip_prefix(workspace_root) {
        Ok(relative) => Cow::Owned(relative.display().to_string()),
        Err(_) => Cow::Owned(path.display().to_string()),
    }
}
fn format_execution_mode(mode: ExecutionMode) -> &'static str {
    match mode {
        ExecutionMode::Single => "single",
        ExecutionMode::Delegate => "delegate",
        ExecutionMode::Parallel => "parallel",
    }
}
fn format_iteration_role(role: IterationRole) -> &'static str {
    match role {
        IterationRole::Design => "design",
        IterationRole::Development => "development",
        IterationRole::Review => "review",
    }
}
fn format_governance_trigger(trigger: GovernanceTrigger) -> &'static str {
    match trigger {
        GovernanceTrigger::TeamCreated => "team_created",
        GovernanceTrigger::Replan => "replan",
        GovernanceTrigger::ReviewHandoff => "review_handoff",
        GovernanceTrigger::Compact => "compact",
        GovernanceTrigger::LeaderResume => "leader_resume",
    }
}
fn format_team_phase(phase: &TeamPhase) -> &'static str {
    match phase {
        TeamPhase::Bootstrap => "bootstrap",
        TeamPhase::Design => "design",
        TeamPhase::Development => "development",
        TeamPhase::Review => "review",
        TeamPhase::Replan => "replan",
    }
}
