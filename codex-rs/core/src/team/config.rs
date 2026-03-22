use crate::git_info::resolve_root_git_project_for_trust;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeSet;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use tokio::fs;

pub(crate) const TEAM_DIRNAME: &str = ".codex";
pub(crate) const TEAM_WORKFLOW_FILENAME: &str = "team-workflow.yaml";
pub(crate) const TEAM_STATE_DIRNAME: &str = "team-state";
pub(crate) const GLOBAL_AGENT_DOC_FILENAME: &str = "AGENT.md";
pub(crate) const TEAM_AGENT_DOC_FILENAME: &str = "AGENT_TEAM.md";
pub(crate) const TEAM_ARTIFACTS_DIRNAME: &str = "artifacts";

const DEFAULT_WORKFLOW_VERSION: u32 = 1;
const DEFAULT_TEAM_MAX_DEPTH: i32 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TeamWorkflowConfig {
    #[serde(default = "default_workflow_version")]
    pub version: u32,
    #[serde(default)]
    pub root_scheduler: RootSchedulerConfig,
    #[serde(default)]
    pub decision_policy: DecisionPolicyConfig,
    #[serde(default)]
    pub workflow_loop: WorkflowLoopConfig,
    #[serde(default)]
    pub governance: GovernanceConfig,
    #[serde(default)]
    pub artifact_policy: ArtifactPolicyConfig,
    #[serde(default)]
    pub memory_provider: TeamMemoryProviderConfig,
    #[serde(default)]
    pub team_templates: Vec<TeamTemplateConfig>,
    #[serde(default = "default_team_max_depth", alias = "max-depth")]
    pub max_depth: i32,
}

impl Default for TeamWorkflowConfig {
    fn default() -> Self {
        Self {
            version: DEFAULT_WORKFLOW_VERSION,
            root_scheduler: RootSchedulerConfig::default(),
            decision_policy: DecisionPolicyConfig::default(),
            workflow_loop: WorkflowLoopConfig::default(),
            governance: GovernanceConfig::default(),
            artifact_policy: ArtifactPolicyConfig::default(),
            memory_provider: TeamMemoryProviderConfig::default(),
            team_templates: Vec::new(),
            max_depth: DEFAULT_TEAM_MAX_DEPTH,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RootSchedulerConfig {
    #[serde(default = "default_root_scheduler_role")]
    pub role: String,
    #[serde(default = "default_root_scheduler_owner")]
    pub charter_owner: String,
}

impl Default for RootSchedulerConfig {
    fn default() -> Self {
        Self {
            role: default_root_scheduler_role(),
            charter_owner: default_root_scheduler_owner(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DecisionPolicyConfig {
    #[serde(default = "default_execution_modes")]
    pub allowed_modes: Vec<ExecutionMode>,
    #[serde(default = "default_true")]
    pub single_writer: bool,
    #[serde(default = "default_true")]
    pub atomic_workflows: bool,
}

impl Default for DecisionPolicyConfig {
    fn default() -> Self {
        Self {
            allowed_modes: default_execution_modes(),
            single_writer: true,
            atomic_workflows: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExecutionMode {
    Single,
    Delegate,
    Parallel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowLoopConfig {
    #[serde(default = "default_iteration_roles")]
    pub required_roles: Vec<IterationRole>,
    #[serde(default = "default_true")]
    pub review_required: bool,
    #[serde(default = "default_true")]
    pub persist_before_compact: bool,
    #[serde(default = "default_true")]
    pub resume_from_artifacts: bool,
}

impl Default for WorkflowLoopConfig {
    fn default() -> Self {
        Self {
            required_roles: default_iteration_roles(),
            review_required: true,
            persist_before_compact: true,
            resume_from_artifacts: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IterationRole {
    Design,
    Development,
    Review,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GovernanceConfig {
    #[serde(default = "default_governance_triggers")]
    pub update_triggers: Vec<GovernanceTrigger>,
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        Self {
            update_triggers: default_governance_triggers(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GovernanceTrigger {
    TeamCreated,
    Replan,
    ReviewHandoff,
    Compact,
    LeaderResume,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArtifactPolicyConfig {
    #[serde(default)]
    pub same_level_context_protocol: SameLevelContextProtocol,
    #[serde(default)]
    pub cross_level_handoff: CrossLevelHandoffPolicy,
    #[serde(default = "default_artifact_directory")]
    pub artifact_directory: String,
}

impl Default for ArtifactPolicyConfig {
    fn default() -> Self {
        Self {
            same_level_context_protocol: SameLevelContextProtocol::default(),
            cross_level_handoff: CrossLevelHandoffPolicy::default(),
            artifact_directory: default_artifact_directory(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SameLevelContextProtocol {
    #[default]
    A2a,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CrossLevelHandoffPolicy {
    #[default]
    #[serde(rename = "openspec-artifacts", alias = "open-spec-artifacts")]
    OpenSpecArtifacts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TeamTemplateConfig {
    pub id: String,
    #[serde(default)]
    pub leader_role: Option<String>,
    #[serde(default)]
    pub default_mode: Option<ExecutionMode>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TeamMemoryProviderMode {
    #[default]
    Local,
    Tape,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TeamMemoryProviderConfig {
    #[serde(default)]
    pub mode: TeamMemoryProviderMode,
    #[serde(default)]
    pub tape: Option<TeamTapeProviderConfig>,
}

impl Default for TeamMemoryProviderConfig {
    fn default() -> Self {
        Self {
            mode: TeamMemoryProviderMode::Local,
            tape: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TeamTapeProviderConfig {
    pub endpoint: String,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
}

impl TeamWorkflowConfig {
    pub(crate) fn validate(&self) -> io::Result<()> {
        if self.version != DEFAULT_WORKFLOW_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "unsupported team workflow version {}; expected {DEFAULT_WORKFLOW_VERSION}",
                    self.version
                ),
            ));
        }

        if self.max_depth < 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "team workflow maxDepth must be at least 1",
            ));
        }

        if self.root_scheduler.role.trim().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "rootScheduler.role must not be empty",
            ));
        }

        if self.root_scheduler.charter_owner.trim().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "rootScheduler.charterOwner must not be empty",
            ));
        }

        validate_unique_slice(
            &self.decision_policy.allowed_modes,
            "decisionPolicy.allowedModes contains duplicate entries",
        )?;

        let required_roles: BTreeSet<_> =
            self.workflow_loop.required_roles.iter().copied().collect();
        for required in [
            IterationRole::Design,
            IterationRole::Development,
            IterationRole::Review,
        ] {
            if !required_roles.contains(&required) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "workflowLoop.requiredRoles must include design, development, and review",
                ));
            }
        }
        validate_unique_slice(
            &self.workflow_loop.required_roles,
            "workflowLoop.requiredRoles contains duplicate entries",
        )?;
        validate_unique_slice(
            &self.governance.update_triggers,
            "governance.updateTriggers contains duplicate entries",
        )?;

        let artifact_directory = self.artifact_policy.artifact_directory.trim();
        if artifact_directory.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "artifactPolicy.artifactDirectory must not be empty",
            ));
        }
        if Path::new(artifact_directory).is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "artifactPolicy.artifactDirectory must be relative to the team state directory",
            ));
        }

        let mut template_ids = BTreeSet::new();
        for template in &self.team_templates {
            let id = template.id.trim();
            if id.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "teamTemplates entries must include a non-empty id",
                ));
            }
            if !template_ids.insert(id.to_string()) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("teamTemplates contains a duplicate id: {id}"),
                ));
            }
        }

        if self.memory_provider.mode == TeamMemoryProviderMode::Tape {
            let Some(tape) = self.memory_provider.tape.as_ref() else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "memoryProvider.tape must be configured when memoryProvider.mode is tape",
                ));
            };
            if tape.endpoint.trim().is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "memoryProvider.tape.endpoint must not be empty when tape mode is enabled",
                ));
            }
        }

        Ok(())
    }
}

pub(crate) fn workflow_path(workspace_root: &Path) -> PathBuf {
    resolve_team_home_root(workspace_root)
        .join(TEAM_DIRNAME)
        .join(TEAM_WORKFLOW_FILENAME)
}

pub(crate) fn resolve_team_home_root(workspace_root: &Path) -> PathBuf {
    let base = if workspace_root.is_dir() {
        workspace_root
    } else {
        workspace_root.parent().unwrap_or(workspace_root)
    };

    if let Some(team_root) = find_team_home_ancestor(base) {
        return team_root;
    }

    if let Some(git_root) = resolve_root_git_project_for_trust(base) {
        if let Some(team_root) = find_team_home_ancestor(&git_root) {
            return team_root;
        }
        return git_root;
    }

    base.to_path_buf()
}

pub(crate) async fn load_workflow_from_workspace(
    workspace_root: &Path,
) -> io::Result<Option<TeamWorkflowConfig>> {
    let workflow_path = workflow_path(workspace_root);
    let contents = match fs::read_to_string(&workflow_path).await {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };

    let workflow: TeamWorkflowConfig = serde_yaml::from_str(&contents).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to parse {}: {err}", workflow_path.display()),
        )
    })?;
    workflow.validate()?;
    Ok(Some(workflow))
}

pub(crate) async fn resolve_team_max_depth(
    workspace_root: &Path,
    fallback: i32,
) -> io::Result<i32> {
    Ok(load_workflow_from_workspace(workspace_root)
        .await?
        .map(|workflow| workflow.max_depth)
        .unwrap_or(fallback))
}

fn find_team_home_ancestor(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(path) = current {
        if path
            .join(TEAM_DIRNAME)
            .join(TEAM_WORKFLOW_FILENAME)
            .is_file()
        {
            return Some(path.to_path_buf());
        }
        current = path.parent();
    }
    None
}

fn validate_unique_slice<T>(items: &[T], message: &str) -> io::Result<()>
where
    T: Clone + Ord,
{
    if items.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, message));
    }
    let unique: BTreeSet<_> = items.iter().cloned().collect();
    if unique.len() != items.len() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, message));
    }
    Ok(())
}

const fn default_workflow_version() -> u32 {
    DEFAULT_WORKFLOW_VERSION
}

const fn default_team_max_depth() -> i32 {
    DEFAULT_TEAM_MAX_DEPTH
}

fn default_root_scheduler_role() -> String {
    "root-scheduler".to_string()
}

fn default_root_scheduler_owner() -> String {
    "root-scheduler".to_string()
}

fn default_execution_modes() -> Vec<ExecutionMode> {
    vec![
        ExecutionMode::Single,
        ExecutionMode::Delegate,
        ExecutionMode::Parallel,
    ]
}

fn default_iteration_roles() -> Vec<IterationRole> {
    vec![
        IterationRole::Design,
        IterationRole::Development,
        IterationRole::Review,
    ]
}

fn default_governance_triggers() -> Vec<GovernanceTrigger> {
    vec![
        GovernanceTrigger::TeamCreated,
        GovernanceTrigger::Replan,
        GovernanceTrigger::ReviewHandoff,
        GovernanceTrigger::Compact,
        GovernanceTrigger::LeaderResume,
    ]
}

fn default_artifact_directory() -> String {
    TEAM_ARTIFACTS_DIRNAME.to_string()
}

const fn default_true() -> bool {
    true
}
