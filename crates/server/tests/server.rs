use github_personal_stats_server::{handle_request, http_bytes};

#[test]
fn health_endpoint_returns_ok() {
    let response = handle_request("/health");

    assert_eq!(response.status, 200);
    assert_eq!(response.body, "ok");
}

#[test]
fn api_endpoint_returns_dashboard_svg_with_fixed_size() {
    let response = handle_request("/api?username=octo&width=640&height=320");

    assert_eq!(response.status, 200);
    assert_eq!(response.content_type, "image/svg+xml; charset=utf-8");
    assert!(response.body.contains(r#"width="640""#));
    assert!(response.body.contains(r#"height="320""#));
    assert!(response.body.contains("Streak"));
}

#[test]
fn api_card_route_returns_stats_svg() {
    let response = handle_request("/api/stats?username=octo");

    assert_eq!(response.status, 200);
    assert!(response.body.contains("Stats"));
    assert!(!response.body.contains("Languages"));
}

#[test]
fn coding_activity_preview_returns_text() {
    let response = handle_request("/api/wakatime-text");

    assert_eq!(response.status, 200);
    assert!(response.body.contains("### Coding Activity"));
    assert!(response.body.contains("Total:"));
}

#[test]
fn unknown_route_returns_not_found() {
    let response = handle_request("/missing");

    assert_eq!(response.status, 404);
    assert_eq!(response.body, "not found");
}

#[test]
fn http_bytes_include_headers_and_body() {
    let bytes = http_bytes(handle_request("/health"));
    let text = String::from_utf8(bytes).unwrap();

    assert!(text.starts_with("HTTP/1.1 200 OK"));
    assert!(text.contains("Content-Type: text/plain"));
    assert!(text.ends_with("ok"));
}
