use claude_buddy_changer::web::handle_http_request;

#[test]
fn html_shell_keeps_required_frontend_hooks() {
    let response = handle_http_request("GET", "/", b"");
    assert_eq!(response.status, 200);

    let body = String::from_utf8(response.body).expect("html should be valid utf-8");

    for hook in [
        "id=\"langSelect\"",
        "id=\"userId\"",
        "id=\"salt\"",
        "id=\"previewBtn\"",
        "id=\"searchBtn\"",
        "id=\"previewContainer\"",
        "id=\"results\"",
        "id=\"binaryStatus\"",
    ] {
        assert!(body.contains(hook), "missing frontend hook {hook}");
    }
}

#[test]
fn html_shell_keeps_required_api_routes() {
    let response = handle_http_request("GET", "/", b"");
    assert_eq!(response.status, 200);

    let body = String::from_utf8(response.body).expect("html should be valid utf-8");

    for route in [
        "/api/meta",
        "/api/preview",
        "/api/search",
        "/api/binary",
        "/api/apply",
        "/api/restore",
    ] {
        assert!(body.contains(route), "missing api route reference {route}");
    }
}
