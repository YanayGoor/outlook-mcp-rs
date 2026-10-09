use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE, WWW_AUTHENTICATE},
        HeaderValue, Request, StatusCode,
    },
    middleware::{self, Next},
    response::Response,
    Router,
};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};

use crate::server::OutlookMcpServer;

/// The single HTTP path the MCP Streamable HTTP endpoint is mounted at.
pub const MCP_PATH: &str = "/mcp";

/// Pure authorization decision, split out so it is unit-testable without a
/// live socket. `configured` is the token the server was started with
/// (`None` = auth disabled). `header` is the raw incoming `Authorization`
/// header value, if any.
pub fn is_authorized(configured: Option<&str>, header: Option<&str>) -> bool {
    let Some(secret) = configured else {
        return true; // auth disabled
    };
    let Some(header) = header else {
        return false; // auth required but no header
    };
    let Some(presented) = header.strip_prefix("Bearer ") else {
        return false; // wrong scheme
    };
    // Constant-time compare to avoid a timing side channel on the secret's
    // *content*. `constant_time_eq` still short-circuits on a length
    // mismatch, so token *length* is technically observable via timing —
    // negligible against a reasonably long, high-entropy token.
    constant_time_eq::constant_time_eq(presented.as_bytes(), secret.as_bytes())
}

/// Axum middleware: gate every request on `is_authorized`, else 401.
async fn auth_middleware(
    State(token): State<Arc<Option<String>>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    if is_authorized(token.as_deref(), header) {
        next.run(req).await
    } else {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(WWW_AUTHENTICATE, "Bearer")
            .body(Body::empty())
            .expect("static empty 401 response is always valid")
    }
}

/// The `Content-Type` to send instead of `content_type` so that it names
/// UTF-8 explicitly, or `None` to leave it unchanged.
///
/// rmcp sends bare `application/json` and `text/event-stream`. Both are
/// UTF-8 by their specs (RFC 8259 and the WHATWG SSE spec), but a generic
/// HTTP client that applies the old RFC 2616 rule ("`text/*` without a
/// charset is ISO-8859-1"), e.g. Python `requests`' `.text`, decodes the SSE
/// stream as latin1 and turns every Hebrew character into two or three
/// mojibake characters (issue #31). Naming the charset removes that guess.
/// Other media types, and ones that already carry a `charset`, are left
/// alone.
pub fn with_utf8_charset(content_type: &str) -> Option<String> {
    let mut parts = content_type.split(';');
    let media_type = parts.next().unwrap_or_default().trim();
    let textual = media_type.eq_ignore_ascii_case("application/json")
        || media_type.eq_ignore_ascii_case("text/event-stream");
    let has_charset = parts.any(|p| {
        p.trim()
            .split('=')
            .next()
            .is_some_and(|name| name.trim().eq_ignore_ascii_case("charset"))
    });
    (textual && !has_charset).then(|| format!("{}; charset=utf-8", content_type.trim_end()))
}

/// Axum response mapper: apply `with_utf8_charset` to the `Content-Type`.
async fn utf8_charset(mut response: Response) -> Response {
    let replacement = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(with_utf8_charset)
        .and_then(|ct| HeaderValue::from_str(&ct).ok());
    if let Some(value) = replacement {
        response.headers_mut().insert(CONTENT_TYPE, value);
    }
    response
}

/// Build the axum router: mount rmcp's Streamable HTTP MCP service at
/// `/mcp`, wrapped by the bearer-auth layer. Exposed (not just used by
/// `run_http`) so integration tests can drive it on an ephemeral port.
///
/// Every JSON / SSE response declares `charset=utf-8` (see
/// `with_utf8_charset`).
///
/// `allowed_hosts` is deliberately disabled: clients connect via the Windows
/// computer name (whose `Host` header we cannot predict), our clients are
/// native (not browsers vulnerable to DNS rebinding), and the bearer token is
/// the real access control. See the design doc's "DNS rebinding" section.
pub fn build_router(server: OutlookMcpServer, token: Option<String>) -> Router {
    let service = StreamableHttpService::new(
        move || Ok(server.clone()),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().disable_allowed_hosts(),
    );
    Router::new()
        .route_service(MCP_PATH, service)
        .layer(middleware::from_fn_with_state(Arc::new(token), auth_middleware))
        .layer(middleware::map_response(utf8_charset))
}

/// Bind `addr` and serve the MCP endpoint until the process is terminated.
pub async fn run_http(
    server: OutlookMcpServer,
    addr: SocketAddr,
    token: Option<String>,
) -> anyhow::Result<()> {
    let router = build_router(server, token);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    // stderr, not stdout — never interferes with an MCP stdio stream (which
    // this mode isn't using anyway) and gives the operator a confirmation line.
    eprintln!("outlook-mcp-rs listening on http://{addr}{MCP_PATH}");
    axum::serve(listener, router).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{is_authorized, with_utf8_charset};

    #[test]
    fn utf8_charset_is_added_to_bare_json_and_sse() {
        assert_eq!(
            with_utf8_charset("application/json").as_deref(),
            Some("application/json; charset=utf-8")
        );
        assert_eq!(
            with_utf8_charset("text/event-stream").as_deref(),
            Some("text/event-stream; charset=utf-8")
        );
        assert_eq!(
            with_utf8_charset("Text/Event-Stream ").as_deref(),
            Some("Text/Event-Stream; charset=utf-8")
        );
    }

    #[test]
    fn utf8_charset_keeps_other_parameters() {
        assert_eq!(
            with_utf8_charset("application/json; foo=bar").as_deref(),
            Some("application/json; foo=bar; charset=utf-8")
        );
    }

    #[test]
    fn utf8_charset_leaves_an_existing_charset_alone() {
        assert_eq!(with_utf8_charset("application/json; charset=utf-8"), None);
        assert_eq!(with_utf8_charset("text/event-stream;Charset=UTF-8"), None);
        // Not ours to correct: only a missing charset is filled in.
        assert_eq!(with_utf8_charset("application/json; charset=iso-8859-1"), None);
    }

    #[test]
    fn utf8_charset_ignores_other_media_types() {
        assert_eq!(with_utf8_charset("text/plain"), None);
        assert_eq!(with_utf8_charset("application/jsonx"), None);
        assert_eq!(with_utf8_charset("image/png"), None);
        assert_eq!(with_utf8_charset(""), None);
    }

    #[test]
    fn no_token_configured_allows_everything() {
        assert!(is_authorized(None, None));
        assert!(is_authorized(None, Some("Bearer whatever")));
        assert!(is_authorized(None, Some("garbage")));
    }

    #[test]
    fn correct_bearer_token_is_authorized() {
        assert!(is_authorized(Some("s3cret"), Some("Bearer s3cret")));
    }

    #[test]
    fn wrong_token_is_rejected() {
        assert!(!is_authorized(Some("s3cret"), Some("Bearer nope")));
    }

    #[test]
    fn missing_header_is_rejected_when_token_configured() {
        assert!(!is_authorized(Some("s3cret"), None));
    }

    #[test]
    fn wrong_scheme_is_rejected() {
        assert!(!is_authorized(Some("s3cret"), Some("Basic s3cret")));
        assert!(!is_authorized(Some("s3cret"), Some("s3cret")));
    }
}
