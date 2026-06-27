use axum::extract::{Query, State};
use axum::Json;
use fff_search::file_picker::FuzzySearchOptions;
use fff_search::types::{MixedItemRef, SearchResult};
use fff_search::{FileItem, FilePicker, FFFQuery, PaginationArgs, QueryParser};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::dto::{FileItemDto, SearchResponse};
use crate::error::{AppError, Result};
use crate::state::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct SearchParams {
    /// Query string. Supports fff constraint syntax, e.g. `*.rs !test/ git:modified`.
    pub q: String,
    /// Maximum results to return.
    #[param(default = 100)]
    pub limit: Option<usize>,
    /// Pagination offset (0-based).
    #[param(default = 0)]
    pub offset: Option<usize>,
    /// Search target: `files` (default), `dirs`, or `mixed`.
    #[param(default = "files")]
    pub mode: Option<String>,
    /// Relative path of the currently open file (deprioritized in scoring).
    pub current_file: Option<String>,
}

/// Fuzzy file / directory search.
#[utoipa::path(
    get,
    path = "/api/search",
    tag = "search",
    params(SearchParams),
    responses(
        (status = 200, description = "Search results", body = SearchResponse),
        (status = 400, description = "Bad request", body = serde_json::Value),
        (status = 500, description = "Internal error", body = serde_json::Value),
    )
)]
pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResponse>> {
    let mode = parse_mode(params.mode.as_deref())?;
    let limit = clamp_limit(params.limit, state.max_results);
    let offset = params.offset.unwrap_or(0);
    let query = params.q;
    let current_file = params.current_file;
    let picker = state.picker.clone();
    let tracker = state.query_tracker.clone();

    let resp = tokio::task::spawn_blocking(move || -> Result<SearchResponse> {
        let guard = picker.read()?;
        let picker_ref = guard
            .as_ref()
            .ok_or(AppError::Internal("file picker not ready".into()))?;
        let parser = QueryParser::default();
        let parsed: FFFQuery<'_> = parser.parse(&query);

        let options = FuzzySearchOptions {
            max_threads: 0,
            current_file: current_file.as_deref(),
            project_path: None,
            combo_boost_score_multiplier: 100,
            min_combo_count: 3,
            pagination: PaginationArgs { offset, limit },
        };

        let base_path = picker_ref.base_path();
        match mode {
            Mode::Files => {
                let qt_guard = tracker.read().ok();
                let qt = qt_guard.as_ref().and_then(|g| g.as_ref());
                let res = picker_ref.fuzzy_search(&parsed, qt, options);
                Ok(build_response_files(&res, picker_ref, base_path))
            }
            Mode::Dirs => {
                let res = picker_ref.fuzzy_search_directories(&parsed, options);
                let items = res
                    .items
                    .iter()
                    .zip(res.scores.iter())
                    .map(|(d, s)| FileItemDto {
                        kind: "directory".into(),
                        name: basename(&d.relative_path(picker_ref)),
                        relative_path: d.relative_path(picker_ref),
                        absolute_path: d
                            .absolute_path(picker_ref, base_path)
                            .to_string_lossy()
                            .into_owned(),
                        size: None,
                        modified: None,
                        git_status: None,
                        is_binary: None,
                        score: Some(s.total),
                    })
                    .collect::<Vec<_>>();
                Ok(SearchResponse {
                    total_matched: res.total_matched,
                    total_files: 0,
                    total_dirs: Some(res.total_dirs),
                    items,
                })
            }
            Mode::Mixed => {
                let qt_guard = tracker.read().ok();
                let qt = qt_guard.as_ref().and_then(|g| g.as_ref());
                let res = picker_ref.fuzzy_search_mixed(&parsed, qt, options);
                let items = res
                    .items
                    .iter()
                    .zip(res.scores.iter())
                    .map(|(item, s)| match item {
                        MixedItemRef::File(f) => file_dto(f, picker_ref, base_path, Some(s.total)),
                        MixedItemRef::Dir(d) => FileItemDto {
                            kind: "directory".into(),
                            name: basename(&d.relative_path(picker_ref)),
                            relative_path: d.relative_path(picker_ref),
                            absolute_path: d
                                .absolute_path(picker_ref, base_path)
                                .to_string_lossy()
                                .into_owned(),
                            size: None,
                            modified: None,
                            git_status: None,
                            is_binary: None,
                            score: Some(s.total),
                        },
                    })
                    .collect::<Vec<_>>();
                Ok(SearchResponse {
                    total_matched: res.total_matched,
                    total_files: res.total_files,
                    total_dirs: Some(res.total_dirs),
                    items,
                })
            }
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("search task failed: {e}")))??;

    Ok(Json(resp))
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct GlobParams {
    /// Literal glob pattern, e.g. `**/*.rs`.
    pub pattern: String,
    #[param(default = 100)]
    pub limit: Option<usize>,
    #[param(default = 0)]
    pub offset: Option<usize>,
    /// Relative path of the currently open file (deprioritized in scoring).
    pub current_file: Option<String>,
}

/// Literal glob search (no fuzzy matching), frecency-ranked.
#[utoipa::path(
    get,
    path = "/api/glob",
    tag = "search",
    params(GlobParams),
    responses(
        (status = 200, description = "Glob results", body = SearchResponse),
        (status = 400, description = "Bad request", body = serde_json::Value),
        (status = 500, description = "Internal error", body = serde_json::Value),
    )
)]
pub async fn glob(
    State(state): State<AppState>,
    Query(params): Query<GlobParams>,
) -> Result<Json<SearchResponse>> {
    let limit = clamp_limit(params.limit, state.max_results);
    let offset = params.offset.unwrap_or(0);
    let pattern = params.pattern;
    let current_file = params.current_file;
    let picker = state.picker.clone();

    let resp = tokio::task::spawn_blocking(move || -> Result<SearchResponse> {
        let guard = picker.read()?;
        let picker_ref = guard
            .as_ref()
            .ok_or(AppError::Internal("file picker not ready".into()))?;

        let options = FuzzySearchOptions {
            max_threads: 0,
            current_file: current_file.as_deref(),
            project_path: None,
            combo_boost_score_multiplier: 100,
            min_combo_count: 3,
            pagination: PaginationArgs { offset, limit },
        };
        let res = picker_ref.glob(&pattern, options);
        Ok(build_response_files(&res, picker_ref, picker_ref.base_path()))
    })
    .await
    .map_err(|e| AppError::Internal(format!("glob task failed: {e}")))??;

    Ok(Json(resp))
}

#[derive(Clone, Copy)]
enum Mode {
    Files,
    Dirs,
    Mixed,
}

fn parse_mode(s: Option<&str>) -> Result<Mode> {
    match s.map(str::to_ascii_lowercase).as_deref() {
        None | Some("") | Some("files") | Some("file") => Ok(Mode::Files),
        Some("dirs") | Some("dir") | Some("directories") => Ok(Mode::Dirs),
        Some("mixed") => Ok(Mode::Mixed),
        Some(other) => Err(AppError::BadRequest(format!("unknown mode '{other}'"))),
    }
}

fn clamp_limit(req: Option<usize>, max: usize) -> usize {
    req.unwrap_or(max).clamp(1, max.max(1))
}

fn build_response_files(
    res: &SearchResult<'_>,
    picker: &FilePicker,
    base_path: &std::path::Path,
) -> SearchResponse {
    let items = res
        .items
        .iter()
        .zip(res.scores.iter())
        .map(|(f, s)| file_dto(f, picker, base_path, Some(s.total)))
        .collect::<Vec<_>>();
    SearchResponse {
        total_matched: res.total_matched,
        total_files: res.total_files,
        total_dirs: None,
        items,
    }
}

pub(crate) fn file_dto(
    f: &FileItem,
    picker: &FilePicker,
    base_path: &std::path::Path,
    score: Option<i32>,
) -> FileItemDto {
    FileItemDto {
        kind: "file".into(),
        name: f.file_name(picker),
        relative_path: f.relative_path(picker),
        absolute_path: f.absolute_path(picker, base_path).to_string_lossy().into_owned(),
        size: Some(f.size),
        modified: Some(f.modified),
        git_status: f.git_status.map(git_status_label),
        is_binary: Some(f.is_binary()),
        score,
    }
}

pub(crate) fn basename(path: &str) -> String {
    let trimmed = path.trim_end_matches('/').trim_end_matches(std::path::MAIN_SEPARATOR);
    trimmed
        .rsplit(|c| c == '/' || c == std::path::MAIN_SEPARATOR)
        .next()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| path.to_owned())
}

pub(crate) fn git_status_label(status: git2::Status) -> String {
    use git2::Status as S;
    if status.contains(S::IGNORED) {
        return "ignored".into();
    }
    let mut parts: Vec<&str> = Vec::new();
    if status.contains(S::INDEX_NEW) || status.contains(S::WT_NEW) {
        parts.push(if status.contains(S::INDEX_NEW) {
            "staged"
        } else {
            "untracked"
        });
    }
    if status.contains(S::INDEX_MODIFIED) || status.contains(S::WT_MODIFIED) {
        parts.push("modified");
    }
    if status.contains(S::INDEX_DELETED) || status.contains(S::WT_DELETED) {
        parts.push("deleted");
    }
    if status.contains(S::INDEX_RENAMED) || status.contains(S::WT_RENAMED) {
        parts.push("renamed");
    }
    if status.contains(S::INDEX_TYPECHANGE) || status.contains(S::WT_TYPECHANGE) {
        parts.push("typechange");
    }
    if parts.is_empty() {
        "current".into()
    } else {
        parts.join("+")
    }
}
