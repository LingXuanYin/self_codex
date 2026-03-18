use std::sync::Arc;

use super::SessionTask;
use super::SessionTaskContext;
use crate::codex::TurnContext;
use crate::state::TaskKind;
use crate::team::record_team_compact_checkpoint;
use async_trait::async_trait;
use codex_protocol::user_input::UserInput;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Default)]
pub(crate) struct CompactTask;

#[async_trait]
impl SessionTask for CompactTask {
    fn kind(&self) -> TaskKind {
        TaskKind::Compact
    }

    fn span_name(&self) -> &'static str {
        "session_task.compact"
    }

    async fn run(
        self: Arc<Self>,
        session: Arc<SessionTaskContext>,
        ctx: Arc<TurnContext>,
        input: Vec<UserInput>,
        _cancellation_token: CancellationToken,
    ) -> Option<String> {
        let session = session.clone_session();
        if let Err(err) =
            record_team_compact_checkpoint(&ctx.cwd, &session.conversation_id.to_string()).await
        {
            return Some(format!(
                "Team workflow blocked compact until the persisted checkpoint succeeded: {err}"
            ));
        }
        let _ = if crate::compact::should_use_remote_compact_task(&ctx.provider) {
            let _ = session.services.session_telemetry.counter(
                "codex.task.compact",
                /*inc*/ 1,
                &[("type", "remote")],
            );
            crate::compact_remote::run_remote_compact_task(session.clone(), ctx).await
        } else {
            let _ = session.services.session_telemetry.counter(
                "codex.task.compact",
                /*inc*/ 1,
                &[("type", "local")],
            );
            crate::compact::run_compact_task(session.clone(), ctx, input).await
        };
        None
    }
}
