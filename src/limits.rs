//! Shared input-validation policy applied uniformly by the REST and MCP layers.
use crate::error::{AppError, Result};

/// Maximum accepted query/pattern length, in characters.
pub const MAX_QUERY_LEN: usize = 256;
/// Maximum accepted pagination offset. Deep pagination is expensive
/// (`offset + limit` entries are materialized then discarded).
pub const MAX_OFFSET: usize = 10000;

pub fn validate_query(q: &str) -> Result<()> {
    if q.chars().count() > MAX_QUERY_LEN {
        return Err(AppError::BadRequest(format!(
            "query too long (max {MAX_QUERY_LEN} chars)"
        )));
    }
    Ok(())
}

pub fn validate_offset(req: Option<usize>) -> Result<usize> {
    match req {
        Some(n) if n > MAX_OFFSET => Err(AppError::BadRequest(format!(
            "offset too large (max {MAX_OFFSET})"
        ))),
        Some(n) => Ok(n),
        None => Ok(0),
    }
}
