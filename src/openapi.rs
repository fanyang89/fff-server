use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "fff-server",
        version = "0.1.0",
        description = "RESTful filename-search API server backed by a plocate trigram index.\n\nThe index lives on disk (built by updatedb), so a restart never rescans. Designed for very large trees (millions of files).",
        license(name = "MIT"),
    ),
    paths(
        crate::routes::search::search,
        crate::routes::search::glob,
        crate::routes::health::health,
        crate::routes::health::base_path,
        crate::routes::stats::stats,
        crate::routes::reindex::reindex,
    ),
    components(schemas(
        crate::dto::FileItemDto,
        crate::dto::SearchResponse,
        crate::dto::HealthResponse,
        crate::dto::StatsResponse,
        crate::dto::StatsProcess,
        crate::dto::StatsIndex,
        crate::dto::ReindexRecordDto,
        crate::dto::BasePathResponse,
        crate::dto::ReindexResponse,
    )),
    tags(
        (name = "search", description = "Filename / path search via plocate"),
        (name = "lifecycle", description = "Index health, stats, and reindex"),
    ),
)]
pub struct ApiDoc;
