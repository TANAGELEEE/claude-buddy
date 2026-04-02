use claude_buddy_changer::web::handle_http_request;
use serde_json::Value;

#[test]
fn meta_route_returns_expected_top_level_keys() {
    let response = handle_http_request("GET", "/api/meta", b"");
    assert_eq!(response.status, 200);
    let json = parse_json(&response.body);
    for key in [
        "defaultSalt",
        "species",
        "rarities",
        "eyes",
        "hats",
        "statNames",
        "detectedUserId",
        "detectionError",
        "binary",
    ] {
        assert!(json.get(key).is_some(), "missing key {key}");
    }
}

#[test]
fn preview_route_returns_expected_shape() {
    let response = handle_http_request(
        "POST",
        "/api/preview",
        br#"{"userId":"11111111-2222-3333-4444-555555555555","salt":"friend-2026-401"}"#,
    );
    assert_eq!(response.status, 200);
    let json = parse_json(&response.body);
    for key in [
        "userId",
        "salt",
        "buddy",
        "face",
        "sprite",
        "spriteFrames",
        "blinkFrame",
    ] {
        assert!(json.get(key).is_some(), "missing key {key}");
    }
}

#[test]
fn search_route_returns_expected_shape() {
    let response = handle_http_request(
        "POST",
        "/api/search",
        br#"{"userId":"11111111-2222-3333-4444-555555555555","total":20,"prefix":"lab-","species":"owl"}"#,
    );
    assert_eq!(response.status, 200);
    let json = parse_json(&response.body);
    assert!(json.get("userId").is_some());
    assert!(json.get("searched").is_some());
    assert!(json.get("matches").is_some());
}

#[test]
fn apply_route_rejects_missing_salt() {
    let response = handle_http_request("POST", "/api/apply", br#"{}"#);
    assert_eq!(response.status, 400);
    let json = parse_json(&response.body);
    assert_eq!(
        json.get("error").and_then(Value::as_str),
        Some("salt is required")
    );
}

#[test]
fn restore_route_rejects_missing_binary_path() {
    let response = handle_http_request(
        "POST",
        "/api/restore",
        br#"{"binaryPath":"Z:\\definitely-missing\\claude.exe"}"#,
    );
    assert_eq!(response.status, 400);
    let json = parse_json(&response.body);
    assert_eq!(
        json.get("error").and_then(Value::as_str),
        Some("Could not find claude binary.")
    );
}

#[test]
fn invalid_min_stat_surfaces_as_internal_error() {
    let response = handle_http_request(
        "POST",
        "/api/search",
        br#"{"userId":"11111111-2222-3333-4444-555555555555","minStat":"CHAOS:not-a-number"}"#,
    );
    assert_eq!(response.status, 500);
    let json = parse_json(&response.body);
    assert_eq!(
        json.get("error").and_then(Value::as_str),
        Some("Invalid min stat value: CHAOS:not-a-number")
    );
}

#[test]
fn unknown_api_route_returns_not_found_json() {
    let response = handle_http_request("GET", "/api/unknown", b"");
    assert_eq!(response.status, 404);
    let json = parse_json(&response.body);
    assert_eq!(json.get("error").and_then(Value::as_str), Some("Not found"));
}

#[test]
fn root_route_serves_static_html_shell() {
    let response = handle_http_request("GET", "/", b"");
    assert_eq!(response.status, 200);
    let body = String::from_utf8(response.body).unwrap();
    assert!(body.contains("Claude Buddy Changer"));
}

fn parse_json(body: &[u8]) -> Value {
    serde_json::from_slice(body).unwrap()
}
