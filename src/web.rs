use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread;

use serde_json::{Value, json};

use crate::binary_patch;
use crate::buddy::{
    SearchFilters, SearchParams, default_salt, detect_user_id, parse_min_stat, render_blink_sprite,
    render_face, render_sprite, render_sprite_frames, roll_with_salt, search_salts,
};
use crate::state::{get_recorded_original_salt, get_state_file_path, record_original_salt};

pub struct HttpResponse {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

pub async fn serve(port: u16) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|error| error.to_string())?;
    println!("Claude Buddy Changer running at http://127.0.0.1:{port}");

    for stream in listener.incoming().flatten() {
        thread::spawn(move || {
            let _ = handle_stream(stream);
        });
    }

    Ok(())
}

pub fn handle_http_request(method: &str, path: &str, body: &[u8]) -> HttpResponse {
    match try_handle_http_request(method, path, body) {
        Ok(response) => response,
        Err(error) => json_response(error.status, json!({ "error": error.message })),
    }
}

fn try_handle_http_request(
    method: &str,
    path: &str,
    body: &[u8],
) -> Result<HttpResponse, ApiError> {
    if path.starts_with("/api/") {
        return handle_api_request(method, path, body);
    }

    if method == "GET" && (path == "/" || path == "/index.html") {
        let html = fs::read_to_string("index.html")
            .or_else(|_| {
                fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("index.html"))
            })
            .map_err(internal_error)?;
        return Ok(HttpResponse {
            status: 200,
            content_type: "text/html; charset=utf-8",
            body: html.into_bytes(),
        });
    }

    Ok(HttpResponse {
        status: 404,
        content_type: "text/plain; charset=utf-8",
        body: b"Not found".to_vec(),
    })
}

fn handle_api_request(method: &str, path: &str, body: &[u8]) -> Result<HttpResponse, ApiError> {
    let response = match (method, path) {
        ("GET", "/api/meta") => {
            let (detected_user_id, detection_error) = match detect_user_id() {
                Ok(user_id) => (user_id, Value::Null),
                Err(error) => (String::new(), Value::String(error)),
            };
            let binary = binary_patch::detect_binary_salt(None);
            json!({
                "defaultSalt": default_salt(),
                "species": crate::assets::assets().species,
                "rarities": crate::assets::assets().rarities,
                "eyes": crate::assets::assets().eyes,
                "hats": crate::assets::assets().hats,
                "statNames": crate::assets::assets().stat_names,
                "detectedUserId": detected_user_id,
                "detectionError": detection_error,
                "binary": binary.map(|entry| json!({
                    "path": entry.file_path.display().to_string(),
                    "currentSalt": entry.salt,
                    "saltLength": entry.length,
                    "originalSaltRecorded": get_recorded_original_salt(&entry.file_path.display().to_string()),
                    "stateFile": get_state_file_path().display().to_string(),
                })).unwrap_or(Value::Null),
            })
        }
        ("POST", "/api/preview") => {
            let body = parse_json_body(body)?;
            let user_id = body_string(&body, "userId")
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .map(Ok)
                .unwrap_or_else(|| detect_user_id().map_err(ApiError::internal))?;
            let salt = body_string(&body, "salt")
                .filter(|value| !value.is_empty())
                .unwrap_or(default_salt())
                .to_string();
            let buddy = roll_with_salt(&user_id, &salt);
            json!({
                "userId": user_id,
                "salt": salt,
                "buddy": buddy,
                "face": render_face(&buddy),
                "sprite": render_sprite(&buddy, 0),
                "spriteFrames": render_sprite_frames(&buddy),
                "blinkFrame": render_blink_sprite(&buddy, 0),
            })
        }
        ("POST", "/api/search") => {
            let body = parse_json_body(body)?;
            let user_id = body_string(&body, "userId")
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .map(Ok)
                .unwrap_or_else(|| detect_user_id().map_err(ApiError::internal))?;
            let total = body_number(&body, "total").unwrap_or(100_000) as usize;
            let prefix = body_string(&body, "prefix").unwrap_or("lab-").to_string();
            let length =
                body_number(&body, "length").unwrap_or(default_salt().len() as u64) as usize;
            let min_stat = parse_min_stat(body_string(&body, "minStat").unwrap_or_default())
                .map_err(ApiError::internal)?;
            let filters = SearchFilters {
                species: body_string(&body, "species")
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                rarity: body_string(&body, "rarity")
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                eye: body_string(&body, "eye")
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                hat: body_string(&body, "hat")
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                shiny: body.get("shiny").and_then(Value::as_bool).unwrap_or(false),
                min_stat,
            };

            let matches = search_salts(SearchParams {
                user_id: user_id.clone(),
                total,
                prefix,
                length,
                filters,
                max_matches: 24,
            });

            let payload = matches
                .into_iter()
                .map(|entry| {
                    json!({
                        "salt": entry.salt,
                        "buddy": entry.buddy.clone(),
                        "face": render_face(&entry.buddy),
                        "sprite": render_sprite(&entry.buddy, 0),
                        "spriteFrames": render_sprite_frames(&entry.buddy),
                        "blinkFrame": render_blink_sprite(&entry.buddy, 0),
                    })
                })
                .collect::<Vec<_>>();

            json!({
                "userId": user_id,
                "searched": total,
                "matches": payload,
            })
        }
        ("GET", "/api/binary") => {
            let detected = binary_patch::detect_binary_salt(None);
            json!({
                "binary": detected.map(|entry| json!({
                    "path": entry.file_path.display().to_string(),
                    "currentSalt": entry.salt,
                    "saltLength": entry.length,
                    "originalSaltRecorded": get_recorded_original_salt(&entry.file_path.display().to_string()),
                })).unwrap_or(Value::Null),
                "stateFile": get_state_file_path().display().to_string(),
            })
        }
        ("POST", "/api/apply") => {
            let body = parse_json_body(body)?;
            let salt = body_string(&body, "salt")
                .filter(|value| !value.is_empty())
                .ok_or_else(|| ApiError::bad_request("salt is required"))?;
            let binary_path = body_string(&body, "binaryPath");
            let resolved_binary_path = binary_patch::resolve_binary_path(binary_path)
                .ok_or_else(|| ApiError::bad_request("Could not find claude binary."))?;
            let detected =
                binary_patch::detect_binary_salt(Some(&resolved_binary_path.display().to_string()))
                    .ok_or_else(|| {
                        ApiError::bad_request(
                            "Could not detect current salt in Claude Code binary.",
                        )
                    })?;
            record_original_salt(&resolved_binary_path.display().to_string(), &detected.salt)
                .map_err(ApiError::internal)?;
            let result =
                binary_patch::apply_binary(salt, Some(&resolved_binary_path.display().to_string()))
                    .map_err(ApiError::internal)?;
            json!({
                "ok": true,
                "filePath": result.file_path,
                "patchCount": result.patch_count,
                "oldSalt": result.old_salt,
                "newSalt": result.new_salt,
                "originalSaltRecorded": get_recorded_original_salt(&resolved_binary_path.display().to_string()),
            })
        }
        ("POST", "/api/restore") => {
            let body = parse_json_body(body)?;
            let binary_path = body_string(&body, "binaryPath");
            let resolved_binary_path = binary_patch::resolve_binary_path(binary_path)
                .ok_or_else(|| ApiError::bad_request("Could not find claude binary."))?;
            let original_salt =
                get_recorded_original_salt(&resolved_binary_path.display().to_string())
                    .ok_or_else(|| {
                        ApiError::bad_request("No recorded original salt found for this binary.")
                    })?;
            let result = binary_patch::restore_binary(
                &original_salt,
                Some(&resolved_binary_path.display().to_string()),
            )
            .map_err(ApiError::internal)?;
            json!({
                "ok": true,
                "filePath": result.file_path,
                "patchCount": result.patch_count,
                "previousSalt": result.previous_salt,
                "restoredSalt": result.restored_salt,
                "originalSaltRecorded": original_salt,
            })
        }
        _ => return Ok(json_response(404, json!({ "error": "Not found" }))),
    };

    Ok(json_response(200, response))
}

fn parse_json_body(body: &[u8]) -> Result<Value, ApiError> {
    if body.is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_slice(body).map_err(internal_error)
}

fn json_response(status: u16, payload: Value) -> HttpResponse {
    HttpResponse {
        status,
        content_type: "application/json; charset=utf-8",
        body: serde_json::to_vec(&payload).expect("JSON response should serialize"),
    }
}

fn body_string<'a>(body: &'a Value, key: &str) -> Option<&'a str> {
    body.get(key).and_then(Value::as_str)
}

fn body_number(body: &Value, key: &str) -> Option<u64> {
    body.get(key).and_then(Value::as_u64)
}

fn internal_error(error: impl ToString) -> ApiError {
    ApiError::internal(error.to_string())
}

struct ApiError {
    status: u16,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: 400,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: 500,
            message: message.into(),
        }
    }
}

fn handle_stream(mut stream: TcpStream) -> std::io::Result<()> {
    let request = read_request(&mut stream)?;
    let response = handle_http_request(&request.method, &request.path, &request.body);
    write_response(&mut stream, response)?;
    Ok(())
}

fn write_response(stream: &mut TcpStream, response: HttpResponse) -> std::io::Result<()> {
    let reason = match response.status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    };
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        reason,
        response.content_type,
        response.body.len()
    )?;
    stream.write_all(&response.body)?;
    Ok(())
}

struct ParsedRequest {
    method: String,
    path: String,
    body: Vec<u8>,
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<ParsedRequest> {
    let mut buffer = Vec::new();
    let mut temp = [0_u8; 1024];
    let header_end;
    loop {
        let read = stream.read(&mut temp)?;
        if read == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "unexpected EOF",
            ));
        }
        buffer.extend_from_slice(&temp[..read]);
        if let Some(position) = find_header_end(&buffer) {
            header_end = position;
            break;
        }
    }

    let header_bytes = &buffer[..header_end];
    let header_text = String::from_utf8_lossy(header_bytes);
    let mut lines = header_text.lines();
    let request_line = lines.next().unwrap_or_default();
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default().to_string();
    let path = request_parts.next().unwrap_or("/").to_string();

    let mut headers = std::collections::HashMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = buffer[header_end + 4..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut temp)?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&temp[..read]);
    }
    body.truncate(content_length);

    Ok(ParsedRequest { method, path, body })
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}
