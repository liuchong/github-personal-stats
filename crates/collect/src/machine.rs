use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use crate::{clock, error::CollectError};

const MACHINE_FILE: &str = "machine";

pub fn identity(state_dir: &Path) -> Result<String, CollectError> {
    let path = state_dir.join(MACHINE_FILE);

    if let Ok(existing) = fs::read_to_string(&path) {
        let trimmed = existing.trim();
        if is_usable(trimmed) {
            return Ok(trimmed.to_string());
        }
    }

    let minted = mint();
    fs::create_dir_all(state_dir).map_err(|error| unwritable(state_dir, error))?;
    fs::write(&path, format!("{minted}\n")).map_err(|error| unwritable(&path, error))?;
    Ok(minted)
}

fn mint() -> String {
    let mut bytes = [0_u8; 4];
    let random = fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .is_ok();

    if !random {
        let fallback = (clock::now() as u64) ^ (u64::from(std::process::id()) << 32);
        bytes = (fallback as u32).to_be_bytes();
    }

    let hex = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("m-{hex}")
}

fn is_usable(candidate: &str) -> bool {
    !candidate.is_empty()
        && candidate.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

fn unwritable(path: &Path, error: std::io::Error) -> CollectError {
    CollectError::Unreadable {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

pub fn state_directory(home: &Path) -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local").join("state"))
        .join("github-personal-stats")
}
