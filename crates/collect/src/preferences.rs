//! Settings that outlive one command line.
//!
//! Where the storage repository lives is a property of the installation, not of
//! each invocation, so it is written down once and read by every command. A flag
//! still wins when given: the file says what is normally wanted, the flag says
//! what is wanted this time.

use std::{collections::BTreeMap, fs, path::Path};

const FILE: &str = "config";

#[derive(Debug, Default, Clone)]
pub struct Preferences {
    values: BTreeMap<String, String>,
}

impl Preferences {
    /// Reads the configuration file, treating an absent or unreadable one as
    /// empty. A missing file is the ordinary state before anything is configured,
    /// and is not worth failing a collection over.
    pub fn load(state_dir: &Path) -> Self {
        let Ok(body) = fs::read_to_string(state_dir.join(FILE)) else {
            return Self::default();
        };
        Self::parse(&body)
    }

    pub fn parse(body: &str) -> Self {
        let mut values = BTreeMap::new();
        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            if value.is_empty() {
                continue;
            }
            values.insert(key.trim().to_owned(), value.to_owned());
        }
        Self { values }
    }

    /// The configured value for a flag, named as the flag is. Asking with the
    /// flag's own name keeps the two from drifting apart.
    pub fn flag(&self, name: &str) -> Option<&str> {
        self.values
            .get(name.trim_start_matches('-'))
            .map(String::as_str)
    }

    /// Whether a flag is switched on by configuration. Anything but a plainly
    /// negative word counts as on, because the reason to write the line at all is
    /// to turn something on.
    pub fn switch(&self, name: &str) -> bool {
        matches!(self.flag(name), Some(value) if !matches!(
            value.to_ascii_lowercase().as_str(),
            "false" | "no" | "off" | "0"
        ))
    }

    pub fn path(state_dir: &Path) -> std::path::PathBuf {
        state_dir.join(FILE)
    }
}
