use super::GLOBAL_AGENT_DOC_FILENAME;
use super::TEAM_AGENT_DOC_FILENAME;
use super::TeamWorkflowPublicLifecycleState;
use super::TeamWorkflowPublicMemoryProviderHealth;
use super::TeamWorkflowPublicMemoryProviderMode;
use super::TeamWorkflowThreadVisibility;
use super::config::TeamWorkflowConfig;
use super::config::load_workflow_from_workspace;
use super::load_public_team_workflow_session;
use super::record_team_compact_checkpoint;
use super::redaction::sanitize_summary_for_export;
use super::redaction::sanitize_workspace_path;
use super::runtime::maybe_initialize_for_thread;
use super::runtime::prepare_team_message;
use super::runtime::record_team_message_delivery;
use super::runtime::record_team_resume;
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
use serde_json::Value;
use std::path::Path;
use std::path::PathBuf;
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

#[test]
fn sanitize_summary_for_export_scrubs_json_escaped_workspace_root() {
    let workspace_root = PathBuf::from(r"C:\root\proj");
    let input = r#"handoff path: C:\\root\\proj\\src\\main.rs (and also C:\root\proj)"#;
    let scrubbed = sanitize_summary_for_export(input, &workspace_root);
    assert!(!scrubbed.contains(r"C:\root\proj"));
    assert!(!scrubbed.contains(r"C:\\root\\proj"));
    assert!(scrubbed.contains("workspace-root"));
}

#[test]
fn sanitize_workspace_path_rejects_parent_traversal() {
    let workspace_root = PathBuf::from(r"C:\root\proj");
    let traversing = workspace_root.join(r"..\secret.txt");
    let sanitized = sanitize_workspace_path(&traversing, &workspace_root, "redacted-artifact");
    assert_eq!(sanitized, PathBuf::from("redacted-artifact"));
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
    assert!(
        temp_dir
            .path()
            .join(".codex")
            .join("team-governance")
            .join("prompts")
            .join("scheduler.md")
            .exists()
    );
    assert!(
        temp_dir
            .path()
            .join(".codex")
            .join("skills")
            .join("team-delegation")
            .join("SKILL.md")
            .exists()
    );
    let root_doc = tokio::fs::read_to_string(
        temp_dir
            .path()
            .join(".codex")
            .join(GLOBAL_AGENT_DOC_FILENAME),
    )
    .await
    .expect("read root agent doc");
    assert!(root_doc.contains(".codex/team-governance/prompts/scheduler.md"));
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
            agent_path: None,
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
            agent_path: None,
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
            agent_path: None,
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
    assert!(!manifest.contains(&child_thread_id.to_string()));
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
            agent_path: None,
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
async fn public_team_session_exposes_root_only_lifecycle_summary() {
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
            agent_path: None,
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
    let handoff_doc = tokio::fs::read_to_string(&prepared.artifact_refs[0])
        .await
        .expect("read sanitized handoff");
    assert!(!handoff_doc.contains("private implementation details"));
    assert!(!handoff_doc.contains(&child_thread_id.to_string()));
    let err =
        record_team_message_delivery(temp_dir.path(), child_thread_id, root_thread_id, &prepared)
            .await
            .expect_err("development handoff should require review evidence");
    assert!(err.to_string().contains("reviewRequired"));
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
    assert_eq!(
        session.memory_provider.mode,
        TeamWorkflowPublicMemoryProviderMode::Local
    );
    assert_eq!(
        session.memory_provider.health,
        TeamWorkflowPublicMemoryProviderHealth::Ready
    );
    assert_eq!(session.root_agent.agent_id, "root-scheduler");
    assert_eq!(
        session.lifecycle.state,
        TeamWorkflowPublicLifecycleState::Blocked
    );
    assert_eq!(session.handoff.trace_group_id, root_thread_id.to_string());
    assert_eq!(session.handoff.active_delegate_count, 1);
    assert_eq!(session.handoff.blocked_delegate_count, 1);
    assert_eq!(session.handoff.awaiting_review_count, 1);
    assert_eq!(session.handoff.integration_ready_count, 0);
    let session_json = serde_json::to_string(&session).expect("serialize session");
    assert!(!session_json.contains(&child_thread_id.to_string()));
    assert!(!session_json.contains("private implementation details"));
}

#[tokio::test]
async fn configured_tape_provider_surfaces_degraded_status_without_blocking_runtime() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_workflow(
        &temp_dir,
        "version: 1\nmemoryProvider:\n  mode: tape\n  tape:\n    endpoint: http://127.0.0.1:9/team-memory\n",
    )
    .await;
    let root_thread_id = ThreadId::new();

    maybe_initialize_for_thread(temp_dir.path(), root_thread_id, &SessionSource::Exec, None)
        .await
        .expect("initialize root team with tape provider");

    let session = load_public_team_workflow_session(temp_dir.path(), root_thread_id, 8)
        .await
        .expect("load public session")
        .expect("session exists");

    assert_eq!(
        session.memory_provider.mode,
        TeamWorkflowPublicMemoryProviderMode::Tape
    );
    assert_eq!(
        session.memory_provider.health,
        TeamWorkflowPublicMemoryProviderHealth::Degraded
    );
}

#[tokio::test]
async fn sibling_peer_messages_require_structured_a2a_and_persist_peer_shape() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_workflow(&temp_dir, "version: 1\n").await;
    let root_thread_id = ThreadId::new();
    let design_thread_id = ThreadId::new();
    let development_thread_id = ThreadId::new();

    maybe_initialize_for_thread(temp_dir.path(), root_thread_id, &SessionSource::Exec, None)
        .await
        .expect("initialize root team");
    maybe_initialize_for_thread(
        temp_dir.path(),
        design_thread_id,
        &SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id: root_thread_id,
            depth: 1,
            agent_path: None,
            agent_nickname: Some("Ada".to_string()),
            agent_role: Some("design-lead".to_string()),
        }),
        None,
    )
    .await
    .expect("initialize design team");
    maybe_initialize_for_thread(
        temp_dir.path(),
        development_thread_id,
        &SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id: root_thread_id,
            depth: 1,
            agent_path: None,
            agent_nickname: Some("Linus".to_string()),
            agent_role: Some("development-lead".to_string()),
        }),
        None,
    )
    .await
    .expect("initialize development team");

    let raw_err = prepare_team_message(
        temp_dir.path(),
        design_thread_id,
        development_thread_id,
        vec![UserInput::Text {
            text: "please align on the API".to_string(),
            text_elements: Vec::new(),
        }],
    )
    .await
    .expect_err("raw same-level payload should be rejected");
    assert!(raw_err.to_string().contains("A2A"));

    let prepared = prepare_team_message(
        temp_dir.path(),
        design_thread_id,
        development_thread_id,
        vec![UserInput::Text {
            text: "protocol: codex-a2a\nintent: align\nphase: design\nsummary: Align the response schema and keep the handoff bounded.\nartifact_refs:\n- docs/interface.md\nreply_needed: true".to_string(),
            text_elements: Vec::new(),
        }],
    )
    .await
    .expect("prepare a2a payload");
    let UserInput::Text { text, .. } = &prepared.items[0] else {
        panic!("expected rendered a2a text");
    };
    assert!(text.contains("protocol: codex-a2a"));
    record_team_message_delivery(
        temp_dir.path(),
        design_thread_id,
        development_thread_id,
        &prepared,
    )
    .await
    .expect("record a2a delivery");

    let tape = std::fs::read_to_string(
        temp_dir
            .path()
            .join(".codex")
            .join("team-state")
            .join(design_thread_id.to_string())
            .join(TEAM_TAPE_FILENAME),
    )
    .expect("read tape");
    let last_entry: Value = serde_json::from_str(
        tape.lines()
            .filter(|line| !line.trim().is_empty())
            .next_back()
            .expect("last tape line"),
    )
    .expect("parse tape entry");
    assert_eq!(last_entry["kind"], "peer_sync");
    assert_eq!(last_entry["peer_message"]["protocol"], "codex-a2a");
    assert_eq!(last_entry["peer_message"]["intent"], "align");
}

#[tokio::test]
async fn sibling_a2a_artifact_refs_reject_parent_traversal() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_workflow(&temp_dir, "version: 1\n").await;
    let root_thread_id = ThreadId::new();
    let design_thread_id = ThreadId::new();
    let development_thread_id = ThreadId::new();

    maybe_initialize_for_thread(temp_dir.path(), root_thread_id, &SessionSource::Exec, None)
        .await
        .expect("initialize root team");
    maybe_initialize_for_thread(
        temp_dir.path(),
        design_thread_id,
        &SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id: root_thread_id,
            depth: 1,
            agent_path: None,
            agent_nickname: Some("Ada".to_string()),
            agent_role: Some("design-lead".to_string()),
        }),
        None,
    )
    .await
    .expect("initialize design team");
    maybe_initialize_for_thread(
        temp_dir.path(),
        development_thread_id,
        &SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id: root_thread_id,
            depth: 1,
            agent_path: None,
            agent_nickname: Some("Linus".to_string()),
            agent_role: Some("development-lead".to_string()),
        }),
        None,
    )
    .await
    .expect("initialize development team");

    let prepared = prepare_team_message(
        temp_dir.path(),
        design_thread_id,
        development_thread_id,
        vec![UserInput::Text {
            text: "protocol: codex-a2a\nintent: align\nphase: design\nsummary: bounded artifact sync\nartifact_refs:\n- ..\\\\secret.txt\nreply_needed: true".to_string(),
            text_elements: Vec::new(),
        }],
    )
    .await
    .expect("prepare a2a payload");

    let envelope = prepared.a2a_envelope.as_ref().expect("a2a envelope");
    assert_eq!(
        envelope.artifact_refs,
        vec![PathBuf::from("redacted-artifact")]
    );

    record_team_message_delivery(
        temp_dir.path(),
        design_thread_id,
        development_thread_id,
        &prepared,
    )
    .await
    .expect("record delivery");

    let tape = std::fs::read_to_string(
        temp_dir
            .path()
            .join(".codex")
            .join("team-state")
            .join(design_thread_id.to_string())
            .join(TEAM_TAPE_FILENAME),
    )
    .expect("read tape");
    assert!(!tape.contains("..\\\\secret.txt"));
}

#[tokio::test]
async fn vertical_a2a_payload_is_rejected_with_artifact_guidance() {
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
            agent_path: None,
            agent_nickname: Some("Ada".to_string()),
            agent_role: Some("design-lead".to_string()),
        }),
        None,
    )
    .await
    .expect("initialize child team");

    let err = prepare_team_message(
        temp_dir.path(),
        child_thread_id,
        root_thread_id,
        vec![UserInput::Text {
            text: "protocol: codex-a2a\nintent: align\nsummary: send context upward".to_string(),
            text_elements: Vec::new(),
        }],
    )
    .await
    .expect_err("vertical a2a should be rejected");
    assert!(err.to_string().contains("vertical artifact handoff"));
}

#[tokio::test]
async fn vertical_handoff_allows_json_payload_without_a2a_protocol() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_workflow(&temp_dir, "version: 1\n").await;
    let root_thread_id = ThreadId::new();
    let child_thread_id = ThreadId::new();
    let workspace_root = temp_dir.path().display().to_string();
    let escaped_workspace_root = workspace_root.replace('\\', "\\\\").replace('"', "\\\"");

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
            agent_role: Some("design-lead".to_string()),
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
            text: format!(
                "{{\"summary\":\"handoff from {} about auth timeout\",\"details\":{{\"intent\":\"fix auth timeout\"}}}}",
                escaped_workspace_root
            ),
            text_elements: Vec::new(),
        }],
    )
    .await
    .expect("plain json handoff should remain valid");

    assert!(prepared.integration_handoff.is_some());
    assert!(prepared.a2a_envelope.is_none());
    assert!(
        !prepared.summary.contains(&child_thread_id.to_string()),
        "vertical handoff summary should remain sanitized"
    );
    assert!(
        !prepared.summary.contains(&workspace_root),
        "vertical handoff summary should not expose the workspace root"
    );
    assert!(
        prepared.summary.contains("workspace-root"),
        "vertical handoff summary should redact the workspace root"
    );
    let handoff_doc = tokio::fs::read_to_string(&prepared.artifact_refs[0])
        .await
        .expect("read sanitized handoff");
    assert!(
        !handoff_doc.contains(&workspace_root),
        "vertical handoff artifact should not expose the workspace root"
    );
}

#[tokio::test]
async fn development_handoff_requires_review_evidence_before_integration_ready() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    init_git_repo(&temp_dir);
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
            agent_path: None,
            agent_nickname: Some("Linus".to_string()),
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
        root_thread_id,
        vec![UserInput::Text {
            text: "ready for parent review".to_string(),
            text_elements: Vec::new(),
        }],
    )
    .await
    .expect("prepare handoff");
    let err =
        record_team_message_delivery(temp_dir.path(), child_thread_id, root_thread_id, &prepared)
            .await
            .expect_err("review gate should reject development finalize");
    assert!(err.to_string().contains("reviewRequired"));

    let updated_bundle = load_team_state_bundle(temp_dir.path(), &child_thread_id.to_string())
        .await
        .expect("reload child bundle")
        .expect("child bundle exists");
    assert_eq!(updated_bundle.status.current_phase, TeamPhase::Review);
    assert!(
        updated_bundle
            .status
            .blockers
            .iter()
            .any(|entry| entry.contains("Review evidence"))
    );
}

#[tokio::test]
async fn compact_checkpoint_and_resume_enforce_artifact_first_recovery() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_workflow(&temp_dir, "version: 1\n").await;
    let root_thread_id = ThreadId::new();

    maybe_initialize_for_thread(temp_dir.path(), root_thread_id, &SessionSource::Exec, None)
        .await
        .expect("initialize root team");

    let mut bundle = load_team_state_bundle(temp_dir.path(), &root_thread_id.to_string())
        .await
        .expect("load bundle")
        .expect("bundle exists");
    bundle.status.blockers = vec!["checkpoint blocker".to_string()];
    bundle.status.next_steps = vec!["persist compact marker".to_string()];
    write_team_state_bundle(&bundle)
        .await
        .expect("write bundle");

    record_team_compact_checkpoint(temp_dir.path(), &root_thread_id.to_string())
        .await
        .expect("record compact checkpoint");

    let compacted_bundle = load_team_state_bundle(temp_dir.path(), &root_thread_id.to_string())
        .await
        .expect("reload compacted bundle")
        .expect("bundle exists");
    assert!(
        compacted_bundle
            .recovery
            .last_compact_checkpoint_at
            .is_some()
    );
    assert_eq!(
        compacted_bundle.recovery.blockers,
        vec!["checkpoint blocker"]
    );

    tokio::fs::remove_file(&compacted_bundle.paths.tape_path)
        .await
        .expect("remove tape");
    let resume_err = record_team_resume(temp_dir.path(), &root_thread_id.to_string())
        .await
        .expect_err("resume should require persisted artifacts");
    assert!(resume_err.to_string().contains("resumeFromArtifacts"));
    assert!(resume_err.to_string().contains(TEAM_TAPE_FILENAME));
}
