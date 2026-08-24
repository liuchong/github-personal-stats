//! The parser faces whatever a stranger sends to the port, so what it does with
//! malformed and hostile input is as much a requirement as the happy path.

use github_personal_stats_daemon::http::{Response, read_request, write_response};

fn parse(raw: &str) -> Option<github_personal_stats_daemon::http::Request> {
    read_request(raw.as_bytes()).expect("a byte slice does not fail to read")
}

#[test]
fn an_ordinary_post_is_read_whole() {
    let request = parse(concat!(
        "POST /v1/pulses HTTP/1.1\r\n",
        "Host: 127.0.0.1:7391\r\n",
        "Authorization: Bearer sekrit\r\n",
        "Content-Length: 9\r\n",
        "\r\n",
        "{\"a\": 1}\n"
    ))
    .expect("a well-formed request should be read");

    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/pulses");
    assert_eq!(request.bearer.as_deref(), Some("sekrit"));
    assert_eq!(request.body, "{\"a\": 1}\n");
}

#[test]
fn a_get_without_a_body_is_read() {
    let request = parse("GET /v1/panel HTTP/1.1\r\nHost: x\r\n\r\n").expect("a GET should be read");

    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/v1/panel");
    assert_eq!(request.bearer, None);
    assert!(request.body.is_empty());
}

#[test]
fn header_names_are_matched_regardless_of_case() {
    // Nothing guarantees a client spells them the way we would.
    let request = parse(concat!(
        "POST /v1/pulses HTTP/1.1\r\n",
        "AUTHORIZATION: Bearer shouty\r\n",
        "content-length: 2\r\n",
        "\r\n",
        "hi"
    ))
    .expect("case should not decide whether a header is seen");

    assert_eq!(request.bearer.as_deref(), Some("shouty"));
    assert_eq!(request.body, "hi");
}

#[test]
fn an_authorization_that_is_not_a_bearer_token_is_not_taken_as_one() {
    let request = parse("GET / HTTP/1.1\r\nAuthorization: Basic aGk=\r\n\r\n").unwrap();

    assert_eq!(request.bearer, None);
}

#[test]
fn a_closed_connection_that_said_nothing_is_not_an_error() {
    // Port scanners and health checks connect and leave.
    assert!(parse("").is_none());
}

#[test]
fn a_request_line_missing_its_path_is_read_as_naming_the_root() {
    // Leniently, so a malformed line gets a routed answer rather than a dropped
    // connection. Nothing is served at the root, so it still goes nowhere.
    let request = parse("GET\r\n\r\n").expect("a bare method should still parse");

    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/");
}

#[test]
fn a_body_larger_than_agreed_is_refused_rather_than_read() {
    let raw = format!(
        "POST /v1/pulses HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
        256 * 1024 + 1
    );

    assert!(
        read_request(raw.as_bytes()).is_err(),
        "an oversized body should be refused"
    );
}

#[test]
fn a_body_at_the_limit_is_still_accepted() {
    let size = 256 * 1024;
    let raw = format!(
        "POST /v1/pulses HTTP/1.1\r\nContent-Length: {size}\r\n\r\n{}",
        "x".repeat(size)
    );

    let request = read_request(raw.as_bytes())
        .expect("the limit is a limit, not one below")
        .expect("a request of exactly the permitted size should be read");

    assert_eq!(request.body.len(), size);
}

#[test]
fn a_content_length_that_is_not_a_number_reads_as_no_body() {
    let request = parse("POST /v1/pulses HTTP/1.1\r\nContent-Length: soon\r\n\r\n").unwrap();

    assert!(request.body.is_empty());
}

#[test]
fn a_body_shorter_than_promised_does_not_hang_forever() {
    // The reader is bounded by what it is given, so a truncated stream ends.
    let outcome = read_request("POST /p HTTP/1.1\r\nContent-Length: 100\r\n\r\nshort".as_bytes());

    assert!(outcome.is_err() || outcome.is_ok(), "it must return at all");
}

#[test]
fn a_header_with_no_colon_is_skipped_rather_than_fatal() {
    let request = parse("GET /v1/panel HTTP/1.1\r\nnonsense\r\nAuthorization: Bearer t\r\n\r\n")
        .expect("one bad header should not lose the request");

    assert_eq!(request.bearer.as_deref(), Some("t"));
}

#[test]
fn a_response_is_written_with_its_status_type_and_length() {
    let mut written = Vec::new();
    write_response(&mut written, &Response::json(200, "{\"ok\":true}")).unwrap();
    let written = String::from_utf8(written).unwrap();

    assert!(written.starts_with("HTTP/1.1 200"), "{written}");
    assert!(
        written.contains("Content-Type: application/json"),
        "{written}"
    );
    assert!(written.contains("Content-Length: 11"), "{written}");
    assert!(written.ends_with("{\"ok\":true}"), "{written}");
}

#[test]
fn each_kind_of_response_names_its_own_type() {
    let kinds = [
        (Response::json(200, "{}"), "application/json"),
        (Response::text(200, "hi"), "text/plain"),
        (Response::html("<p>hi</p>"), "text/html"),
    ];

    for (response, expected) in kinds {
        assert!(
            response.content_type.starts_with(expected),
            "{} should be {expected}",
            response.content_type
        );
    }
}

#[test]
fn a_problem_carries_its_status_and_says_what_went_wrong() {
    let response = Response::problem(401, "who are you");

    assert_eq!(response.status, 401);
    assert!(response.body.contains("who are you"), "{}", response.body);
}

#[test]
fn a_length_is_counted_in_bytes_not_characters() {
    // A body with anything outside ASCII would be cut short by a wrong count.
    let mut written = Vec::new();
    write_response(&mut written, &Response::text(200, "héllo")).unwrap();
    let written = String::from_utf8(written).unwrap();

    assert!(written.contains("Content-Length: 6"), "{written}");
}
