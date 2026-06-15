//! Shared application state held by Tauri's managed state.

use crate::model::{AppSettings, ScanResult};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// Global app state. Cloneable handles are cheap (Arc inside).
#[derive(Default)]
pub struct AppState {
    pub settings: Mutex<AppSettings>,
    /// Cancellation flags keyed by scan_id / session_id.
    pub cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
    /// Most recent scan result, so the agent can query it without rescanning.
    pub last_scan: Mutex<Option<ScanResult>>,
}

impl AppState {
    /// Register (or reset) a cancellation flag for an id and return it.
    pub fn new_cancel(&self, id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        self.cancels
            .lock()
            .unwrap()
            .insert(id.to_string(), flag.clone());
        flag
    }

    /// Signal cancellation for an id, if registered.
    pub fn cancel(&self, id: &str) {
        if let Some(flag) = self.cancels.lock().unwrap().get(id) {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    /// Remove a finished cancellation flag.
    pub fn clear_cancel(&self, id: &str) {
        self.cancels.lock().unwrap().remove(id);
    }
}
