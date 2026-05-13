use github_personal_stats_server::{handle_request, http_bytes};
use std::{
    env,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

fn main() -> std::io::Result<()> {
    let address =
        env::var("GITHUB_PERSONAL_STATS_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
    let listener = TcpListener::bind(address)?;

    for stream in listener.incoming() {
        handle_stream(stream?)?;
    }

    Ok(())
}

fn handle_stream(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = [0_u8; 2048];
    let read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/health");
    let response = handle_request(path);
    stream.write_all(&http_bytes(response))?;
    Ok(())
}
