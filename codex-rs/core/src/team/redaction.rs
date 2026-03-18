use super::state::TeamKind;
use codex_protocol::user_input::UserInput;
use sha2::Digest;
use sha2::Sha256;
use std::path::Path;
use std::path::PathBuf;

const MAX_SUMMARY_LEN: usize = 180;

pub(crate) fn public_team_ref(team_id: &str, role: &str, depth: i32, kind: TeamKind) -> String {
    if matches!(kind, TeamKind::Root) {
        return "root-scheduler".to_string();
    }
    let mut hasher = Sha256::new();
    hasher.update(team_id.as_bytes());
    let digest = hasher.finalize();
    let suffix = format!(
        "{:02x}{:02x}{:02x}{:02x}",
        digest[0], digest[1], digest[2], digest[3]
    );
    let role_slug = slugify(role);
    format!("team-d{depth}-{role_slug}-{suffix}")
}

pub(crate) fn sanitize_summary_text(text: &str) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return "Sanitized artifact handoff.".to_string();
    }
    if trimmed.len() <= MAX_SUMMARY_LEN {
        return trimmed.to_string();
    }
    let mut clipped = trimmed[..MAX_SUMMARY_LEN].trim_end().to_string();
    clipped.push_str("...");
    clipped
}

pub(crate) fn sanitize_user_input_summary(items: &[UserInput]) -> String {
    let mut parts = Vec::new();
    for item in items {
        let part = match item {
            UserInput::Text { text, .. } => sanitize_summary_text(text),
            UserInput::Mention { name, .. } => format!("mention `{name}` included"),
            UserInput::Skill { name, .. } => format!("skill `{name}` referenced"),
            UserInput::LocalImage { .. } => "local image supplied".to_string(),
            UserInput::Image { .. } => "image supplied".to_string(),
            _ => "structured input supplied".to_string(),
        };
        if !part.is_empty() {
            parts.push(part);
        }
    }
    if parts.is_empty() {
        "Sanitized artifact handoff.".to_string()
    } else {
        sanitize_summary_text(parts.join(" | ").as_str())
    }
}

pub(crate) fn sanitize_workspace_path(
    path: &Path,
    workspace_root: &Path,
    fallback: &str,
) -> PathBuf {
    match path.strip_prefix(workspace_root) {
        Ok(relative) => relative.to_path_buf(),
        Err(_) => PathBuf::from(fallback),
    }
}

pub(crate) fn sanitize_workspace_paths(
    paths: &[PathBuf],
    workspace_root: &Path,
    fallback: &str,
) -> Vec<PathBuf> {
    let mut sanitized = Vec::new();
    for path in paths {
        let safe = sanitize_workspace_path(path, workspace_root, fallback);
        if !sanitized.iter().any(|existing| existing == &safe) {
            sanitized.push(safe);
        }
    }
    sanitized
}

pub(crate) fn vertical_receiver_label(
    sender_parent_team_id: Option<&str>,
    receiver_team_id: Option<&str>,
) -> &'static str {
    match receiver_team_id {
        Some(receiver_id) if sender_parent_team_id == Some(receiver_id) => "parent",
        Some(_) => "child",
        None => "child",
    }
}

pub(crate) fn public_worktree_label(public_team_id: &str, managed: bool) -> PathBuf {
    if managed {
        PathBuf::from(format!("worktrees/{public_team_id}"))
    } else {
        PathBuf::from("workspace-root")
    }
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in value.chars() {
        let next = if ch.is_ascii_alphanumeric() {
            last_dash = false;
            ch.to_ascii_lowercase()
        } else if !last_dash {
            last_dash = true;
            '-'
        } else {
            continue;
        };
        slug.push(next);
    }
    slug.trim_matches('-')
        .to_string()
        .chars()
        .take(24)
        .collect()
}
