//! Agent IPC commands: start a streaming chat turn and cancel an in-flight one.

use crate::agent::runner;
use crate::error::AppResult;
use crate::model::ChatMessage;
use crate::state::AppState;
use tauri::State;

/// Start a streaming agent turn. Emits `AgentEvent`s on
/// `agent://event/{session_id}` until the model finishes or is cancelled.
///
/// State is read out (settings) and a cancel flag registered *before* any
/// `.await`, so the non-`Send` `State` guard is never held across awaits.
#[tauri::command]
pub async fn agent_chat(
    session_id: String,
    messages: Vec<ChatMessage>,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let settings = state.settings.lock().unwrap().clone();
    let cancel = state.new_cancel(&session_id);

    let result = runner::run_chat(session_id.clone(), messages, settings, app, cancel).await;

    state.clear_cancel(&session_id);
    result
}

/// Signal cancellation for an in-flight agent turn.
#[tauri::command]
pub fn agent_cancel(session_id: String, state: State<AppState>) -> AppResult<()> {
    state.cancel(&session_id);
    Ok(())
}
