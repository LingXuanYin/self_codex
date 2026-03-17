use super::GLOBAL_AGENT_DOC_FILENAME;
use super::TEAM_AGENT_DOC_FILENAME;
use super::TeamWorkflowPublicTapeKind;
use super::TeamWorkflowThreadVisibility;
use super::config::TeamWorkflowConfig;
use super::config::load_workflow_from_workspace;
use super::load_public_team_workflow_session;
use super::runtime::maybe_initialize_for_thread;
use super::runtime::prepare_team_message;
use super::runtime::record_team_message_delivery;
use super::state::TEAM_AUDIT_FILENAME;
use super::state::TEAM_HANDOFF_FILENAME;
use super::state::TEAM_INDEX_FILENAME;
use super::state::TEAM_METADATA_FILENAME;
use super::state::TEAM_RECOVERY_FILENAME;
use super::state::TEAM_STATUS_FILENAME;
use super::state::TEAM_TAPE_FILENAME;
use super::state::TeamPhase;
use super::state::load_team_state_bundle;
use super::state::write_team_state_bundle;
use super::team_workflow_thread_visibility;
use codex_protocol::ThreadId;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::user_input::UserInput;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

async fn write_workflow(temp_dir: &TempDir, contents: &str) {
    let codex_dir = temp_dir.path().join(".codex");
    tokio::fs::create_dir_all(&codex_dir)
        .await
        .expect("create .codex dir");
    tokio::fs::write(codex_dir.join("team-workflow.yaml"), contents)
        .await
        .expect("write workflow");
}

fn run_git(cwd: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap_or_else(|err| panic!("git {:?} failed to start: {err}", args));
    assert!(
        status.success(),
        "git {:?} failed in {}",
        args,
        cwd.display()
    );
}

fn init_git_repo(temp_dir: &TempDir) {
    run_git(temp_dir.path(), &["init", "--initial-branch=main"]);
    run_git(temp_dir.path(), &["config", "user.name", "Codex"]);
    run_git(
        temp_dir.path(),
        &["config", "user.email", "codex@example.com"],
    );
    std::fs::write(temp_dir.path().join("README.md"), "seed\n").expect("write readme");
    run_git(temp_dir.path(), &["add", "README.md"]);
    run_git(temp_dir.path(), &["commit", "-m", "init"]);
}

#[tokio::test]
async fn workflow_loader_applies_default_depth_and_roles() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_workflow(
        &temp_dir,
        "version: 1\nrootScheduler:\n  role: root-scheduler\n",
    )
    .await;

    let workflow = load_workflow_from_workspace(temp_dir.path())
        .await
        .expect("load workflow")
        .expect("workflow should exist");

    assert_eq!(workflow.max_depth, 5);
    assert_eq!(
        workflow.workflow_loop.required_roles,
        TeamWorkflowConfig::default().workflow_loop.required_roles
    );
}

#[tokio::test]
async fn workflow_loader_rejects_missing_triads() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_workflow(
        &temp_dir,
        "version: 1\nworkflowLoop:\n  requiredRoles:\n    - design\n    - review\n",
    )
    .await;

    let err = load_workflow_from_workspace(temp_dir.path())
        .await
        .expect_err("missing development role should fail");

    assert!(
        err.to_string()
            .contains("workflowLoop.requiredRoles must include design, development, and review")
    );
}

#[tokio::test]
async fn root_team_initialization_persists_state_and_governance_docs() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_workflow(&temp_dir, "version: 1\n").await;
    let thread_id = ThreadId::new();
    let rollout_path = temp_dir.path().join("sessions").join("rollout.jsonl");

    maybe_initialize_for_thread(
        temp_dir.path(),
        thread_id,
        &SessionSource::Exec,
        Some(rollout_path.as_path()),
    )
    .await
    .expect("initialize team runtime");

    let team_dir = temp_dir
        .path()
        .join(".codex")
        .join("team-state")
        .join(thread_id.to_string());
    assert!(team_dir.join(TEAM_METADATA_FILENAME).exists());
    assert!(team_dir.join(TEAM_STATUS_FILENAME).exists());
    assert!(team_dir.join(TEAM_HANDOFF_FILENAME).exists());
    assert!(team_dir.join(TEAM_RECOVERY_FILENAME).exists());
    assert!(team_dir.join(TEAM_AUDIT_FILENAME).exists());
    assert!(team_dir.join(TEAM_TAPE_FILENAME).exists());
    assert!(team_dir.join(TEAM_AGENT_DOC_FILENAME).exists());
    assert!(team_dir.join("artifacts").exists());
    assert!(
        temp_dir
            .path()
            .join(".codex")
            .join(GLOBAL_AGENT_DOC_FILENAME)
            .exists()
    );
    assert!(
        temp_dir
            .path()
            .join(".codex")
            .join("team-state")
            .join(TEAM_INDEX_FILENAME)
            .exists()
    );
    let tape = tokio::fs::read_to_string(team_dir.join(TEAM_TAPE_FILENAME))
        .await
        .expect("read team tape");
    assert!(tape.contains("\"kind\":\"bootstrap\""));
}

#[tokio::test]
async fn child_team_initialization_tracks_parent_and_preserves_manual_doc_content() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_workflow(&temp_dir, "version: 1\n").await;
    let parent_thread_id = ThreadId::new();
    let child_thread_id = ThreadId::new();
    maybe_initialize_for_thread(
        temp_dir.path(),
        parent_thread_id,
        &SessionSource::Exec,
        None,
    )
    .await
    .expect("initialize parent team");
    let team_dir = temp_dir
        .path()
        .join(".codex")
        .join("team-state")
        .join(child_thread_id.to_string());
    tokio::fs::create_dir_all(&team_dir)
        .await
        .expect("create team dir");
    tokio::fs::write(
        team_dir.join(TEAM_AGENT_DOC_FILENAME),
        "# AGENT_TEAM.md\n\nManual notes.\n",
    )
    .await
    .expect("seed manual team doc");

    maybe_initialize_for_thread(
        temp_dir.path(),
        child_thread_id,
        &SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id,
            depth: 1,
            agent_nickname: Some("Dirac".to_string()),
            agent_role: Some("review-lead".to_string()),
        }),
        None,
    )
    .await
    .expect("initialize child team");

    let team_state = tokio::fs::read_to_string(team_dir.join(TEAM_METADATA_FILENAME))
        .await
        .expect("read team metadata");
    assert!(team_state.contains(parent_thread_id.to_string().as_str()));
    let parent_bundle = load_team_state_bundle(temp_dir.path(), &parent_thread_id.to_string())
        .await
        .expect("load parent bundle")
        .expect("parent bundle should exist");
    assert!(
        parent_bundle
            .record
            .child_team_ids
            .iter()
            .any(|team_id| team_id == &child_thread_id.to_string())
    );
    let team_doc = tokio::fs::read_to_string(team_dir.join(TEAM_AGENT_DOC_FILENAME))
        .await
        .expect("read team doc");
    assert!(team_doc.contains("Manual notes."));
    assert!(team_doc.contains("codex-team-runtime:start"));
}

#[tokio::test]
async fn reinitialization_preserves_cycle_and_recovery_state() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_workflow(&temp_dir, "version: 1\n").await;
    let team_id = ThreadId::new();

    maybe_initialize_for_thread(temp_dir.path(), team_id, &SessionSource::Exec, None)
        .await
        .expect("initialize team");

    let mut bundle = load_team_state_bundle(temp_dir.path(), &team_id.to_string())
        .await
        .expect("load bundle")
        .expect("bundle should exist");
    bundle.status.current_phase = TeamPhase::Review;
    bundle.status.cycle.phase = TeamPhase::Review;
    bundle.status.blockers = vec!["review requested rework".to_string()];
    bundle.status.next_steps = vec!["return work to design".to_string()];
    bundle.recovery.last_compact_checkpoint_at = Some("2026-03-17T00:00:00Z".to_string());
    write_team_state_bundle(&bundle)
        .await
        .expect("persist modified bundle");

    maybe_initialize_for_thread(temp_dir.path(), team_id, &SessionSource::Exec, None)
        .await
        .expect("reinitialize team");

    let bundle = load_team_state_bundle(temp_dir.path(), &team_id.to_string())
        .await
        .expect("reload bundle")
        .expect("bundle should exist");
    assert_eq!(bundle.status.current_phase, TeamPhase::Review);
    assert_eq!(bundle.status.cycle.phase, TeamPhase::Review);
    assert_eq!(bundle.status.blockers, vec!["review requested rework"]);
    assert_eq!(bundle.status.next_steps, vec!["return work to design"]);
    assert_eq!(
        bundle.recovery.last_compact_checkpoint_at.as_deref(),
        Some("2026-03-17T00:00:00Z")
    );
}

#[tokio::test]
async fn child_team_initialization_allocates_managed_worktree_and_branch_namespace() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    init_git_repo(&temp_dir);
    write_workflow(&temp_dir, "version: 1\n").await;
    let parent_thread_id = ThreadId::new();
    let child_thread_id = ThreadId::new();

    maybe_initialize_for_thread(
        temp_dir.path(),
        parent_thread_id,
        &SessionSource::Exec,
        None,
    )
    .await
    .expect("initialize root team");
    maybe_initialize_for_thread(
        temp_dir.path(),
        child_thread_id,
        &SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id,
            depth: 1,
            agent_nickname: Some("Noether".to_string()),
            agent_role: Some("development-lead".to_string()),
        }),
        None,
    )
    .await
    .expect("initialize child team");

    let root_bundle = load_team_state_bundle(temp_dir.path(), &parent_thread_id.to_string())
        .await
        .expect("load root bundle")
        .expect("root bundle exists");
    let child_bundle = load_team_state_bundle(temp_dir.path(), &child_thread_id.to_string())
        .await
        .expect("load child bundle")
        .expect("child bundle exists");

    let root_worktree = root_bundle.record.worktree.expect("root worktree assigned");
    assert_eq!(root_worktree.checkout_path, temp_dir.path());
    assert_eq!(
        root_worktree.branch_name,
        format!("team/{}/root", parent_thread_id)
    );
    assert!(!root_worktree.managed);

    let child_worktree = child_bundle
        .record
        .worktree
        .clone()
        .expect("child worktree assigned");
    assert!(child_worktree.managed);
    assert_eq!(
        child_worktree.branch_name,
        format!("team/{}/{}", parent_thread_id, child_thread_id)
    );
    assert_eq!(
        child_worktree.source_checkout_path.as_deref(),
        Some(temp_dir.path())
    );
    assert!(child_worktree.checkout_path.join(".git").exists());
    assert_eq!(
        child_worktree.current_branch.as_deref(),
        Some(child_worktree.branch_name.as_str())
    );
    assert_eq!(child_bundle.status.environment.managed_resources.len(), 1);
    assert_eq!(child_bundle.status.environment.stale_resources.len(), 0);
    assert!(
        child_bundle.status.environment.managed_resources[0].cleanup_required,
        "managed child worktrees should be tracked for cleanup"
    );
}

#[tokio::test]
async fn child_to_parent_handoff_includes_reviewable_integration_metadata() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    init_git_repo(&temp_dir);
    write_workflow(&temp_dir, "version: 1\n").await;
    let parent_thread_id = ThreadId::new();
    let child_thread_id = ThreadId::new();

    maybe_initialize_for_thread(
        temp_dir.path(),
        parent_thread_id,
        &SessionSource::Exec,
        None,
    )
    .await
    .expect("initialize root team");
    maybe_initialize_for_thread(
        temp_dir.path(),
        child_thread_id,
        &SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id,
            depth: 1,
            agent_nickname: Some("Noether".to_string()),
            agent_role: Some("development-lead".to_string()),
        }),
        None,
    )
    .await
    .expect("initialize child team");

    let child_bundle = load_team_state_bundle(temp_dir.path(), &child_thread_id.to_string())
        .await
        .expect("load child bundle")
        .expect("child bundle exists");
    let child_worktree = child_bundle
        .record
        .worktree
        .expect("child worktree assigned");
    std::fs::write(
        child_worktree.checkout_path.join("module.rs"),
        "pub fn ready() {}\n",
    )
    .expect("write module");
    run_git(&child_worktree.checkout_path, &["add", "module.rs"]);
    run_git(
        &child_worktree.checkout_path,
        &["commit", "-m", "child change"],
    );

    let prepared = prepare_team_message(
        temp_dir.path(),
        child_thread_id,
        parent_thread_id,
        vec![UserInput::Text {
            text: "ready for parent review".to_string(),
            text_elements: Vec::new(),
        }],
    )
    .await
    .expect("prepare handoff");

    let manifest = match &prepared.items[0] {
        UserInput::Text { text, .. } => text.as_str(),
        other => panic!("expected manifest text, got {other:?}"),
    };
    assert!(manifest.contains("integration_modes: merge,cherry-pick,patch"));
    assert!(manifest.contains("source_branch:"));
    assert!(manifest.contains("patch:"));
    let integration = prepared
        .integration_handoff
        .expect("integration handoff should be recorded");
    assert!(integration.review_ready);
    assert!(integration.patch_path.expect("patch path").exists());
    let tape = std::fs::read_to_string(
        temp_dir
            .path()
            .join(".codex")
            .join("team-state")
            .join(child_thread_id.to_string())
            .join(TEAM_TAPE_FILENAME),
    )
    .expect("read team tape");
    assert!(tape.contains("\"kind\":\"worktree_assigned\""));
}

#[tokio::test]
async fn public_team_visibility_hides_child_threads_but_keeps_root_public() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_workflow(&temp_dir, "version: 1\n").await;
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
            agent_nickname: Some("Ada".to_string()),
            agent_role: Some("design-lead".to_string()),
        }),
        None,
    )
    .await
    .expect("initialize child team");

    assert_eq!(
        team_workflow_thread_visibility(temp_dir.path(), root_thread_id)
            .await
            .expect("resolve root visibility"),
        TeamWorkflowThreadVisibility::PublicRoot
    );
    assert_eq!(
        team_workflow_thread_visibility(temp_dir.path(), child_thread_id)
            .await
            .expect("resolve child visibility"),
        TeamWorkflowThreadVisibility::HiddenChild { root_thread_id }
    );
}

#[tokio::test]
async fn public_team_session_redacts_child_context_but_keeps_artifacts_and_status() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_workflow(&temp_dir, "version: 1\n").await;
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
            agent_nickname: Some("Ada".to_string()),
            agent_role: Some("development-lead".to_string()),
        }),
        None,
    )
    .await
    .expect("initialize child team");

    let prepared = prepare_team_message(
        temp_dir.path(),
        child_thread_id,
        root_thread_id,
        vec![UserInput::Text {
            text: "private implementation details and interface assumptions".to_string(),
            text_elements: Vec::new(),
        }],
    )
    .await
    .expect("prepare handoff");
    record_team_message_delivery(temp_dir.path(), child_thread_id, root_thread_id, &prepared)
        .await
        .expect("record handoff");
    assert!(
        !prepared.artifact_refs.is_empty(),
        "vertical handoff should persist artifacts"
    );

    let session = load_public_team_workflow_session(temp_dir.path(), root_thread_id, 8)
        .await
        .expect("load public session")
        .expect("session exists");
    assert_eq!(session.root_thread_id, root_thread_id);
    assert_eq!(session.active_team_count, 2);
    let child_team = session
        .teams
        .iter()
        .find(|team| team.thread_id == child_thread_id)
        .expect("child team present in redacted session");
    assert_eq!(
        child_team.produced_artifacts.len(),
        prepared.artifact_refs.len()
    );
    assert!(
        child_team
            .recent_tape
            .iter()
            .all(|entry| !entry.summary.contains("private implementation details")),
        "public tape summaries must be redacted"
    );
    assert!(
        child_team
            .recent_tape
            .iter()
            .any(|entry| entry.kind == TeamWorkflowPublicTapeKind::Bootstrap),
        "bootstrap tape should still be visible"
    );
}
