mod gate;
mod repository;
mod scratch;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use gate::{Failure, Request, State};

const USAGE: &str =
    "usage: apk2 gate --root ROOT --identity github.com/OWNER/REPO --base COMMIT --path PATH [--path PATH ...] -- /absolute/checker [ARG ...]";

fn request(mut args: impl Iterator<Item = OsString>) -> Result<Request, Failure> {
    let invalid = || Failure::blocked(USAGE);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("gate")) {
        return Err(invalid());
    }

    let mut root = None;
    let mut identity = None;
    let mut base = None;
    let mut paths = Vec::new();
    loop {
        let option = args.next().ok_or_else(invalid)?;
        if option == "--" {
            break;
        }
        let value = args.next().ok_or_else(invalid)?;
        match option.to_str() {
            Some("--root") if root.is_none() => root = Some(PathBuf::from(value)),
            Some("--identity") if identity.is_none() => identity = value.into_string().ok(),
            Some("--base") if base.is_none() => base = value.into_string().ok(),
            Some("--path") => paths.push(PathBuf::from(value)),
            _ => return Err(invalid()),
        }
    }
    let checker = PathBuf::from(args.next().ok_or_else(invalid)?);
    if paths.is_empty() || !checker.is_absolute() {
        return Err(invalid());
    }

    Ok(Request {
        root: root.ok_or_else(invalid)?,
        identity: identity.ok_or_else(invalid)?,
        base: base.ok_or_else(invalid)?,
        paths,
        checker,
        checker_args: args.collect(),
    })
}

fn main() -> ExitCode {
    let report = match request(std::env::args_os().skip(1)).and_then(gate::run) {
        Ok(report) => report,
        Err(failure) => {
            eprintln!("apk2: {}", failure.message);
            gate::Report {
                state: failure.state,
                base: None,
                tree: None,
            }
        }
    };

    let (name, code) = match report.state {
        State::Pass => ("PASS", 0),
        State::Rejected => ("REJECTED", 1),
        State::Error => ("ERROR", 2),
        State::Blocked => ("BLOCKED", 3),
    };
    // Object IDs enter the report only after Git has returned validated hex IDs.
    let id_json = |id: Option<String>| id.map_or("null".to_owned(), |hex| format!("\"{hex}\""));
    println!(
        "{{\"state\":\"{name}\",\"base\":{},\"tree\":{}}}",
        id_json(report.base),
        id_json(report.tree),
    );
    ExitCode::from(code)
}
