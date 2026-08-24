//! Just enough HTTP to talk to a plugin on the same machine.
//!
//! The card-rendering service in the `server` crate answers GET for anyone who
//! can reach it. This one takes a body, holds a secret, and writes to disk, so it
//! is kept separate rather than grown out of that one: it binds the loopback
//! address only and refuses a request whose size it did not agree to.

use std::io::{BufRead, BufReader, Read, Write};

/// A body larger than this is refused unread. A pulse batch is a few hundred
/// bytes; anything near the limit is a mistake or a probe.
const MAX_BODY: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub bearer: Option<String>,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
}

impl Response {
    pub fn json(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "application/json; charset=utf-8",
            body: body.into(),
        }
    }

    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8",
            body: body.into(),
        }
    }

    pub fn html(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            content_type: "text/html; charset=utf-8",
            body: body.into(),
        }
    }

    pub fn problem(status: u16, message: &str) -> Self {
        Self::json(status, format!("{{\"error\":{}}}\n", quote(message)))
    }
}

/// Reads one request from anything readable. Taking the trait rather than a
/// socket is what lets the parser be exercised directly, which matters because
/// this code faces whatever a stranger sends it.
pub fn read_request(stream: impl Read) -> std::io::Result<Option<Request>> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(None);
    }

    let mut parts = line.split_whitespace();
    let Some(method) = parts.next().map(str::to_owned) else {
        return Ok(None);
    };
    let path = parts.next().unwrap_or("/").to_owned();

    let mut length = 0_usize;
    let mut bearer = None;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            break;
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        let Some((name, value)) = header.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match name.to_ascii_lowercase().as_str() {
            "content-length" => length = value.parse().unwrap_or(0),
            "authorization" => {
                bearer = value
                    .strip_prefix("Bearer ")
                    .or_else(|| value.strip_prefix("bearer "))
                    .map(str::to_owned);
            }
            _ => {}
        }
    }

    // Refused before a single byte of it is read. Reported as an error rather
    // than as an empty body, so the answer says the size was the problem instead
    // of blaming the contents that were never looked at.
    if length > MAX_BODY {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("body of {length} bytes is larger than the {MAX_BODY} permitted"),
        ));
    }

    let mut body = vec![0_u8; length];
    if length > 0 {
        reader.read_exact(&mut body)?;
    }

    Ok(Some(Request {
        method,
        path,
        bearer,
        body: String::from_utf8_lossy(&body).into_owned(),
    }))
}

pub fn write_response(mut stream: impl Write, response: &Response) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        response.status,
        reason(response.status),
        response.content_type,
        response.body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(response.body.as_bytes())?;
    stream.flush()
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        _ => "Status",
    }
}

pub fn quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if control < ' ' => out.push_str(&format!("\\u{:04x}", control as u32)),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}
