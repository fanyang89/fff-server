use std::path::{Path, PathBuf};
use std::sync::Arc;

use fff_search::{
    file_picker::{FFFMode, FilePicker, FilePickerOptions},
    frecency::FrecencyTracker,
    query_tracker::QueryTracker,
    SharedFilePicker, SharedFrecency, SharedQueryTracker,
};

use crate::config::Config;
use crate::error::{AppError, Result};

/// Shared application state injected into every handler.
#[derive(Clone)]
pub struct AppState {
    pub picker: SharedFilePicker,
    pub frecency: SharedFrecency,
    pub query_tracker: SharedQueryTracker,
    pub base_path: Arc<PathBuf>,
    pub mode: FFFMode,
    pub max_results: usize,
}

impl AppState {
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}

/// Initialize the fff engine (frecency DB, query tracker DB, file picker scan)
/// and return the shared application state.
pub fn init_state(cfg: &Config) -> Result<AppState> {
    let base_path = cfg
        .base_path
        .canonicalize()
        .map_err(|e| AppError::BadRequest(format!("base_path invalid: {e}")))?;

    let db_dir = cfg.resolved_db_dir();
    std::fs::create_dir_all(&db_dir)?;

    let shared_picker = SharedFilePicker::default();
    let shared_frecency = SharedFrecency::default();
    let shared_query_tracker = SharedQueryTracker::default();

    let frecency = FrecencyTracker::open(db_dir.join("frecency"))?;
    shared_frecency.init(frecency)?;

    let query_tracker = QueryTracker::open(db_dir.join("queries"))?;
    shared_query_tracker.init(query_tracker)?;

    let mode = if cfg.ai_mode {
        FFFMode::Ai
    } else {
        FFFMode::Neovim
    };

    let options = FilePickerOptions {
        base_path: base_path.display().to_string(),
        mode,
        watch: cfg.watch,
        enable_mmap_cache: cfg.mmap_cache,
        enable_content_indexing: cfg.content_indexing,
        ..Default::default()
    };

    FilePicker::new_with_shared_state(shared_picker.clone(), shared_frecency.clone(), options)?;

    tracing::info!(base_path = %base_path.display(), "file picker initialized");

    Ok(AppState {
        picker: shared_picker,
        frecency: shared_frecency,
        query_tracker: shared_query_tracker,
        base_path: Arc::new(base_path),
        mode,
        max_results: cfg.max_results,
    })
}

/// Block until the initial scan finishes or the timeout elapses.
pub fn wait_for_scan(state: &AppState, timeout: std::time::Duration) -> bool {
    state.picker.wait_for_scan(timeout)
}
