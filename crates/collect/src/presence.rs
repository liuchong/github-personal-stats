//! Which editors have announced themselves, as distinct from which have reported
//! work.
//!
//! A plugin that has loaded into an idle window sends no pulses, because there is
//! no work to report, and that is indistinguishable from a plugin that never
//! loaded at all. An announcement closes that gap. It is kept apart from the pulse
//! journal on purpose: presence is a fact about the plugin, not about the day, and
//! must never become time worked.

use std::{collections::BTreeMap, fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::{clock, error::CollectError};

const FILE: &str = "editors.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Announcement {
    /// Which editor is reporting, matching the name it puts on its pulses.
    pub editor: String,
    /// The plugin's own version, so a stale one can be recognised.
    #[serde(default)]
    pub version: String,
    /// When it last said hello.
    #[serde(default)]
    pub at: i64,
}

pub fn path(state_dir: &Path) -> std::path::PathBuf {
    state_dir.join(FILE)
}

/// Notes that an editor is present, replacing whatever it said before. Only the
/// latest announcement per editor is worth keeping: the question is whether the
/// plugin is loaded now, not how many times it has started.
pub fn announce(state_dir: &Path, editor: &str, version: &str) -> Result<(), CollectError> {
    let mut known = read(state_dir)
        .into_iter()
        .map(|announcement| (announcement.editor.clone(), announcement))
        .collect::<BTreeMap<_, _>>();

    known.insert(
        editor.to_owned(),
        Announcement {
            editor: editor.to_owned(),
            version: version.to_owned(),
            at: clock::now(),
        },
    );

    let announcements = known.into_values().collect::<Vec<_>>();
    let body =
        serde_json::to_string_pretty(&announcements).map_err(|error| CollectError::Rejected {
            message: format!("an announcement could not be written down: {error}"),
        })?;

    fs::create_dir_all(state_dir).map_err(|error| CollectError::Unreadable {
        path: state_dir.to_path_buf(),
        message: error.to_string(),
    })?;
    fs::write(path(state_dir), body).map_err(|error| CollectError::Unreadable {
        path: path(state_dir),
        message: error.to_string(),
    })
}

/// Who has announced themselves, most recent first. An unreadable file counts as
/// nobody, because a missing announcement is exactly what it would mean anyway.
pub fn read(state_dir: &Path) -> Vec<Announcement> {
    let Ok(body) = fs::read_to_string(path(state_dir)) else {
        return Vec::new();
    };
    let mut announcements = serde_json::from_str::<Vec<Announcement>>(&body).unwrap_or_default();
    announcements.sort_by_key(|announcement| std::cmp::Reverse(announcement.at));
    announcements
}
