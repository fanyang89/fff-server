//! Agent skill install endpoints.
//!
//! Two GET handlers that render `install/SKILL.md.tmpl` and
//! `install/install.sh.tmpl` server-side, with user-supplied query parameters
//! baked in as bash single-quoted literals. Designed for the
//! `curl ... | bash` one-liner pattern.
//!
//! Safety: every user value is wrapped in single quotes with `'` escaped to
//! `'\''`, so no shell expansion of user input can happen at script runtime.
//! `name` is constrained to `^[a-z0-9]+(-[a-z0-9]+)*$` and must match its
//! skill directory; `url` must be `http(s)://`.

use axum::extract::Query;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

const SKILL_TMPL: &str = include_str!("../../install/SKILL.md.tmpl");
const INSTALL_SH_TMPL: &str = include_str!("../../install/install.sh.tmpl");

const NAME_MAX: usize = 64;
const URL_MAX: usize = 2048;
const SCOPE_MAX: usize = 200;
const NOTES_MAX: usize = 1000;

#[derive(Debug, Deserialize)]
pub struct InstallParams {
    pub name: String,
    pub url: String,
    pub scope: Option<String>,
    pub notes: Option<String>,
    pub agent: Option<String>,
    pub target: Option<String>,
}

/// `GET /install/skill.md` — render the skill file. Served with an attachment
/// disposition so a direct browser visit downloads as `SKILL.md`.
pub async fn skill_md(Query(p): Query<InstallParams>) -> Response {
    match render_skill(&p) {
        Ok(body) => (
            [
                (header::CONTENT_TYPE, "text/markdown; charset=utf-8"),
                (header::CONTENT_DISPOSITION, "attachment; filename=\"SKILL.md\""),
            ],
            body,
        )
            .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}

/// `GET /install.sh` — render the one-liner install script with all params
/// baked in. `curl -fsSL '<this url>' | bash` is the intended invocation.
pub async fn install_sh(Query(mut p): Query<InstallParams>, headers: HeaderMap) -> Response {
    let agent = p.agent.take().unwrap_or_else(|| "opencode".into());
    let target = p.target.take().unwrap_or_else(|| "global".into());
    if let Err(e) = validate_all(&p, &agent, &target) {
        return (StatusCode::BAD_REQUEST, e).into_response();
    }
    let origin = infer_origin(&headers);
    let body = render_install_sh(&p, &agent, &target, &origin);
    (
        [(header::CONTENT_TYPE, "text/x-shellscript; charset=utf-8")],
        body,
    )
        .into_response()
}

// --- validation --------------------------------------------------------------

fn validate_all(p: &InstallParams, agent: &str, target: &str) -> Result<(), String> {
    validate_name(&p.name)?;
    validate_url(&p.url)?;
    validate_choice("agent", agent, &["opencode", "claude", "generic"])?;
    validate_choice("target", target, &["global", "project"])?;
    if let Some(s) = p.scope.as_deref() {
        check_len("scope", s, SCOPE_MAX)?;
    }
    if let Some(n) = p.notes.as_deref() {
        check_len("notes", n, NOTES_MAX)?;
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), String> {
    check_len("name", name, NAME_MAX)?;
    if name.is_empty() || name == "-" || name.contains("--") {
        return Err(format!("invalid name: {name:?}"));
    }
    let bad = name
        .chars()
        .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-'));
    if let Some(c) = bad {
        return Err(format!("invalid name char {c:?}: only a-z 0-9 - allowed"));
    }
    if name.starts_with('-') || name.ends_with('-') {
        return Err(format!("invalid name: must not start or end with '-': {name:?}"));
    }
    Ok(())
}

fn validate_url(url: &str) -> Result<(), String> {
    check_len("url", url, URL_MAX)?;
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("invalid url: must start with http:// or https://".into());
    }
    if url.split_once("://").is_none_or(|(_, rest)| rest.is_empty()) {
        return Err("invalid url: missing host".into());
    }
    Ok(())
}

fn validate_choice(field: &str, value: &str, allowed: &[&str]) -> Result<(), String> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(format!(
            "invalid {field}: {value:?}; allowed: {}",
            allowed.join(", ")
        ))
    }
}

fn check_len(field: &str, value: &str, max: usize) -> Result<(), String> {
    if value.len() <= max {
        Ok(())
    } else {
        Err(format!("{field} too long: {} > {max} chars", value.len()))
    }
}

// --- rendering ---------------------------------------------------------------

fn render_skill(p: &InstallParams) -> Result<String, String> {
    validate_all(
        p,
        p.agent.as_deref().unwrap_or("opencode"),
        p.target.as_deref().unwrap_or("global"),
    )?;
    let scope = p.scope.as_deref().unwrap_or("the indexed tree");
    let notes = p.notes.as_deref().unwrap_or("");
    let body = SKILL_TMPL
        .replace("{{INSTANCE_NAME}}", &p.name)
        .replace("{{INSTANCE_URL}}", &p.url)
        .replace("{{SCOPE}}", scope);
    // Notes: drop the marker line entirely when empty, else inline the text.
    let body = if notes.is_empty() {
        body.replace("{{NOTES}}\n", "")
    } else {
        body.replace("{{NOTES}}", notes)
    };
    Ok(body)
}

fn render_install_sh(p: &InstallParams, agent: &str, target: &str, origin: &str) -> String {
    let scope = p.scope.as_deref().unwrap_or("");
    let notes = p.notes.as_deref().unwrap_or("");
    INSTALL_SH_TMPL
        .replace("{{INSTANCE_NAME}}", &shell_quote(&p.name))
        .replace("{{INSTANCE_URL}}", &shell_quote(&p.url))
        .replace("{{SCOPE}}", &shell_quote(scope))
        .replace("{{NOTES}}", &shell_quote(notes))
        .replace("{{NOTES_ENC}}", &shell_quote(&percent_encode(notes)))
        .replace("{{AGENT}}", &shell_quote(agent))
        .replace("{{TARGET}}", &shell_quote(target))
        .replace("{{ORIGIN}}", &shell_quote(origin))
}

/// Wrap a value in bash single quotes, escaping embedded `'` as `'\''`.
/// After this, the value is inert at runtime — no `$()`, backticks, or
/// variable expansion can take effect.
fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

/// Minimal percent-encoder for embedding a value back into a query string
/// (used by the install script to re-fetch SKILL.md with the same params).
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn infer_origin(headers: &HeaderMap) -> String {
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .filter(|s| *s == "https" || *s == "http")
        .unwrap_or("http");
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("127.0.0.1:8787");
    format!("{scheme}://{host}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(name: &str, url: &str) -> InstallParams {
        InstallParams {
            name: name.into(),
            url: url.into(),
            scope: None,
            notes: None,
            agent: None,
            target: None,
        }
    }

    #[test]
    fn name_validation_accepts_legal() {
        assert!(validate_name("plocate").is_ok());
        assert!(validate_name("my-files-2").is_ok());
        assert!(validate_name("a").is_ok());
    }

    #[test]
    fn name_validation_rejects_illegal() {
        assert!(validate_name("").is_err());
        assert!(validate_name("-x").is_err());
        assert!(validate_name("x-").is_err());
        assert!(validate_name("a--b").is_err());
        assert!(validate_name("Plocate").is_err()); // uppercase
        assert!(validate_name("a_b").is_err()); // underscore
        assert!(validate_name("a.b").is_err()); // dot
        assert!(validate_name(&"x".repeat(NAME_MAX + 1)).is_err());
    }

    #[test]
    fn url_validation() {
        assert!(validate_url("http://127.0.0.1:8787/mcp").is_ok());
        assert!(validate_url("https://files.example.com/mcp").is_ok());
        assert!(validate_url("ftp://x/y").is_err());
        assert!(validate_url("https://").is_err()); // no host
        assert!(validate_url("not a url").is_err());
    }

    #[test]
    fn agent_target_choices() {
        assert!(validate_choice("agent", "opencode", &["opencode", "claude", "generic"]).is_ok());
        assert!(validate_choice("agent", "claude", &["opencode", "claude", "generic"]).is_ok());
        assert!(validate_choice("agent", "cursor", &["opencode", "claude", "generic"]).is_err());
        assert!(validate_choice("target", "global", &["global", "project"]).is_ok());
        assert!(validate_choice("target", "project", &["global", "project"]).is_ok());
        assert!(validate_choice("target", "user", &["global", "project"]).is_err());
    }

    #[test]
    fn shell_quote_escapes_single_quote() {
        assert_eq!(shell_quote("plain"), "'plain'");
        assert_eq!(shell_quote("a'b"), "'a'\\''b'");
    }

    #[test]
    fn shell_quote_neutralizes_command_substitution() {
        // A malicious url that would otherwise expand inside double quotes.
        let evil = "$(rm -rf ~)";
        let q = shell_quote(evil);
        assert_eq!(q, "'$(rm -rf ~)'");
        // Inside single quotes at runtime this is the literal string.
        assert!(!q.contains("\""));
    }

    #[test]
    fn install_sh_bakes_params_as_single_quoted_literals() {
        let mut params = p("plocate", "https://host/mcp");
        params.scope = Some("/srv/files".into());
        params.notes = Some("private".into());
        let body = render_install_sh(&params, "opencode", "global", "https://host:8787");
        assert!(body.contains("NAME='plocate'"));
        assert!(body.contains("URL='https://host/mcp'"));
        assert!(body.contains("SCOPE='/srv/files'"));
        assert!(body.contains("AGENT='opencode'"));
        assert!(body.contains("TARGET='global'"));
        assert!(body.contains("ORIGIN='https://host:8787'"));
        assert!(body.contains("NOTES_ENC='private'"));
        // No raw template placeholders left.
        assert!(!body.contains("{{"));
        assert!(!body.contains("}}"));
    }

    #[test]
    fn install_sh_neutralizes_shell_injection_in_url() {
        // A url containing a single quote and a command substitution.
        let evil_url = "http://x'$(rm -rf ~)')";
        let mut params = p("plocate", "");
        params.url = evil_url.into();
        let body = render_install_sh(&params, "opencode", "global", "http://x");
        // The dangerous payload must be contained inside single quotes — a
        // leading `$` inside `'...'` is a literal, not a substitution.
        assert!(body.contains("'$(rm -rf ~)'"));
        // And the line starts as a quoted assignment (no bare expansion).
        assert!(body.contains("URL='http://x'"));
    }

    #[test]
    fn render_skill_replaces_all_placeholders() {
        let mut params = p("my-files", "https://host/mcp");
        params.scope = Some("/home/user".into());
        let body = render_skill(&params).unwrap();
        assert!(body.contains("name: my-files"));
        assert!(body.contains("/home/user"));
        assert!(body.contains("https://host/mcp"));
        assert!(!body.contains("{{"));
        assert!(!body.contains("}}"));
    }

    #[test]
    fn render_skill_drops_notes_line_when_empty() {
        let params = p("plocate", "https://host/mcp");
        let body = render_skill(&params).unwrap();
        assert!(!body.contains("{{NOTES}}"));
    }

    #[test]
    fn render_skill_inlines_notes_when_present() {
        let mut params = p("plocate", "https://host/mcp");
        params.notes = Some("only indexes /srv".into());
        let body = render_skill(&params).unwrap();
        assert!(body.contains("only indexes /srv"));
    }

    #[test]
    fn percent_encode_round_trip_safe() {
        assert_eq!(percent_encode("plain"), "plain");
        assert_eq!(percent_encode("a b/c"), "a%20b%2Fc");
        assert_eq!(percent_encode("a&b=c"), "a%26b%3Dc");
        assert_eq!(percent_encode("100%"), "100%25");
    }

    #[test]
    fn infer_origin_uses_forwarded_proto_and_host() {
        let mut h = HeaderMap::new();
        h.insert("host", "files.example.com".parse().unwrap());
        h.insert("x-forwarded-proto", "https".parse().unwrap());
        assert_eq!(infer_origin(&h), "https://files.example.com");
    }

    #[test]
    fn infer_origin_defaults_to_http_localhost() {
        let h = HeaderMap::new();
        assert_eq!(infer_origin(&h), "http://127.0.0.1:8787");
    }
}
