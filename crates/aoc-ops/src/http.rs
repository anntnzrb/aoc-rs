//! Requests to adventofcode.com through curl, with the session cookie passed on stdin.

use std::path::Path;

use zeroize::Zeroizing;

use crate::error::OpsError;
use crate::process;
use crate::session::Session;

/// A bare product token: requests stay anonymous.
const USER_AGENT: &str = "aoc-rs";
/// Upper bound for a whole request, including TLS setup and the response body.
const MAX_TIME_SECONDS: &str = "60";
/// Marker separating the response body from curl's status trailer.
const TRAILER: &str = "\n@@aoc-ops@@";

/// A form field for POST requests; values are restricted to URL-safe ASCII by their parsers.
pub(crate) type Field<'a> = (&'a str, &'a str);

/// Escapes a value for a double-quoted curl config string, where `\n` denotes a newline.
fn quoted(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Builds curl's config: every secret-bearing option goes through stdin, never argv.
fn config(base: &str, session: &Session, path: &str, form: &[Field<'_>]) -> Zeroizing<String> {
    let mut text = Zeroizing::new(format!(
        "url = \"{}{}\"\nuser-agent = \"{USER_AGENT}\"\nmax-time = {MAX_TIME_SECONDS}\n\
         silent\nshow-error\nwrite-out = \"{}%{{http_code}} %header{{retry-after}}\"\n",
        quoted(base),
        quoted(path),
        quoted(TRAILER),
    ));
    text.push_str("cookie = \"session=");
    text.push_str(&quoted(session.as_str()));
    text.push_str("\"\n");
    for (key, value) in form {
        text.push_str("data-urlencode = \"");
        text.push_str(&quoted(key));
        text.push('=');
        text.push_str(&quoted(value));
        text.push_str("\"\n");
    }
    text
}

/// Splits curl's output into the body and the parsed status trailer.
fn response(output: &str) -> Result<&str, OpsError> {
    let (body, trailer) = output
        .rsplit_once(TRAILER)
        .ok_or_else(|| OpsError::Http("HTTP request failed".to_owned()))?;
    let (status, retry_after) = trailer.split_once(' ').unwrap_or((trailer, ""));
    match status {
        "200" => Ok(body),
        "429" => Err(OpsError::Http(
            retry_after.trim().parse::<u64>().map_or_else(
                |_unparsed| "rate limited; retry delay unavailable".to_owned(),
                |seconds| format!("rate limited; retry after {seconds}s"),
            ),
        )),
        "000" => Err(OpsError::Http("HTTP request failed".to_owned())),
        other => Err(OpsError::Http(format!(
            "HTTP {}",
            other
                .chars()
                .filter(char::is_ascii_digit)
                .collect::<String>()
        ))),
    }
}

/// Requests `base` + `path` (GET without a form, POST with one) and returns a 200 response body.
pub(crate) fn request(
    root: &Path,
    base: &str,
    session: &Session,
    path: &str,
    form: &[Field<'_>],
) -> Result<String, OpsError> {
    let config = config(base, session, path, form);
    let finished = process::capture(root, "curl", ["--config", "-"], config.as_bytes())?;
    let output = String::from_utf8(finished.stdout)
        .map_err(|_utf8| OpsError::Http("response is not UTF-8".to_owned()))?;
    let body = response(&output)?;
    // Transport failures leave curl non-zero with status 000, reported above without stderr.
    if finished.success {
        Ok(body.to_owned())
    } else {
        Err(OpsError::Http("HTTP request failed".to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::{TRAILER, config, request, response};
    use crate::session;
    use crate::test_support::serve;

    #[test]
    fn config_carries_the_cookie_and_escapes_values() {
        let session = session::parse("AOC_SESSION=abc").unwrap();
        let text = config(
            "https://adventofcode.com/",
            &session,
            "2018/day/1/answer",
            &[("level", "1"), ("answer", "a\"b\nc")],
        );
        assert!(text.contains("cookie = \"session=abc\"\n"), "cookie line");
        assert!(
            text.contains("url = \"https://adventofcode.com/2018/day/1/answer\"\n"),
            "url"
        );
        assert!(
            text.contains("data-urlencode = \"answer=a\\\"b\\nc\"\n"),
            "quotes and newlines escaped"
        );
    }

    #[test]
    fn request_sends_cookie_agent_and_form_and_returns_the_body() {
        let (base, server) =
            serve("HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nbody");
        let session = session::parse("AOC_SESSION=c00kie").unwrap();
        let dir = std::env::temp_dir();
        let body = request(
            &dir,
            &base,
            &session,
            "2018/day/1/answer",
            &[("level", "2"), ("answer", "a,b")],
        );
        let seen = server.join().unwrap();
        assert_eq!(body.unwrap(), "body", "200 returns the body");
        assert!(
            seen.starts_with("POST /2018/day/1/answer "),
            "form requests post: {seen}"
        );
        assert!(
            seen.contains("Cookie: session=c00kie\r\n"),
            "cookie header: {seen}"
        );
        assert!(
            seen.contains("User-Agent: aoc-rs\r\n"),
            "anonymous agent: {seen}"
        );
        assert!(
            seen.ends_with("level=2&answer=a%2Cb"),
            "urlencoded form: {seen}"
        );
    }

    #[test]
    fn request_maps_error_statuses_and_gets_without_a_form() {
        let (base, server) = serve(
            "HTTP/1.1 429 Too Many\r\nRetry-After: 7\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        let session = session::parse("AOC_SESSION=x").unwrap();
        let error = request(
            &std::env::temp_dir(),
            &base,
            &session,
            "2018/day/1/input",
            &[],
        );
        assert!(
            server.join().unwrap().starts_with("GET /2018/day/1/input "),
            "no form gets"
        );
        assert_eq!(
            error.unwrap_err().to_string(),
            "rate limited; retry after 7s",
            "429 reports the delay"
        );
        let (missing_base, missing_server) =
            serve("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        let missing = request(&std::env::temp_dir(), &missing_base, &session, "x", &[]);
        missing_server.join().unwrap();
        assert_eq!(missing.unwrap_err().to_string(), "HTTP 404", "other status");
    }

    #[test]
    fn request_reports_transport_failure() {
        // Bind then drop a listener so the port refuses connections.
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let session = session::parse("AOC_SESSION=x").unwrap();
        let error = request(
            &std::env::temp_dir(),
            &format!("http://127.0.0.1:{port}/"),
            &session,
            "x",
            &[],
        );
        assert_eq!(
            error.unwrap_err().to_string(),
            "HTTP request failed",
            "refused"
        );
    }

    #[test]
    fn response_maps_status_codes() {
        let ok = format!("body{TRAILER}200 ");
        assert_eq!(response(&ok).unwrap(), "body", "200 returns the body");
        let limited = format!("x{TRAILER}429 30");
        assert_eq!(
            response(&limited).unwrap_err().to_string(),
            "rate limited; retry after 30s",
            "429 reports the delay"
        );
        let unknown = format!("x{TRAILER}429 Wed, 21 Oct 2026 07:28:00 GMT");
        assert_eq!(
            response(&unknown).unwrap_err().to_string(),
            "rate limited; retry delay unavailable",
            "dates are not echoed"
        );
        let missing = format!("x{TRAILER}404 ");
        assert_eq!(
            response(&missing).unwrap_err().to_string(),
            "HTTP 404",
            "other status"
        );
        assert!(response("no trailer").is_err(), "missing trailer fails");
        assert!(
            response(&format!("x{TRAILER}000 ")).is_err(),
            "transport failure"
        );
    }
}
