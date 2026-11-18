//! `urzua board`: one HTML page showing every record, laid out only from what
//! the configuration declares and read from the export's view model. Written to
//! a file, or served at a local address with `--serve`. Stdout stays JSON: the
//! page is a file or a response, never stdout.
//!
//! The server is a read-only, foreground, standard-library one: it answers
//! `GET /`, `GET /check` (the `urzua check` summary, which the page does not
//! wait for) and each record's text at `GET /record/<id>`, and nothing else, on
//! 127.0.0.1 only, and re-reads the records on every request.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use urzua_core::report::{
    ExportReport, Failure, FindingSeverity, Notice, ReadStamp, Report, ReportStatus,
};

use super::board_page::{is_path_id, render, CheckLine, CheckNote};
use super::check::{self, CheckArgs};
use super::export::{self, ExportOptions, RecordFilter};
use crate::emit;
use crate::session::{emit_or_not_run, Workspace};

/// The file `urzua board` writes when `--out` is not given, under the
/// repository's derived-state directory.
const DEFAULT_FILE: &str = ".urzua/cache/board.html";

/// The longest request head the server reads, and its longest line.
const MAX_HEAD_BYTES: usize = 8 * 1024;
const MAX_LINE_BYTES: usize = 2 * 1024;
/// How long a client may take to send its request head, and to take the page.
const READ_BUDGET: Duration = Duration::from_secs(5);
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);

pub struct BoardArgs {
    pub out: Option<PathBuf>,
    pub serve: bool,
    pub port: u16,
}

/// What `urzua board` printed when it wrote a file.
#[derive(serde::Serialize)]
struct BoardReport {
    status: ReportStatus,
    file: PathBuf,
    bytes: usize,
    record_count: usize,
    read: ReadStamp,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notices: Vec<Notice>,
}

impl Report for BoardReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::SUCCESS
    }
}

/// The one line `urzua board --serve` prints when it is listening.
#[derive(serde::Serialize)]
struct Listening {
    status: ReportStatus,
    listening: String,
    address: String,
    port: u16,
}

impl Report for Listening {
    fn notices(&self) -> &[Notice] {
        &[]
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::SUCCESS
    }
}

pub fn run(config_path: Option<PathBuf>, args: BoardArgs) -> ExitCode {
    emit_or_not_run(if args.serve {
        serve(config_path, args.port)
    } else {
        write_file(config_path, args.out)
    })
}

/// `urzua check` over the repository, reduced to the counts the page shows.
fn check_line(config_path: &Option<PathBuf>) -> Option<CheckLine> {
    let report = check::execute(config_path.clone(), CheckArgs::default()).ok()?;
    let count = |severity: FindingSeverity| {
        report
            .findings
            .iter()
            .filter(|f| f.waived.is_none() && f.severity == severity)
            .count()
    };
    Some(CheckLine {
        status: serde_json::to_value(report.status)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default(),
        errors: count(FindingSeverity::Error),
        warnings: count(FindingSeverity::Warning),
        infos: count(FindingSeverity::Info),
        waived: report
            .findings
            .iter()
            .filter(|f| f.waived.is_some())
            .count(),
    })
}

/// The page, and the export it was drawn from. The file carries the check
/// summary, computed when it is written. A served page does not wait for one:
/// `check` takes as long as it takes, so the page links `GET /check`.
fn build_page(
    config_path: &Option<PathBuf>,
    serving: bool,
) -> Result<(String, ExportReport), Failure> {
    let export = export::build_with(
        config_path.clone(),
        &RecordFilter::default(),
        &ExportOptions::default(),
    )?;
    let line = (!serving).then(|| check_line(config_path)).flatten();
    let note = match (&line, serving) {
        (_, true) => CheckNote::Linked,
        (Some(line), false) => CheckNote::Line(line),
        (None, false) => CheckNote::Unavailable,
    };
    Ok((render(&export, &note, serving), export))
}

/// The one line `GET /check` answers: the summary, when it was computed and
/// how long it took.
fn check_text(config_path: &Option<PathBuf>) -> String {
    let started = Instant::now();
    let line = check_line(config_path);
    let seconds = started.elapsed().as_secs();
    match line {
        Some(line) => format!(
            "{} (computed {}, took {seconds} s)\n",
            line.text(),
            urzua_io::read_instant()
        ),
        None => "urzua check: could not be run\n".to_string(),
    }
}

fn write_file(config_path: Option<PathBuf>, out: Option<PathBuf>) -> Result<ExitCode, Failure> {
    let target = match out {
        Some(path) => path,
        None => Workspace::open(&[], config_path.clone())?
            .repo_root
            .join(DEFAULT_FILE),
    };
    let (page, export) = build_page(&config_path, false)?;
    if let Some(dir) = target.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    }
    std::fs::write(&target, &page)
        .map_err(|e| format!("could not write {}: {e}", target.display()))?;
    Ok(emit(&BoardReport {
        status: ReportStatus::Ok,
        file: target,
        bytes: page.len(),
        record_count: export.record_count,
        read: export.read,
        notices: export.notices,
    }))
}

fn serve(config_path: Option<PathBuf>, port: u16) -> Result<ExitCode, Failure> {
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
    for stream in listener.incoming().flatten() {
        let _ = answer(stream, &config_path);
    }
    Ok(ExitCode::SUCCESS)
}

/// How a request head was refused, or what it asked for.
enum Request {
    Page,
    /// The text of the record with this id.
    RecordText(String),
    /// The `urzua check` summary, computed for the request.
    Check,
    NotFound,
    WrongMethod,
    Malformed,
}

/// The request line of a head read within the byte, line and time bounds.
fn read_request(stream: &mut TcpStream) -> std::io::Result<Request> {
    let started = Instant::now();
    let mut head: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 1024];
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
            return Ok(Request::Malformed);
        }
    }
    let text = String::from_utf8_lossy(&head);
    let line = text.lines().next().unwrap_or_default();
    if line.len() > MAX_LINE_BYTES {
        return Ok(Request::Malformed);
    }
    let mut parts = line.split(' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Ok(Request::Malformed);
    };
    if !version.starts_with("HTTP/") {
        return Ok(Request::Malformed);
    }
    if method != "GET" {
        return Ok(Request::WrongMethod);
    }
    let path = target.split(['?', '#']).next().unwrap_or_default();
    Ok(match path.strip_prefix("/record/") {
        _ if path == "/" => Request::Page,
        _ if path == "/check" => Request::Check,
        Some(id) if is_path_id(id) => Request::RecordText(id.to_string()),
        _ => Request::NotFound,
    })
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

/// The text of the record with this id, read from the records themselves and
/// never from a path the client named.
fn record_text(config_path: &Option<PathBuf>, id: &str) -> Result<Option<String>, Failure> {
    let export = export::build_with(
        config_path.clone(),
        &RecordFilter::default(),
        &ExportOptions::default(),
    )?;
    Ok(export
        .records
        .into_iter()
        .find(|r| r.id.as_deref() == Some(id))
        .and_then(|r| r.raw_text))
}

fn answer(mut stream: TcpStream, config_path: &Option<PathBuf>) -> std::io::Result<()> {
    const TEXT: &str = "text/plain; charset=utf-8";
    match read_request(&mut stream)? {
        Request::RecordText(id) => match record_text(config_path, &id) {
            Ok(Some(text)) => respond(&mut stream, "200 OK", TEXT, "", &text),
            Ok(None) => respond(&mut stream, "404 Not Found", TEXT, "", "not found\n"),
            Err(failure) => respond(
                &mut stream,
                "500 Internal Server Error",
                TEXT,
                "",
                &format!("the records could not be read: {failure}\n"),
            ),
        },
        Request::Check => respond(&mut stream, "200 OK", TEXT, "", &check_text(config_path)),
        Request::Page => match build_page(config_path, true) {
            Ok((page, _)) => respond(&mut stream, "200 OK", "text/html; charset=utf-8", "", &page),
            Err(failure) => respond(
                &mut stream,
                "500 Internal Server Error",
                TEXT,
                "",
                &format!("the records could not be read: {failure}\n"),
            ),
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
