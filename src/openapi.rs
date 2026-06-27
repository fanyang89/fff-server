use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "fff-server",
        version = "0.1.0",
        description = "RESTful API server built on top of the fff file-search engine.\n\nExposes frecency-ranked fuzzy file search, glob matching, and index lifecycle management.",
        license(name = "MIT"),
    ),
    paths(
        crate::routes::search::search,
        crate::routes::search::glob,
        crate::routes::history::history,
        crate::routes::history::track,
        crate::routes::lifecycle::health,
        crate::routes::lifecycle::scan_progress,
        crate::routes::lifecycle::rescan,
        crate::routes::lifecycle::refresh_git,
        crate::routes::lifecycle::base_path,
        crate::routes::stats::stats,
    ),
    components(schemas(
        crate::dto::FileItemDto,
        crate::dto::SearchResponse,
        crate::dto::HealthResponse,
        crate::dto::ScanProgressResponse,
        crate::dto::BasePathResponse,
        crate::dto::RescanResponse,
        crate::dto::RefreshGitResponse,
        crate::dto::TrackRequest,
        crate::dto::TrackResponse,
        crate::dto::HistoryResponse,
        crate::dto::StatsProcess,
        crate::dto::StatsIndex,
        crate::dto::StatsCache,
        crate::dto::StatsResponse,
    )),
    tags(
        (name = "search", description = "Fuzzy / glob file search"),
        (name = "history", description = "Frecency and query history"),
        (name = "lifecycle", description = "Index lifecycle & health"),
    ),
)]
pub struct ApiDoc;
