use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use claude_buddy_changer::web::{HttpResponse, handle_http_request};

fn main() {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(43123);
    let listener = TcpListener::bind(("127.0.0.1", port)).expect("server should bind");
    println!("Claude Buddy Changer running at http://127.0.0.1:{port}");

    for stream in listener.incoming().flatten() {
        thread::spawn(move || {
            let _ = handle_stream(stream);
        });
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

    let mut headers = HashMap::new();
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
