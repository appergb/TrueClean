//! Agent IPC commands: start a streaming chat turn, cancel an in-flight one,
//! and resolve a pending destructive-tool confirmation.

use crate::agent::runner;
use crate::error::AppResult;
use crate::model::ChatMessage;
use crate::state::AppState;
use tauri::State;

/// Start a streaming agent turn. Emits `AgentEvent`s on
/// `agent://event/{session_id}` until the model finishes or is cancelled.
///
/// `scan_target` 是用户已扫描确认的工作目录根路径，会注入系统提示词约束
/// agent 的所有文件操作在此路径内。前端从 scanStore.scanTarget 传入。
///
/// State is read out (settings) and a cancel flag registered *before* any
/// `.await`, so the non-`Send` `State` guard is never held across awaits.
#[tauri::command]
pub async fn agent_chat(
    session_id: String,
    messages: Vec<ChatMessage>,
    scan_target: Option<String>,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let settings = state
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let cancel = state.new_cancel(&session_id);

    let result = runner::run_chat(session_id.clone(), messages, settings, scan_target, app, cancel).await;

    state.clear_cancel(&session_id);
    result
}

/// Signal cancellation for an in-flight agent turn.
#[tauri::command]
pub fn agent_cancel(session_id: String, state: State<AppState>) -> AppResult<()> {
    state.cancel(&session_id);
    Ok(())
}

/// Resolve a pending destructive-tool confirmation. The frontend invokes this
/// after receiving a `ConfirmationRequest` event; `approved=true` lets the
/// tool proceed, `false` skips it. Returns `true` if the confirmation id was
/// found and resolved (i.e. the session was still waiting).
///
/// This is the command-based counterpart to the `agent://confirm` event
/// listener; either path routes to [`runner::resolve_confirmation`].
#[tauri::command]
pub fn agent_confirm(confirmation_id: String, approved: bool) -> AppResult<bool> {
    Ok(runner::resolve_confirmation(&confirmation_id, approved))
}
