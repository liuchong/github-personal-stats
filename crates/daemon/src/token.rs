//! The shared secret a plugin presents when it reports.
//!
//! Binding to the loopback address keeps other machines out, but not other
//! programs on this one, and the daemon writes to disk on request. The token is a
//! file both sides can read, so a plugin needs no configuration beyond finding
//! the state directory, and anything that cannot read that file cannot write to
//! the journal.

use std::{fs, path::Path};

use github_personal_stats_collect::{CollectError, random};

const TOKEN_FILE: &str = "token";
const TOKEN_BYTES: usize = 32;

pub fn path(state_dir: &Path) -> std::path::PathBuf {
    state_dir.join(TOKEN_FILE)
}

/// Reads the token, minting one on first run. The file is created readable only
/// by its owner; if it already exists with wider permissions that is left alone
/// rather than silently changed, because the user may have shared it on purpose.
pub fn read_or_mint(state_dir: &Path) -> Result<String, CollectError> {
    let path = path(state_dir);

    if let Ok(existing) = fs::read_to_string(&path) {
        let trimmed = existing.trim();
        if trimmed.len() == TOKEN_BYTES * 2 {
            return Ok(trimmed.to_owned());
        }
    }

    let minted = random::hex(TOKEN_BYTES);
    fs::create_dir_all(state_dir).map_err(|error| unreadable(state_dir, error))?;
    fs::write(&path, format!("{minted}\n")).map_err(|error| unreadable(&path, error))?;
    restrict(&path);
    Ok(minted)
}

/// Length-independent comparison, so a wrong guess reveals nothing about how much
/// of it was right.
pub fn matches(expected: &str, offered: Option<&str>) -> bool {
    let Some(offered) = offered else {
        return false;
    };
    if offered.len() != expected.len() {
        return false;
    }
    expected
        .bytes()
        .zip(offered.bytes())
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(unix)]
fn restrict(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict(_path: &Path) {}

fn unreadable(path: &Path, error: std::io::Error) -> CollectError {
    CollectError::Unreadable {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}
