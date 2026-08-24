//! Keeping the daemon running without anyone remembering to start it.
//!
//! The service file is written from the arguments the install was asked for, so
//! what runs in the background is what was just tried by hand. The daemon is
//! given an absolute path to its own binary rather than a name on a path, because
//! a launch agent's environment is not a login shell's.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use github_personal_stats_collect::CollectError;

pub const LABEL: &str = "dev.liuchong.github-personal-stats";

pub struct Service {
    pub path: PathBuf,
    pub load: Vec<String>,
    pub unload: Vec<String>,
}

/// Where the service description belongs, and how the system is told to read it.
pub fn describe() -> Result<Service, CollectError> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| rejected("HOME is not set, so there is nowhere to install a service"))?;

    if cfg!(target_os = "macos") {
        let path = home
            .join("Library")
            .join("LaunchAgents")
            .join(format!("{LABEL}.plist"));
        let target = format!("gui/{}", user_id());
        return Ok(Service {
            load: vec![
                "bootstrap".to_owned(),
                target.clone(),
                path.display().to_string(),
            ],
            unload: vec!["bootout".to_owned(), format!("{target}/{LABEL}")],
            path,
        });
    }

    let path = home
        .join(".config")
        .join("systemd")
        .join("user")
        .join(format!("{LABEL}.service"));
    Ok(Service {
        load: vec!["--user".to_owned(), "enable".to_owned(), "--now".to_owned()],
        unload: vec![
            "--user".to_owned(),
            "disable".to_owned(),
            "--now".to_owned(),
        ],
        path,
    })
}

pub fn loader() -> &'static str {
    if cfg!(target_os = "macos") {
        "launchctl"
    } else {
        "systemctl"
    }
}

/// The service description itself. `arguments` are the ones `serve` should be
/// started with, so an install repeats whatever was just verified by hand.
pub fn contents(program: &Path, arguments: &[String], logs: &Path) -> String {
    if cfg!(target_os = "macos") {
        let mut program_arguments = vec![program.display().to_string()];
        program_arguments.push("serve".to_owned());
        program_arguments.extend(arguments.iter().cloned());
        let listed = program_arguments
            .iter()
            .map(|argument| format!("    <string>{}</string>", escape(argument)))
            .collect::<Vec<_>>()
            .join("\n");

        return format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\">\n<dict>\n\
  <key>Label</key>\n  <string>{LABEL}</string>\n\
  <key>ProgramArguments</key>\n  <array>\n{listed}\n  </array>\n\
  <key>RunAtLoad</key>\n  <true/>\n\
  <key>KeepAlive</key>\n  <true/>\n\
  <key>ProcessType</key>\n  <string>Background</string>\n\
  <key>StandardOutPath</key>\n  <string>{log}</string>\n\
  <key>StandardErrorPath</key>\n  <string>{log}</string>\n\
</dict>\n</plist>\n",
            log = escape(&logs.display().to_string()),
        );
    }

    let command = std::iter::once(program.display().to_string())
        .chain(std::iter::once("serve".to_owned()))
        .chain(arguments.iter().cloned())
        .map(|argument| shell_quote(&argument))
        .collect::<Vec<_>>()
        .join(" ");

    format!(
        "[Unit]\n\
Description=Collect local coding activity and serve a panel\n\
After=default.target\n\n\
[Service]\n\
Type=simple\n\
ExecStart={command}\n\
Restart=always\n\
RestartSec=10\n\n\
[Install]\n\
WantedBy=default.target\n"
    )
}

pub fn write(service: &Service, body: &str) -> Result<(), CollectError> {
    if let Some(parent) = service.path.parent() {
        fs::create_dir_all(parent).map_err(|error| CollectError::Unreadable {
            path: parent.to_path_buf(),
            message: error.to_string(),
        })?;
    }
    fs::write(&service.path, body).map_err(|error| CollectError::Unreadable {
        path: service.path.clone(),
        message: error.to_string(),
    })
}

pub fn remove(service: &Service) -> Result<(), CollectError> {
    match fs::remove_file(&service.path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(CollectError::Unreadable {
            path: service.path.clone(),
            message: error.to_string(),
        }),
    }
}

fn user_id() -> u32 {
    // Safe: getuid reads a process property and cannot fail.
    unsafe extern "C" {
        fn getuid() -> u32;
    }
    unsafe { getuid() }
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|character| character.is_alphanumeric() || "-_./=:".contains(character))
    {
        return value.to_owned();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn rejected(message: &str) -> CollectError {
    CollectError::Rejected {
        message: message.to_owned(),
    }
}
