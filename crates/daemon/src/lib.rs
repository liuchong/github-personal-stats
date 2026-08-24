pub mod http;
pub mod panel;
pub mod service;
pub mod token;

use std::{
    net::{IpAddr, SocketAddr, TcpListener},
    sync::Mutex,
};

use github_personal_stats_collect::{
    CollectError, Settings, collect, presence,
    pulse::{self, PulseBatch},
    sink::Sink,
};
use github_personal_stats_core::summarise_activity;

use crate::http::{Request, Response, quote, read_request, write_response};

pub const DEFAULT_ADDRESS: &str = "127.0.0.1:7391";
pub const DEFAULT_INTERVAL_MINUTES: u64 = 30;

pub struct Daemon {
    settings: Settings,
    token: String,
    /// Where a rebuilt snapshot goes. A machine that renders its own cards writes
    /// a file; one that feeds a data repository commits and pushes.
    sink: Box<dyn Sink + Send + Sync>,
    /// Held while a snapshot is being rebuilt, so the timer and a request asking
    /// for the same work cannot write the file at once.
    writing: Mutex<()>,
}

impl Daemon {
    pub fn new(
        settings: Settings,
        sink: Box<dyn Sink + Send + Sync>,
    ) -> Result<Self, CollectError> {
        let token = token::read_or_mint(&settings.state_dir)?;
        Ok(Self {
            settings,
            token,
            sink,
            writing: Mutex::new(()),
        })
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    /// Refuses to listen anywhere but the loopback address. The daemon holds a
    /// machine's whole activity history and accepts writes, and nothing about it
    /// needs to be reachable from another host.
    pub fn listen(&self, address: &str) -> std::io::Result<TcpListener> {
        let parsed = address.parse::<SocketAddr>().map_err(|error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{address:?} is not an address and port: {error}"),
            )
        })?;
        if !is_loopback(parsed.ip()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "{address} is not a loopback address, and this daemon only serves this machine"
                ),
            ));
        }
        TcpListener::bind(parsed)
    }

    pub fn answer(&self, request: &Request) -> Response {
        let (route, query) = split_query(&request.path);
        let offered = request
            .bearer
            .clone()
            .or_else(|| query_value(query, "token"));

        match (request.method.as_str(), route) {
            // Unauthenticated on purpose: a plugin needs to know whether the
            // daemon is up before it has read the token, and the answer tells a
            // caller nothing it did not already know by connecting.
            ("GET", "/v1/health") => Response::text(200, "ok"),
            ("GET", "/") => self.guarded(offered.as_deref(), |daemon| daemon.panel()),
            ("POST", "/v1/hello") => {
                self.guarded(offered.as_deref(), |daemon| daemon.take_hello(request))
            }
            ("POST", "/v1/pulses") => {
                self.guarded(offered.as_deref(), |daemon| daemon.take_pulses(request))
            }
            ("POST", "/v1/collect") => self.guarded(offered.as_deref(), Self::rebuild),
            ("GET", "/v1/summary") => self.guarded(offered.as_deref(), Self::summary),
            ("GET", _) | ("POST", _) => Response::problem(404, "no such path"),
            _ => Response::problem(405, "that method is not answered here"),
        }
    }

    /// The token may arrive as a bearer header, which is what a plugin sends, or
    /// as a query parameter, which is the only way a browser opening the panel
    /// can present it.
    fn guarded(&self, offered: Option<&str>, work: impl FnOnce(&Self) -> Response) -> Response {
        if !token::matches(&self.token, offered) {
            return Response::problem(
                401,
                "present the token from the state directory, as a bearer header or a token query parameter",
            );
        }
        work(self)
    }

    fn panel(&self) -> Response {
        match collect(&self.settings) {
            Ok(snapshot) => {
                let totals = summarise_activity(&snapshot.days);
                Response::html(panel::page(&snapshot, &totals))
            }
            Err(error) => Response::problem(500, &error.to_string()),
        }
    }

    fn take_pulses(&self, request: &Request) -> Response {
        if request.body.is_empty() {
            return Response::problem(413, "a body that was empty or larger than allowed");
        }
        let batch = match serde_json::from_str::<PulseBatch>(&request.body) {
            Ok(batch) => batch,
            Err(error) => {
                return Response::problem(400, &format!("that is not a pulse batch: {error}"));
            }
        };
        match pulse::append(&self.settings.state_dir, &batch) {
            Ok(accepted) => Response::json(200, format!("{{\"accepted\":{accepted}}}\n")),
            Err(CollectError::Rejected { message }) => Response::problem(400, &message),
            Err(error) => Response::problem(500, &error.to_string()),
        }
    }

    /// Records that a plugin has loaded. This is not work and never becomes time;
    /// it exists so that a plugin sitting in an idle window can be told apart from
    /// one that was never loaded, which otherwise look identical.
    fn take_hello(&self, request: &Request) -> Response {
        #[derive(serde::Deserialize)]
        struct Hello {
            editor: String,
            #[serde(default)]
            version: String,
        }

        let hello = match serde_json::from_str::<Hello>(&request.body) {
            Ok(hello) => hello,
            Err(error) => {
                return Response::problem(400, &format!("that is not an announcement: {error}"));
            }
        };

        // The same naming rule as a pulse batch, because this name is written to
        // disk and compared against the one on pulses.
        let batch = PulseBatch {
            editor: hello.editor.clone(),
            pulses: vec![pulse::Pulse {
                at: 1,
                day: "1970-01-01".to_owned(),
                ext: String::new(),
                write: false,
            }],
        };
        if let Err(CollectError::Rejected { message }) = batch.validate() {
            return Response::problem(400, &message);
        }

        match presence::announce(&self.settings.state_dir, &hello.editor, &hello.version) {
            Ok(()) => Response::json(200, "{\"noted\":true}\n".to_owned()),
            Err(error) => Response::problem(500, &error.to_string()),
        }
    }

    fn rebuild(&self) -> Response {
        let _held = self.writing.lock().unwrap_or_else(|poisoned| {
            // A panic while writing leaves the file either old or new, never
            // half-written, because saving replaces it in one call.
            poisoned.into_inner()
        });
        let snapshot = match collect(&self.settings) {
            Ok(snapshot) => snapshot,
            Err(error) => return Response::problem(500, &error.to_string()),
        };
        let written = match self.sink.publish(&snapshot) {
            Ok(path) => path,
            Err(error) => return Response::problem(500, &error.to_string()),
        };
        Response::json(
            200,
            format!(
                "{{\"days\":{},\"snapshot\":{}}}\n",
                snapshot.days.len(),
                quote(&written.display().to_string())
            ),
        )
    }

    fn summary(&self) -> Response {
        let snapshot = match collect(&self.settings) {
            Ok(snapshot) => snapshot,
            Err(error) => return Response::problem(500, &error.to_string()),
        };
        let totals = summarise_activity(&snapshot.days);
        Response::json(200, panel::summary_json(&snapshot, &totals))
    }

    pub fn serve(&self, listener: &TcpListener) {
        for stream in listener.incoming() {
            let Ok(stream) = stream else {
                continue;
            };
            let response = match read_request(&stream) {
                Ok(Some(request)) => self.answer(&request),
                Ok(None) => continue,
                Err(error) => Response::problem(400, &error.to_string()),
            };
            let _ = write_response(&stream, &response);
        }
    }

    /// Rebuilds the snapshot on a timer, so a machine left alone keeps its record
    /// up to date without anyone running a command.
    pub fn rebuild_on_schedule(&self, minutes: u64) {
        loop {
            let _ = self.rebuild();
            std::thread::sleep(std::time::Duration::from_secs(minutes * 60));
        }
    }
}

fn split_query(path: &str) -> (&str, &str) {
    match path.find('?') {
        Some(index) => (&path[..index], &path[index + 1..]),
        None => (path, ""),
    }
}

fn query_value(query: &str, name: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == name).then(|| value.to_owned())
    })
}

fn is_loopback(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(value) => value.is_loopback(),
        IpAddr::V6(value) => value.is_loopback(),
    }
}
