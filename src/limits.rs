//! Shared input-validation policy applied uniformly by the REST and MCP layers.
use crate::error::{AppError, Result};

/// Maximum accepted query/pattern length, in characters.
pub const MAX_QUERY_LEN: usize = 256;
/// Maximum accepted pagination offset. Deep pagination is expensive
/// (`offset + limit` entries are materialized then discarded).
pub const MAX_OFFSET: usize = 10000;

/// Maximum length of a skill/MCP instance name.
pub const MAX_NAME_LEN: usize = 64;

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

/// Validate a skill/MCP instance name. Must match `^[a-z0-9]+(-[a-z0-9]+)*$`,
/// 1..=MAX_NAME_LEN chars, no leading/trailing/double hyphens. The same rule
/// is enforced at startup for the `--instance-name` flag.
pub fn validate_skill_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > MAX_NAME_LEN {
        return Err(AppError::BadRequest(format!(
            "name must be 1..={MAX_NAME_LEN} chars"
        )));
    }
    if name.starts_with('-') || name.ends_with('-') || name.contains("--") {
        return Err(AppError::BadRequest(format!(
            "invalid name {name:?}: hyphen rules violated"
        )));
    }
    if name
        .chars()
        .any(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
    {
        return Err(AppError::BadRequest(format!(
            "invalid name {name:?}: only a-z 0-9 - allowed"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_name_accepts_legal() {
        assert!(validate_skill_name("plocate").is_ok());
        assert!(validate_skill_name("my-files-2").is_ok());
        assert!(validate_skill_name("a").is_ok());
    }

    #[test]
    fn skill_name_rejects_illegal() {
        assert!(validate_skill_name("").is_err());
        assert!(validate_skill_name("-x").is_err());
        assert!(validate_skill_name("x-").is_err());
        assert!(validate_skill_name("a--b").is_err());
        assert!(validate_skill_name("Plocate").is_err()); // uppercase
        assert!(validate_skill_name("a_b").is_err()); // underscore
        assert!(validate_skill_name("a.b").is_err()); // dot
        assert!(validate_skill_name(&"x".repeat(MAX_NAME_LEN + 1)).is_err());
    }
}
