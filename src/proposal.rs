use crate::gate::{self, Failure, Report, State};
use crate::repository::Git;

pub struct Request {
    pub source: String,
    pub name: String,
    pub message: String,
}

pub struct Receipt {
    pub commit: String,
    pub outcome: &'static str,
}

pub fn run(
    gate_request: gate::Request,
    request: Request,
) -> Result<(Report, Option<Receipt>), Failure> {
    let workspace = gate::resolve(&gate_request)?;
    let git = workspace.git();
    let target = proposal_ref(&git, &request)?;
    if git.is_symbolic_ref(&request.source)?
        || git.read_ref(&request.source)?.as_deref() != Some(workspace.base.as_str())
    {
        return Err(Failure::blocked(
            "source ref does not point directly to the expected base",
        ));
    }
    if git.is_symbolic_ref(&target)? {
        return Err(Failure::blocked("proposal ref is symbolic"));
    }

    let mut report = gate::check(&workspace, &gate_request)?;
    if !matches!(report.state, State::Pass) {
        return Ok((report, None));
    }
    let tree = report
        .tree
        .as_deref()
        .expect("the gate returns a tree on PASS");
    match publish(&git, &request, &workspace.base, tree, &target) {
        Ok(receipt) => Ok((report, Some(receipt))),
        Err(error) => {
            eprintln!("apk2: {}", error.message);
            report.state = error.state;
            Ok((report, None))
        }
    }
}

fn proposal_ref(git: &Git<'_>, request: &Request) -> Result<String, Failure> {
    if !request.source.starts_with("refs/heads/") {
        return Err(Failure::blocked(
            "source ref must name a local branch under refs/heads/",
        ));
    }
    git.validate_ref(&request.source)
        .map_err(|_| Failure::blocked("invalid source ref"))?;

    let mut name = request.name.bytes();
    if !name.next().is_some_and(|byte| byte.is_ascii_alphanumeric())
        || !name.all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    {
        return Err(Failure::blocked("invalid proposal name"));
    }
    if request.message.trim().is_empty() {
        return Err(Failure::blocked("commit message cannot be empty"));
    }
    let target = format!("refs/apk2/proposals/{}", request.name);
    git.validate_ref(&target)
        .map_err(|_| Failure::blocked("invalid proposal ref"))?;
    Ok(target)
}

fn publish(
    git: &Git<'_>,
    request: &Request,
    base: &str,
    tree: &str,
    target: &str,
) -> Result<Receipt, Failure> {
    if git.is_symbolic_ref(target)? {
        return Err(Failure::blocked("proposal ref is symbolic"));
    }
    let existing = git.read_ref(target)?;
    let (commit, outcome) = match &existing {
        Some(commit) if git.matches_commit(commit, tree, base, &request.message)? => {
            (commit.clone(), "NOOP")
        }
        Some(_) => {
            return Err(Failure::blocked(
                "a different proposal already exists at this name",
            ))
        }
        None => (git.create_commit(tree, base, &request.message)?, "DONE"),
    };

    if let Err(error) =
        git.publish_proposal(&request.source, base, target, &commit, existing.is_some())
    {
        let source_now = git.read_ref(&request.source)?;
        let target_now = git.read_ref(target)?;
        if existing.is_none() && target_now.as_deref() == Some(commit.as_str()) {
            // The subprocess failed but the commit may already be published.
            return Err(error.into());
        }
        if source_now.as_deref() != Some(base)
            || target_now != existing
            || git.is_symbolic_ref(target)?
        {
            return Err(Failure::blocked(
                "source or proposal ref changed before publication",
            ));
        }
        return Err(error.into());
    }
    Ok(Receipt { commit, outcome })
}
