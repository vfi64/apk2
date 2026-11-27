mod gate;
mod proposal;
mod repository;
mod scratch;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use gate::{Failure, Report, Request, State};

const USAGE: &str =
    "usage: apk2 gate --root ROOT --identity github.com/OWNER/REPO --base COMMIT --path PATH [--path PATH ...] -- /absolute/checker [ARG ...]\n       apk2 propose --root ROOT --identity github.com/OWNER/REPO --base COMMIT --source-ref REF --proposal NAME --message TEXT --path PATH [--path PATH ...] -- /absolute/checker [ARG ...]";

fn request(
    mut args: impl Iterator<Item = OsString>,
    is_proposal: bool,
) -> Result<(Request, Option<proposal::Request>), Failure> {
    let invalid = || Failure::blocked(USAGE);

    let mut root = None;
    let mut identity = None;
    let mut base = None;
    let mut source = None;
    let mut name = None;
    let mut message = None;
    let mut paths = Vec::new();
    loop {
        let option = args.next().ok_or_else(invalid)?;
        if option == "--" {
            break;
        }
        let value = args.next().ok_or_else(invalid)?;
        match option.to_str() {
            Some("--root") if root.is_none() => root = Some(PathBuf::from(value)),
            Some("--identity") if identity.is_none() => {
                identity = Some(value.into_string().map_err(|_| invalid())?)
            }
            Some("--base") if base.is_none() => {
                base = Some(value.into_string().map_err(|_| invalid())?)
            }
            Some("--source-ref") if is_proposal && source.is_none() => {
                source = Some(value.into_string().map_err(|_| invalid())?)
            }
            Some("--proposal") if is_proposal && name.is_none() => {
                name = Some(value.into_string().map_err(|_| invalid())?)
            }
            Some("--message") if is_proposal && message.is_none() => {
                message = Some(value.into_string().map_err(|_| invalid())?)
            }
            Some("--path") => paths.push(PathBuf::from(value)),
            _ => return Err(invalid()),
        }
    }
    let checker = PathBuf::from(args.next().ok_or_else(invalid)?);
    if paths.is_empty() || !checker.is_absolute() {
        return Err(invalid());
    }

    let gate = Request {
        root: root.ok_or_else(invalid)?,
        identity: identity.ok_or_else(invalid)?,
        base: base.ok_or_else(invalid)?,
        paths,
        checker,
        checker_args: args.collect(),
    };
    let proposal = if is_proposal {
        Some(proposal::Request {
            source: source.ok_or_else(invalid)?,
            name: name.ok_or_else(invalid)?,
            message: message.ok_or_else(invalid)?,
        })
    } else {
        None
    };
    Ok((gate, proposal))
}

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let command = args.next();
    let is_proposal = command.as_ref().is_some_and(|value| value == "propose");
    let result = if is_proposal || command.as_ref().is_some_and(|value| value == "gate") {
        request(args, is_proposal).and_then(|(request, proposal)| match proposal {
            Some(proposal) => proposal::run(request, proposal),
            None => gate::run(request).map(|report| (report, None)),
        })
    } else {
        Err(Failure::blocked(USAGE))
    };
    let (report, receipt) = match result {
        Ok(result) => result,
        Err(failure) => {
            eprintln!("apk2: {}", failure.message);
            (
                Report {
                    state: failure.state,
                    base: None,
                    tree: None,
                },
                None,
            )
        }
    };

    let (name, code) = match report.state {
        State::Pass => ("PASS", 0),
        State::Rejected => ("REJECTED", 1),
        State::Error => ("ERROR", 2),
        State::Blocked => ("BLOCKED", 3),
    };
    // Values emitted as strings are validated object IDs or fixed state names.
    let json_value =
        |value: Option<&str>| value.map_or("null".to_owned(), |text| format!("\"{text}\""));
    let base = json_value(report.base.as_deref());
    let tree = json_value(report.tree.as_deref());
    if is_proposal {
        let commit = json_value(receipt.as_ref().map(|receipt| receipt.commit.as_str()));
        let outcome = json_value(receipt.as_ref().map(|receipt| receipt.outcome));
        println!("{{\"state\":\"{name}\",\"base\":{base},\"tree\":{tree},\"commit\":{commit},\"outcome\":{outcome}}}");
    } else {
        println!("{{\"state\":\"{name}\",\"base\":{base},\"tree\":{tree}}}");
    }
    ExitCode::from(code)
}
