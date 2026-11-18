//! `urzua board --serve`: a read-only, foreground, standard-library server on
//! 127.0.0.1. It answers `GET /`, `GET /stamp` (a digest the page polls),
//! `GET /check` (the `urzua check` summary, which the page does not wait for)
//! and each record's text at `GET /record/<id>`, and nothing else.
//!
//! Each connection is handled on its own thread, up to a cap; past it the
//! answer is 503 at once. The page and the export behind it are cached against
//! the `/stamp` digest, so an unchanged repository answers from memory, and one
//! `urzua check` run at a time serves every `/check` request that arrives
//! while it runs and every one after it until the digest changes. Neither the
//! page build nor a check run holds anything `/stamp` needs.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use urzua_core::report::{ExportReport, Failure, ReportStatus};

use super::board::{build_page, check_line, stamp, Listening, LAST_CHECK_SECS};
use super::board_page::is_path_id;
use crate::session::{Session, Workspace};

/// The longest request head the server reads, and its longest line.
const MAX_HEAD_BYTES: usize = 8 * 1024;
const MAX_LINE_BYTES: usize = 2 * 1024;
/// How long a client may take to send its request head, and to take the page.
const READ_BUDGET: Duration = Duration::from_secs(2);
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
/// Connections handled at once; the next gets 503.
const MAX_CONNECTIONS: usize = 32;

const TEXT: &str = "text/plain; charset=utf-8";

struct Built {
    digest: String,
    page: String,
    export: ExportReport,
}

/// The one `urzua check` run in flight, and the last result.
struct CheckState {
    running: bool,
    finished_runs: u64,
    result: Option<(String, String)>,
}

struct Served {
    config_path: Option<PathBuf>,
    allowed_hosts: Vec<String>,
    page: Mutex<Option<Built>>,
    check: Mutex<CheckState>,
    check_done: Condvar,
}

/// Marks a check run as over when it ends, however it ends.
struct RunGuard<'a>(&'a Served);

impl Drop for RunGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.check.lock() {
            state.running = false;
            state.finished_runs += 1;
        }
        self.0.check_done.notify_all();
    }
}

pub(super) fn serve(
    config_path: Option<PathBuf>,
    port: u16,
    allowed_hosts: Vec<String>,
) -> Result<ExitCode, Failure> {
    // Fails before listening when the records cannot be read at all.
    Workspace::open(&[], config_path.clone())?;
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))
        .map_err(|e| format!("could not listen on 127.0.0.1:{port}: {e}"))?;
    let address = listener
        .local_addr()
        .map_err(|e| format!("could not read the listening address: {e}"))?;
    let line = crate::report_value(&Listening {
        status: ReportStatus::Ok,
        listening: format!("http://{address}/"),
        address: address.to_string(),
        port: address.port(),
    })?;
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{line}");
    let _ = stdout.flush();
    drop(stdout);

    let served = Arc::new(Served {
        config_path,
        allowed_hosts: allowed_hosts.iter().map(|h| h.to_lowercase()).collect(),
        page: Mutex::new(None),
        check: Mutex::new(CheckState {
            running: false,
            finished_runs: 0,
            result: None,
        }),
        check_done: Condvar::new(),
    });
    let active = Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming().flatten() {
        if active.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
            active.fetch_sub(1, Ordering::SeqCst);
            refuse(stream);
            continue;
        }
        let (served, active_here) = (Arc::clone(&served), Arc::clone(&active));
        let spawned = std::thread::Builder::new().spawn({
            let stream = stream.try_clone();
            move || {
                if let Ok(stream) = stream {
                    let _ = answer(stream, &served);
                }
                active_here.fetch_sub(1, Ordering::SeqCst);
            }
        });
        if spawned.is_err() {
            active.fetch_sub(1, Ordering::SeqCst);
            refuse(stream);
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// The answer to a connection the server has no room for. It is small enough
/// for the socket's buffer, so a client that never reads cannot hold the loop.
fn refuse(mut stream: TcpStream) {
    let _ = respond(
        &mut stream,
        "503 Service Unavailable",
        TEXT,
        "Retry-After: 2\r\n",
        "too many connections\n",
    );
}

/// What a request head asked for, or why it is refused.
enum Request {
    Page,
    RecordText(String),
    Check,
    Stamp,
    NotFound,
    WrongMethod,
    Malformed,
}

struct Head {
    request: Request,
    host: Option<String>,
}

fn host_name(value: &str) -> String {
    let value = value.trim().to_lowercase();
    if let Some(rest) = value.strip_prefix('[') {
        return format!("[{}", rest.split(']').next().unwrap_or_default()) + "]";
    }
    match value.rsplit_once(':') {
        Some((name, port)) if port.chars().all(|c| c.is_ascii_digit()) => name.to_string(),
        _ => value,
    }
}

fn read_head(stream: &mut TcpStream) -> std::io::Result<Head> {
    let started = Instant::now();
    let mut head: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 1024];
    let bad = || Head {
        request: Request::Malformed,
        host: None,
    };
    while !head.windows(4).any(|w| w == b"\r\n\r\n") && !head.windows(2).any(|w| w == b"\n\n") {
        let left = READ_BUDGET.saturating_sub(started.elapsed());
        if left.is_zero() {
            return Err(std::io::ErrorKind::TimedOut.into());
        }
        stream.set_read_timeout(Some(left))?;
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        head.extend_from_slice(&chunk[..n]);
        let first_line_open = !head.contains(&b'\n');
        if head.len() > MAX_HEAD_BYTES || (first_line_open && head.len() > MAX_LINE_BYTES) {
            return Ok(bad());
        }
    }
    let text = String::from_utf8_lossy(&head);
    let mut lines = text.lines();
    let line = lines.next().unwrap_or_default();
    if line.len() > MAX_LINE_BYTES {
        return Ok(bad());
    }
    let mut parts = line.split(' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Ok(bad());
    };
    if !version.starts_with("HTTP/") {
        return Ok(bad());
    }
    let host = lines.find_map(|l| {
        let (name, value) = l.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case("host")
            .then(|| host_name(value))
    });
    let path = target.split(['?', '#']).next().unwrap_or_default();
    let request = if method != "GET" {
        Request::WrongMethod
    } else {
        match path.strip_prefix("/record/") {
            _ if path == "/" => Request::Page,
            _ if path == "/check" => Request::Check,
            _ if path == "/stamp" => Request::Stamp,
            Some(id) if is_path_id(id) => Request::RecordText(id.to_string()),
            _ => Request::NotFound,
        }
    };
    Ok(Head { request, host })
}

fn respond(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    extra: &str,
    body: &str,
) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n{extra}\r\n",
        body.len()
    );
    stream.set_write_timeout(Some(WRITE_TIMEOUT))?;
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()
}

/// A request for a page that could not be read is a 500 that says why.
fn unreadable(stream: &mut TcpStream, failure: &Failure) -> std::io::Result<()> {
    respond(
        stream,
        "500 Internal Server Error",
        TEXT,
        "",
        &format!("the records could not be read: {failure}\n"),
    )
}

/// Whether the request's `Host` is one this server answers to: always
/// `localhost` and `127.0.0.1` on any port, and the names given with
/// `--allowed-host`. With none given, every name is answered.
fn host_allowed(served: &Served, host: Option<&str>) -> bool {
    if served.allowed_hosts.is_empty() {
        return true;
    }
    host.is_some_and(|h| {
        h == "localhost" || h == "127.0.0.1" || served.allowed_hosts.iter().any(|a| a == h)
    })
}

#[cfg(feature = "test-seams")]
pub(super) fn test_delay(variable: &str) {
    if let Some(ms) = std::env::var(variable)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
    {
        std::thread::sleep(Duration::from_millis(ms));
    }
}

/// The cached page for the current digest, building it when the records or the
/// configuration changed. Requests for the page wait for one build, not for one
/// each.
fn page_for(served: &Served, digest: &str) -> Result<String, Failure> {
    let mut cache = served.page.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(built) = cache.as_ref().filter(|b| b.digest == digest) {
        return Ok(built.page.clone());
    }
    let (page, export) = build_page(&served.config_path, Some(digest))?;
    *cache = Some(Built {
        digest: digest.to_string(),
        page: page.clone(),
        export,
    });
    Ok(page)
}

/// The cached page says "which takes a while" beside its `/check` link; once a
/// run has shown how long it takes, the answer says so, without a rebuild.
fn with_check_cost(page: String) -> String {
    match LAST_CHECK_SECS.load(Ordering::Relaxed) {
        0 => page,
        secs => page.replace(
            "runs urzua check, which takes a while",
            &format!("runs urzua check, about {secs} s"),
        ),
    }
}

/// A record's text: from the cached export when it is current, else from the
/// records themselves, without building the view model. Never from a path the
/// client named.
fn text_of(served: &Served, digest: &str, id: &str) -> Result<Option<String>, Failure> {
    if let Ok(cache) = served.page.try_lock() {
        if let Some(built) = cache.as_ref().filter(|b| b.digest == digest) {
            return Ok(built
                .export
                .records
                .iter()
                .find(|r| r.id.as_deref() == Some(id))
                .and_then(|r| r.raw_text.clone()));
        }
    }
    let session = Session::open(served.config_path.clone())?;
    Ok(session
        .records
        .iter()
        .find(|r| urzua_core::rules::record_id(r).as_deref() == Some(id))
        .and_then(|r| session.full_text.get(&r.path).cloned()))
}

/// The `urzua check` summary line for the digest: from the last run when it
/// was for this digest, from the run in flight when there is one, else from a
/// run of its own, and one run at a time.
fn check_for(served: &Served, digest: &str) -> String {
    let mut state = served.check.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, text)) = &state.result {
        if at == digest {
            return text.clone();
        }
    }
    if state.running {
        let seen = state.finished_runs;
        while state.running && state.finished_runs == seen {
            state = served
                .check_done
                .wait(state)
                .unwrap_or_else(|e| e.into_inner());
        }
        return state.result.as_ref().map_or_else(
            || "urzua check: could not be run\n".to_string(),
            |r| r.1.clone(),
        );
    }
    state.running = true;
    drop(state);
    let _guard = RunGuard(served);
    let text = run_check(served);
    served
        .check
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .result = Some((digest.to_string(), text.clone()));
    text
}

fn run_check(served: &Served) -> String {
    let started = Instant::now();
    #[cfg(feature = "test-seams")]
    test_delay("URZUA_BOARD_TEST_CHECK_DELAY_MS");
    let line = check_line(&served.config_path);
    let seconds = started.elapsed().as_secs();
    LAST_CHECK_SECS.store(seconds.max(1), Ordering::Relaxed);
    match line {
        Some(line) => format!(
            "{} (computed {}, took {seconds} s)\n",
            line.text(),
            urzua_io::read_instant()
        ),
        None => "urzua check: could not be run\n".to_string(),
    }
}

fn answer(mut stream: TcpStream, served: &Served) -> std::io::Result<()> {
    let head = read_head(&mut stream)?;
    if !matches!(head.request, Request::Malformed) && !host_allowed(served, head.host.as_deref()) {
        return respond(&mut stream, "403 Forbidden", TEXT, "", "host not allowed\n");
    }
    match head.request {
        Request::Stamp => match stamp(&served.config_path).map(|s| serde_json::to_string(&s)) {
            Ok(Ok(json)) => respond(
                &mut stream,
                "200 OK",
                "application/json",
                "",
                &(json + "\n"),
            ),
            Ok(Err(_)) | Err(_) => respond(
                &mut stream,
                "500 Internal Server Error",
                TEXT,
                "",
                "the stamp could not be read\n",
            ),
        },
        Request::Page => match stamp(&served.config_path).map(|s| s.digest) {
            Ok(digest) => match page_for(served, &digest) {
                Ok(page) => respond(
                    &mut stream,
                    "200 OK",
                    "text/html; charset=utf-8",
                    "",
                    &with_check_cost(page),
                ),
                Err(failure) => unreadable(&mut stream, &failure),
            },
            Err(failure) => unreadable(&mut stream, &failure),
        },
        Request::RecordText(id) => {
            match stamp(&served.config_path).and_then(|s| text_of(served, &s.digest, &id)) {
                Ok(Some(text)) => respond(&mut stream, "200 OK", TEXT, "", &text),
                Ok(None) => respond(&mut stream, "404 Not Found", TEXT, "", "not found\n"),
                Err(failure) => unreadable(&mut stream, &failure),
            }
        }
        Request::Check => match stamp(&served.config_path) {
            Ok(s) => respond(
                &mut stream,
                "200 OK",
                TEXT,
                "",
                &check_for(served, &s.digest),
            ),
            Err(failure) => unreadable(&mut stream, &failure),
        },
        Request::NotFound => respond(&mut stream, "404 Not Found", TEXT, "", "not found\n"),
        Request::WrongMethod => respond(
            &mut stream,
            "405 Method Not Allowed",
            TEXT,
            "Allow: GET\r\n",
            "only GET is answered\n",
        ),
        Request::Malformed => respond(&mut stream, "400 Bad Request", TEXT, "", "bad request\n"),
    }
}
