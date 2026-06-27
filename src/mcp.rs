//! MCP (Model Context Protocol) server over Streamable HTTP, mounted at /mcp.
//!
//! Exposes plocate file search as two MCP tools — `search_files` and `glob` —
//! so AI agents can query the indexed tree the same way REST clients do. Shares
//! the single AppState (engine, concurrency cap, timeouts) and the same input
//! validation as the REST layer.

use rmcp::{
    ServerHandler,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, ErrorData, Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;

use crate::dto::SearchResponse;
use crate::limits::{validate_offset, validate_query};
use crate::state::AppState;

pub struct PlocateMcpHandler {
    state: AppState,
}

impl PlocateMcpHandler {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchFilesParams {
    /// Substring to match in file paths. If it contains glob metacharacters
    /// (`*` `?` `[`) plocate treats it as a glob.
    pub query: String,
    /// Maximum results (default 100, server-capped).
    #[serde(default)]
    pub limit: Option<usize>,
    /// Pagination offset (0-based, max 10000).
    #[serde(default)]
    pub offset: Option<usize>,
    /// Case-insensitive match (default true).
    #[serde(default = "default_true")]
    pub case_insensitive: bool,
    /// `path` (default, match the full path) or `basename`.
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GlobParams {
    /// Glob pattern, e.g. `*.rs` or `**/2024/*.log`.
    pub pattern: String,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
    /// Case-insensitive match (default true).
    #[serde(default = "default_true")]
    pub case_insensitive: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FuzzySearchParams {
    /// Query string. Whitespace-separated tokens are matched with AND
    /// semantics (every token must appear in the path); results are ranked
    /// by fzf-style fuzzy relevance.
    pub query: String,
    /// Maximum results (default 100, server-capped).
    #[serde(default)]
    pub limit: Option<usize>,
    /// Pagination offset (0-based, max 10000).
    #[serde(default)]
    pub offset: Option<usize>,
    /// Case-insensitive match (default true).
    #[serde(default = "default_true")]
    pub case_insensitive: bool,
}

#[tool_router]
impl PlocateMcpHandler {
    #[tool(
        name = "search_files",
        description = "Search indexed file paths by substring (or by glob when the query contains *, ? or []). Returns matching relative paths, one per line. The index is refreshed periodically; very recent files may not yet appear."
    )]
    async fn search_files(
        &self,
        Parameters(p): Parameters<SearchFilesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = validate_query(&p.query) {
            return Ok(err_text(&e.to_string()));
        }
        let offset = match validate_offset(p.offset) {
            Ok(n) => n,
            Err(e) => return Ok(err_text(&e.to_string())),
        };
        let limit = p
            .limit
            .unwrap_or(self.state.max_results)
            .clamp(1, self.state.max_results.max(1));
        let basename_only = matches!(
            p.scope.as_deref(),
            Some("basename") | Some("b") | Some("name")
        );
        match self
            .state
            .search(&p.query, limit, offset, p.case_insensitive, basename_only)
            .await
        {
            Ok(resp) => Ok(CallToolResult::success(vec![Content::text(format_search(
                &resp,
            ))])),
            Err(e) => Ok(err_text(&format!("search failed: {e}"))),
        }
    }

    #[tool(
        name = "glob",
        description = "Search indexed file paths by a glob pattern (e.g. *.rs, **/2024/*.log). Returns matching relative paths, one per line."
    )]
    async fn glob(
        &self,
        Parameters(p): Parameters<GlobParams>,
    ) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = validate_query(&p.pattern) {
            return Ok(err_text(&e.to_string()));
        }
        let offset = match validate_offset(p.offset) {
            Ok(n) => n,
            Err(e) => return Ok(err_text(&e.to_string())),
        };
        let limit = p
            .limit
            .unwrap_or(self.state.max_results)
            .clamp(1, self.state.max_results.max(1));
        match self
            .state
            .search(&p.pattern, limit, offset, p.case_insensitive, false)
            .await
        {
            Ok(resp) => Ok(CallToolResult::success(vec![Content::text(format_search(
                &resp,
            ))])),
            Err(e) => Ok(err_text(&format!("glob failed: {e}"))),
        }
    }

    #[tool(
        name = "fuzzy_search",
        description = "Fuzzy multi-keyword search over indexed file paths. Whitespace-separated tokens are AND-ed (every token must appear), then ranked by fzf-style relevance (nucleo). Best for queries like 'zookeeper rpm oe1' where substring search would return nothing. Returns matching relative paths ranked by relevance, one per line."
    )]
    async fn fuzzy_search(
        &self,
        Parameters(p): Parameters<FuzzySearchParams>,
    ) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = validate_query(&p.query) {
            return Ok(err_text(&e.to_string()));
        }
        let offset = match validate_offset(p.offset) {
            Ok(n) => n,
            Err(e) => return Ok(err_text(&e.to_string())),
        };
        let limit = p
            .limit
            .unwrap_or(self.state.max_results)
            .clamp(1, self.state.max_results.max(1));
        match self
            .state
            .search_fuzzy(&p.query, limit, offset, p.case_insensitive)
            .await
        {
            Ok(resp) => Ok(CallToolResult::success(vec![Content::text(format_search(
                &resp,
            ))])),
            Err(e) => Ok(err_text(&format!("fuzzy search failed: {e}"))),
        }
    }
}

#[tool_handler]
impl ServerHandler for PlocateMcpHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                String::from("plocate-server"),
                String::from(env!("CARGO_PKG_VERSION")),
            ))
            .with_instructions(String::from(
                "Filename/path search over a large indexed file tree via plocate. \
                 Use search_files for substring/glob queries; results are relative paths.",
            ))
    }
}

/// Format search results as agent-friendly text (relative paths only, to save tokens).
fn format_search(resp: &SearchResponse) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} match(es){}\n",
        resp.total_matched,
        if resp.truncated {
            " (truncated, more exist)"
        } else {
            ""
        }
    ));
    for it in &resp.items {
        out.push_str(&it.relative_path);
        if it.kind == "directory" {
            out.push('/');
        }
        out.push('\n');
    }
    out.trim_end().to_owned()
}

fn err_text(msg: &str) -> CallToolResult {
    CallToolResult::error(vec![Content::text(msg.to_owned())])
}
