//! `urzua refs refresh` (`MILE-143`): write the committed snapshot of a
//! repository's issue and pull request numbers that `citation.issue-exists`
//! checks citations against.
//!
//! This is the one command that touches the network; `check` never does. The
//! requests go through `curl`, which honours the proxy and certificate
//! settings of the environment and adds no TLS stack to the binary. The token
//! comes from `GITHUB_TOKEN` or `GH_TOKEN`; without one the request is made
//! unauthenticated, which a public repository answers and a private one
//! refuses with a message that names the token.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};

use urzua_core::config::RuleLevel;
use urzua_core::refs_snapshot::RefsSnapshot;
use urzua_core::report::{
    CouldNotRun, Notice, NoticeSeverity, RefsRefreshOutcome, RefsRefreshReport, ReportStatus,
};
use urzua_core::rules::RULE_CITATION_ISSUE_EXISTS;

use crate::discovery::{find_repo_root, load_config};
use crate::emit;

/// The API origin, overridable for GitHub Enterprise and for tests.
const API_ENV: &str = "URZUA_GITHUB_API";
const DEFAULT_API: &str = "https://api.github.com";
const PER_PAGE: usize = 100;
/// A repository would need a million issues and pull requests to reach this;
/// it only stops a server that never ends a listing.
const MAX_PAGES: usize = 10_000;

pub fn run_refresh(config_path: Option<PathBuf>) -> ExitCode {
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };
    let config_path =
        config_path.unwrap_or_else(|| crate::discovery::default_config_path(&repo_root));
    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };
    let Some(refs) = config
        .rules
        .get(RULE_CITATION_ISSUE_EXISTS)
        .filter(|s| s.level != RuleLevel::Off)
        .and_then(|s| s.issue_refs.clone())
    else {
        return emit(&CouldNotRun::from(
            "urzua refs refresh: no snapshot is declared; enable citation.issue-exists with an `issue_refs` option",
        ));
    };

    let token = ["GITHUB_TOKEN", "GH_TOKEN"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .map(|v| v.trim().to_string())
        .find(|v| !v.is_empty());
    let base = std::env::var(API_ENV)
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_API.to_string());

    let (issues, pulls) = match collect_numbers(&Curl, &base, &refs.repository, token.as_deref()) {
        Ok(found) => found,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };
    let snapshot = RefsSnapshot::new(&refs.repository, &urzua_io::today(), issues, pulls);
    let mut text = serde_json::to_string_pretty(&snapshot).expect("snapshot serializes");
    text.push('\n');

    let path = repo_root.join(&refs.snapshot);
    let before = std::fs::read_to_string(&path).ok();
    let outcome = match &before {
        Some(b) if *b == text => RefsRefreshOutcome::Current,
        Some(_) => RefsRefreshOutcome::Written,
        None => RefsRefreshOutcome::Created,
    };
    if outcome != RefsRefreshOutcome::Current {
        if let Err(e) = write_atomically(&path, &text) {
            return emit(&CouldNotRun::from(format!(
                "could not write {}: {e}",
                refs.snapshot
            )));
        }
    }
    let mut notices = Vec::new();
    if token.is_none() {
        notices.push(Notice {
            severity: NoticeSeverity::Info,
            subject: "refs".to_string(),
            message: "no GITHUB_TOKEN or GH_TOKEN was set; the request was unauthenticated and subject to the lower rate limit".to_string(),
        });
    }
    emit(&RefsRefreshReport {
        status: ReportStatus::Ok,
        snapshot: PathBuf::from(&refs.snapshot),
        repository: refs.repository,
        generated_at: snapshot.generated_at.clone(),
        outcome,
        issues: snapshot.issues.len(),
        pulls: snapshot.pulls.len(),
        newest: snapshot.newest(),
        notices,
    })
}

/// Replace `path` only once the whole text exists, so a failed write never
/// leaves half a snapshot that a later `check` would read as a stale one.
fn write_atomically(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut temp = path.as_os_str().to_owned();
    temp.push(".tmp");
    let temp = PathBuf::from(temp);
    std::fs::write(&temp, text)?;
    std::fs::rename(&temp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temp);
    })
}

/// One HTTP response, as much as the refresh reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Response {
    pub status: u16,
    /// Lower-cased names.
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Response {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// The one seam to the network, so the paging and failure handling are tested
/// against recorded responses.
pub(crate) trait Fetch {
    fn get(&self, url: &str, token: Option<&str>) -> Result<Response, String>;
}

struct Curl;

impl Fetch for Curl {
    fn get(&self, url: &str, token: Option<&str>) -> Result<Response, String> {
        let mut command = Command::new("curl");
        command
            .args([
                "--silent",
                "--show-error",
                "--include",
                "--max-time",
                "60",
                "--proto",
                "=http,https",
                "--noproxy",
                "localhost,127.0.0.1",
                "-H",
                "Accept: application/vnd.github+json",
                "-H",
                "X-GitHub-Api-Version: 2022-11-28",
                "-H",
                "User-Agent: urzua",
                "--config",
                "-",
                url,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|e| {
            format!("could not run curl, which `urzua refs refresh` uses to reach GitHub: {e}")
        })?;
        // The token goes over stdin, never onto the command line where another
        // process on the host could read it.
        if let (Some(token), Some(mut stdin)) = (token, child.stdin.take()) {
            let escaped = token.replace('\\', "\\\\").replace('"', "\\\"");
            let _ = writeln!(stdin, "header = \"Authorization: Bearer {escaped}\"");
        }
        let output = child
            .wait_with_output()
            .map_err(|e| format!("curl did not finish: {e}"))?;
        if !output.status.success() {
            let why = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "could not reach {url}: curl {}: {}",
                output.status,
                why.trim()
            ));
        }
        parse_response(&String::from_utf8_lossy(&output.stdout))
    }
}

/// Parse `curl --include` output. More than one header block can precede the
/// body (a proxy's `CONNECT` answer, `100 Continue`); the last one is the
/// response.
pub(crate) fn parse_response(raw: &str) -> Result<Response, String> {
    let mut rest = raw;
    let mut headers_text = "";
    while rest.starts_with("HTTP/") {
        let (head, tail) = rest
            .split_once("\r\n\r\n")
            .or_else(|| rest.split_once("\n\n"))
            .ok_or("the response ended inside its headers")?;
        headers_text = head;
        rest = tail;
    }
    if headers_text.is_empty() {
        return Err("the response was not HTTP".to_string());
    }
    let mut lines = headers_text.lines();
    let status = lines
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or("the response had no status line")?;
    let headers = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    Ok(Response {
        status,
        headers,
        body: rest.to_string(),
    })
}

/// Every issue and pull request number of `repository`, as
/// `(issues, pulls)`, fetched a page at a time with `page=N`. The listing's
/// `Link: rel="next"` is read only as the signal that another page exists: its
/// URL is not followed, so the token is only ever sent to `base`.
pub(crate) fn collect_numbers(
    fetch: &dyn Fetch,
    base: &str,
    repository: &str,
    token: Option<&str>,
) -> Result<(Vec<u64>, Vec<u64>), String> {
    let (mut issues, mut pulls) = (Vec::new(), Vec::new());
    for page in 1..=MAX_PAGES {
        let url =
            format!("{base}/repos/{repository}/issues?state=all&per_page={PER_PAGE}&page={page}");
        let response = fetch.get(&url, token)?;
        check_status(&response, repository, token.is_some())?;
        let items: Vec<serde_json::Value> = serde_json::from_str(&response.body)
            .map_err(|e| format!("GitHub's answer for page {page} was not a JSON list: {e}"))?;
        for item in &items {
            let number = item
                .get("number")
                .and_then(|n| n.as_u64())
                .ok_or_else(|| format!("an entry on page {page} carries no number"))?;
            if item.get("pull_request").is_some() {
                pulls.push(number);
            } else {
                issues.push(number);
            }
        }
        let more = response
            .header("link")
            .is_some_and(|l| l.contains("rel=\"next\""));
        if items.is_empty() || !more {
            return Ok((issues, pulls));
        }
    }
    Err(format!(
        "the listing of {repository} did not end after {MAX_PAGES} pages"
    ))
}

fn check_status(r: &Response, repository: &str, with_token: bool) -> Result<(), String> {
    match r.status {
        200 => Ok(()),
        401 => {
            Err("GitHub rejected the token (HTTP 401); check GITHUB_TOKEN or GH_TOKEN".to_string())
        }
        403 | 429
            if r.header("x-ratelimit-remaining") == Some("0")
                || r.header("retry-after").is_some() =>
        {
            let reset = r
                .header("x-ratelimit-reset")
                .map(|t| format!(", resetting at unix time {t}"))
                .or_else(|| {
                    r.header("retry-after")
                        .map(|s| format!(", retry after {s} seconds"))
                })
                .unwrap_or_default();
            let hint = if with_token {
                ""
            } else {
                "; set GITHUB_TOKEN or GH_TOKEN for a higher limit"
            };
            Err(format!(
                "GitHub's rate limit is exhausted (HTTP {}{reset}){hint}; nothing was written",
                r.status
            ))
        }
        403 => Err(
            "GitHub refused the request (HTTP 403); the token may lack read access to issues"
                .to_string(),
        ),
        404 => Err(format!(
            "GitHub reports no repository '{repository}' visible to this request (HTTP 404){}",
            if with_token {
                "; check the repository name and that the token can read it"
            } else {
                "; if it is private, set GITHUB_TOKEN or GH_TOKEN"
            }
        )),
        other => Err(format!("GitHub answered HTTP {other}; nothing was written")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const BASE: &str = "https://api.test";

    fn fixture(name: &str) -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/github")
            .join(name);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
            .replace("{{BASE}}", BASE)
    }

    /// Replays recorded responses in order and remembers what was asked.
    struct Replay {
        responses: RefCell<Vec<String>>,
        asked: RefCell<Vec<(String, Option<String>)>>,
    }

    impl Replay {
        fn of(names: &[&str]) -> Self {
            Self {
                responses: RefCell::new(names.iter().rev().map(|n| fixture(n)).collect()),
                asked: RefCell::new(Vec::new()),
            }
        }
    }

    impl Fetch for Replay {
        fn get(&self, url: &str, token: Option<&str>) -> Result<Response, String> {
            self.asked
                .borrow_mut()
                .push((url.to_string(), token.map(String::from)));
            let raw = self
                .responses
                .borrow_mut()
                .pop()
                .expect("more requests than recorded responses");
            parse_response(&raw)
        }
    }

    #[test]
    fn pages_are_followed_until_the_listing_ends_and_split_by_kind() {
        let replay = Replay::of(&[
            "issues-page1.http",
            "issues-page2.http",
            "issues-page3.http",
        ]);
        let (issues, pulls) = collect_numbers(&replay, BASE, "urzuahq/urzua", Some("tok")).unwrap();
        assert_eq!(issues, vec![217]);
        assert_eq!(pulls, vec![224, 223, 222, 221, 220, 219, 218, 216]);
        let asked = replay.asked.borrow();
        assert_eq!(asked.len(), 3);
        assert_eq!(
            asked[0].0,
            "https://api.test/repos/urzuahq/urzua/issues?state=all&per_page=100&page=1"
        );
        assert!(asked[2].0.ends_with("&page=3"));
        assert!(asked.iter().all(|(_, t)| t.as_deref() == Some("tok")));
    }

    #[test]
    fn the_snapshot_a_refresh_builds_is_sorted_whatever_order_the_api_used() {
        let replay = Replay::of(&[
            "issues-page1.http",
            "issues-page2.http",
            "issues-page3.http",
        ]);
        let (issues, pulls) = collect_numbers(&replay, BASE, "urzuahq/urzua", None).unwrap();
        let snap = RefsSnapshot::new("urzuahq/urzua", "2026-11-04", issues, pulls);
        assert_eq!(snap.pulls, vec![216, 218, 219, 220, 221, 222, 223, 224]);
        assert_eq!(snap.newest(), Some(224));
    }

    #[test]
    fn a_rate_limited_answer_fails_with_the_reset_time_and_writes_nothing() {
        let replay = Replay::of(&["issues-page1.http", "rate-limited.http"]);
        let err = collect_numbers(&replay, BASE, "urzuahq/urzua", None).unwrap_err();
        assert!(err.contains("rate limit"), "{err}");
        assert!(err.contains("1793792528"), "{err}");
        assert!(err.contains("GITHUB_TOKEN"), "{err}");
        assert!(err.contains("nothing was written"), "{err}");
    }

    #[test]
    fn a_missing_repository_and_a_bad_token_are_named() {
        let err = collect_numbers(&Replay::of(&["not-found.http"]), BASE, "o/r", None).unwrap_err();
        assert!(err.contains("'o/r'") && err.contains("private"), "{err}");
        let err =
            collect_numbers(&Replay::of(&["not-found.http"]), BASE, "o/r", Some("t")).unwrap_err();
        assert!(err.contains("token can read it"), "{err}");
        let err = collect_numbers(
            &Replay::of(&["bad-credentials.http"]),
            BASE,
            "o/r",
            Some("t"),
        )
        .unwrap_err();
        assert!(err.contains("rejected the token"), "{err}");
    }

    #[test]
    fn a_transport_failure_is_passed_up() {
        struct Down;
        impl Fetch for Down {
            fn get(&self, _: &str, _: Option<&str>) -> Result<Response, String> {
                Err("could not reach x: curl exit status: 6: Could not resolve host".to_string())
            }
        }
        let err = collect_numbers(&Down, BASE, "o/r", None).unwrap_err();
        assert!(err.contains("Could not resolve host"), "{err}");
    }

    #[test]
    fn curl_output_with_a_proxy_preamble_parses_as_the_last_response() {
        let raw = "HTTP/1.1 200 Connection established\r\n\r\nHTTP/2 200\r\nLink: <x>; rel=\"next\"\r\nX-Ratelimit-Remaining: 7\r\n\r\n[]";
        let r = parse_response(raw).unwrap();
        assert_eq!(r.status, 200);
        assert_eq!(r.header("x-ratelimit-remaining"), Some("7"));
        assert_eq!(r.body, "[]");
        assert!(parse_response("not http").is_err());
        assert!(parse_response("HTTP/1.1 200 OK\r\nA: b").is_err());
    }
}
