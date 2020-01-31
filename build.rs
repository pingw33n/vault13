use regex::Regex;
use std::process::Command;

struct LastGitCommit {
    hash: String,
    timestamp: String,
}

impl LastGitCommit {
    fn short_hash(&self) -> &str {
        &self.hash[..9]
    }

    fn date(&self) -> &str {
        &self.timestamp[..10]
    }
}

fn last_git_commit() -> LastGitCommit {
    let output = Command::new("git")
        .args(&["log", "-1", "--format=%H %cd", "--date=format-local:%Y-%m-%dT%H:%M:%SZ"])
        .env("TZ", "UTC")
        .output().unwrap();
    let parts: Vec<_> = std::str::from_utf8(&output.stdout).unwrap().trim()
        .split(' ')
        .collect();
    assert_eq!(parts.len(), 2, "{:?}", parts);
    LastGitCommit {
        hash: parts[0].into(),
        timestamp: parts[1].into(),
    }
}

#[derive(Debug)]
enum GitVersionStatus {
    Stable,
    Dev,
    Dirty,
}

// Stable:
// 1.2.3
//
// Dev:
// 1.2.3-1-g70e989d
// 0c6cf14
//
// Dirty:
// 1.2.3-dirty
// 1.2.3-broken
// 0c6cf14-dirty
// 0c6cf14-broken
fn git_version_status() -> GitVersionStatus {
    let o = Command::new("git")
        .args(&["describe", "--always", "--dirty", "--broken"])
        .output()
        .unwrap();
    let o = std::str::from_utf8(&o.stdout).unwrap().trim();
    assert!(!o.is_empty());
    let re = Regex::new(r#"^(?P<ver>\d+\.\d+\.\d+)?(-(?P<dirty>dirty|broken))?$"#).unwrap();
    if let Some(caps) = re.captures(o) {
        let has_ver = caps.name("ver").is_some();
        let dirty = caps.name("dirty").is_some();
        match (has_ver, dirty) {
            (true, false) => {
                assert_eq!(&caps["ver"], env!("CARGO_PKG_VERSION"),
                    "version in Cargo.toml doesn't match version in Git tag");
                GitVersionStatus::Stable
            }
            (false, false) => GitVersionStatus::Dev,
            (_, true) => GitVersionStatus::Dirty,
        }
    } else {
        GitVersionStatus::Dev
    }
}

fn main() {
    let last_git_commit = last_git_commit();
    let git_version_status = git_version_status();
    println!("cargo:rustc-env=GIT_HASH={}", last_git_commit.hash);
    println!("cargo:rustc-env=GIT_SHORT_HASH={}", last_git_commit.short_hash());
    println!("cargo:rustc-env=GIT_TIMESTAMP={}", last_git_commit.timestamp);
    println!("cargo:rustc-env=GIT_DATE={}", last_git_commit.date());
    println!("cargo:rustc-env=GIT_VERSION_STATUS={:?}", git_version_status);
}