use recall_core::{normalize_remote, RepoContext};
use std::path::Path;
use std::process::Command;

/// Detect the current repo context (remote, branch, languages) by shelling out
/// to git and probing for language marker files. All fields degrade to None/empty.
pub fn detect_context(cwd: &Path) -> RepoContext {
    let remote_id = git(cwd, &["remote", "get-url", "origin"]).map(|s| normalize_remote(&s));
    // `rev-parse --abbrev-ref HEAD` fails on an unborn branch (no commits yet);
    // fall back to `symbolic-ref` which works before the first commit.
    let branch = git(cwd, &["rev-parse", "--abbrev-ref", "HEAD"])
        .or_else(|| git(cwd, &["symbolic-ref", "--short", "HEAD"]));
    let languages = detect_languages(cwd);
    RepoContext {
        remote_id,
        branch,
        languages,
    }
}

fn git(cwd: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn detect_languages(cwd: &Path) -> Vec<String> {
    let checks = [
        ("Cargo.toml", "rust"),
        ("package.json", "typescript"),
        ("go.mod", "go"),
        ("pyproject.toml", "python"),
        ("requirements.txt", "python"),
    ];
    let mut langs: Vec<String> = Vec::new();
    for (file, lang) in checks {
        if cwd.join(file).exists() && !langs.iter().any(|l| l == lang) {
            langs.push(lang.to_string());
        }
    }
    langs
}

#[cfg(test)]
mod tests {
    use super::detect_context;
    use std::process::Command;

    fn git(dir: &std::path::Path, args: &[&str]) {
        let ok = Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .unwrap();
        assert!(ok.status.success(), "git {:?} failed", args);
    }

    #[test]
    fn detects_remote_branch_and_language() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        git(dir, &["init", "-q"]);
        git(
            dir,
            &["remote", "add", "origin", "git@github.com:me/app.git"],
        );
        git(dir, &["checkout", "-q", "-b", "feature/x"]);
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();

        let ctx = detect_context(dir);
        assert_eq!(ctx.remote_id.as_deref(), Some("github.com/me/app"));
        assert_eq!(ctx.branch.as_deref(), Some("feature/x"));
        assert!(ctx.languages.contains(&"rust".to_string()));
    }

    #[test]
    fn non_git_dir_yields_empty_context() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = detect_context(tmp.path());
        assert!(ctx.remote_id.is_none());
    }
}
