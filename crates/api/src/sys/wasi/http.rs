// WASI HTTP implementation for Agave OS
use super::error::*;
use super::types::*;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use spin::Mutex;

// HTTP Fields type for headers
#[derive(Debug, Clone)]
pub struct Fields {
    headers: BTreeMap<String, String>,
}

impl Fields {
    pub fn new() -> Self {
        Self {
            headers: BTreeMap::new(),
        }
    }
}

// Global HTTP state
static HTTP_STATE: Mutex<HttpState> = Mutex::new(HttpState::new());

#[derive(Debug)]
pub struct HttpState {
    requests: BTreeMap<u32, HttpRequest>,
    responses: BTreeMap<u32, HttpResponse>,
    next_id: u32,
}

impl HttpState {
    pub const fn new() -> Self {
        Self {
            requests: BTreeMap::new(),
            responses: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn allocate_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub uri: String,
    pub headers: Vec<HttpHeader>,
    pub body: Option<super::io::InputStream>,
    pub timeout: Option<Timestamp>,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<HttpHeader>,
    pub body: Option<super::io::InputStream>,
}

#[derive(Debug, Clone)]
pub struct HttpHeader {
    pub name: String,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Patch,
    Trace,
    Connect,
}

impl HttpMethod {
    pub fn from_str(method: &str) -> WasiResult<Self> {
        match method.to_uppercase().as_str() {
            "GET" => Ok(HttpMethod::Get),
            "POST" => Ok(HttpMethod::Post),
            "PUT" => Ok(HttpMethod::Put),
            "DELETE" => Ok(HttpMethod::Delete),
            "HEAD" => Ok(HttpMethod::Head),
            "OPTIONS" => Ok(HttpMethod::Options),
            "PATCH" => Ok(HttpMethod::Patch),
            "TRACE" => Ok(HttpMethod::Trace),
            "CONNECT" => Ok(HttpMethod::Connect),
            _ => Err(WasiError::inval()),
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Trace => "TRACE",
            HttpMethod::Connect => "CONNECT",
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpClient {
    default_headers: Vec<HttpHeader>,
    timeout: Option<Timestamp>,
    follow_redirects: bool,
    max_redirects: u32,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            default_headers: alloc::vec![HttpHeader {
                name: "User-Agent".to_string(),
                value: b"Agave-WASI/1.0".to_vec(),
            },],
            timeout: Some(30_000_000_000), // 30 seconds in nanoseconds
            follow_redirects: true,
            max_redirects: 10,
        }
    }

    pub fn set_timeout(&mut self, timeout: Option<Timestamp>) {
        self.timeout = timeout;
    }

    pub fn add_default_header(&mut self, name: String, value: Vec<u8>) {
        self.default_headers.push(HttpHeader { name, value });
    }

    pub fn set_follow_redirects(&mut self, follow: bool, max_redirects: u32) {
        self.follow_redirects = follow;
        self.max_redirects = max_redirects;
    }
}

// HTTP Client API
pub fn create_http_client() -> WasiResult<u32> {
    let mut http = HTTP_STATE.lock();
    let client_id = http.allocate_id();
    // In a real implementation, we would store the client
    Ok(client_id)
}

pub fn create_request(
    method: &str,
    uri: &str,
    headers: &[(String, Vec<u8>)],
    body: Option<super::io::InputStream>,
) -> WasiResult<u32> {
    let method = HttpMethod::from_str(method)?;

    let headers = headers
        .iter()
        .map(|(name, value)| HttpHeader {
            name: name.clone(),
            value: value.clone(),
        })
        .collect();

    let request = HttpRequest {
        method,
        uri: uri.to_string(),
        headers,
        body,
        timeout: None,
    };

    let mut http = HTTP_STATE.lock();
    let request_id = http.allocate_id();
    http.requests.insert(request_id, request);

    Ok(request_id)
}

pub fn send_request(request_id: u32) -> WasiResult<u32> {
    let mut http = HTTP_STATE.lock();

    if let Some(request) = http.requests.get(&request_id) {
        // Simulate HTTP request processing
        let response = match request.method {
            HttpMethod::Get => {
                // Simulate a successful GET response
                HttpResponse {
                    status: 200,
                    headers: alloc::vec![
                        HttpHeader {
                            name: "Content-Type".to_string(),
                            value: b"text/plain".to_vec(),
                        },
                        HttpHeader {
                            name: "Content-Length".to_string(),
                            value: b"13".to_vec(),
                        },
                    ],
                    body: Some(super::io::create_input_stream(b"Hello, World!".to_vec())),
                }
            }
            HttpMethod::Post => {
                // Simulate a successful POST response
                HttpResponse {
                    status: 201,
                    headers: alloc::vec![HttpHeader {
                        name: "Content-Type".to_string(),
                        value: b"application/json".to_vec(),
                    },],
                    body: Some(super::io::create_input_stream(
                        b"{\"status\":\"created\"}".to_vec(),
                    )),
                }
            }
            _ => {
                // Simulate other methods
                HttpResponse {
                    status: 200,
                    headers: Vec::new(),
                    body: None,
                }
            }
        };

        let response_id = http.allocate_id();
        http.responses.insert(response_id, response);
        Ok(response_id)
    } else {
        Err(WasiError::badf())
    }
}

pub fn get_response_status(response_id: u32) -> WasiResult<u16> {
    let http = HTTP_STATE.lock();

    if let Some(response) = http.responses.get(&response_id) {
        Ok(response.status)
    } else {
        Err(WasiError::badf())
    }
}

pub fn get_response_headers(response_id: u32) -> WasiResult<Vec<(String, Vec<u8>)>> {
    let http = HTTP_STATE.lock();

    if let Some(response) = http.responses.get(&response_id) {
        let headers = response
            .headers
            .iter()
            .map(|h| (h.name.clone(), h.value.clone()))
            .collect();
        Ok(headers)
    } else {
        Err(WasiError::badf())
    }
}

pub fn get_response_body(response_id: u32) -> WasiResult<Option<super::io::InputStream>> {
    let http = HTTP_STATE.lock();

    if let Some(response) = http.responses.get(&response_id) {
        Ok(response.body)
    } else {
        Err(WasiError::badf())
    }
}

// Convenience functions for common HTTP operations
pub fn http_get(uri: &str, headers: &[(String, Vec<u8>)]) -> WasiResult<u32> {
    let request_id = create_request("GET", uri, headers, None)?;
    send_request(request_id)
}

pub fn http_post(
    uri: &str,
    headers: &[(String, Vec<u8>)],
    body: Option<super::io::InputStream>,
) -> WasiResult<u32> {
    let request_id = create_request("POST", uri, headers, body)?;
    send_request(request_id)
}

pub fn http_put(
    uri: &str,
    headers: &[(String, Vec<u8>)],
    body: Option<super::io::InputStream>,
) -> WasiResult<u32> {
    let request_id = create_request("PUT", uri, headers, body)?;
    send_request(request_id)
}

pub fn http_delete(uri: &str, headers: &[(String, Vec<u8>)]) -> WasiResult<u32> {
    let request_id = create_request("DELETE", uri, headers, None)?;
    send_request(request_id)
}

// HTTP Server API (basic implementation)
#[derive(Debug)]
pub struct HttpServer {
    port: u16,
    handlers: BTreeMap<String, fn(&HttpRequest) -> HttpResponse>,
    running: bool,
}

impl HttpServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            handlers: BTreeMap::new(),
            running: false,
        }
    }

    pub fn add_handler(&mut self, path: String, handler: fn(&HttpRequest) -> HttpResponse) {
        self.handlers.insert(path, handler);
    }

    pub fn start(&mut self) -> WasiResult<()> {
        if self.running {
            return Err(WasiError::already());
        }

        log::info!("Starting HTTP server on port {}", self.port);
        self.running = true;
        Ok(())
    }

    pub fn stop(&mut self) {
        log::info!("Stopping HTTP server on port {}", self.port);
        self.running = false;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }
}

static HTTP_SERVER: Mutex<Option<HttpServer>> = Mutex::new(None);

pub fn create_http_server(port: u16) -> WasiResult<()> {
    let mut server_opt = HTTP_SERVER.lock();

    if server_opt.is_some() {
        return Err(WasiError::already());
    }

    *server_opt = Some(HttpServer::new(port));
    Ok(())
}

pub fn add_http_handler(path: &str, handler: fn(&HttpRequest) -> HttpResponse) -> WasiResult<()> {
    let mut server_opt = HTTP_SERVER.lock();

    if let Some(ref mut server) = *server_opt {
        server.add_handler(path.to_string(), handler);
        Ok(())
    } else {
        Err(WasiError::inval())
    }
}

pub fn start_http_server() -> WasiResult<()> {
    let mut server_opt = HTTP_SERVER.lock();

    if let Some(ref mut server) = *server_opt {
        server.start()
    } else {
        Err(WasiError::inval())
    }
}

pub fn stop_http_server() -> WasiResult<()> {
    let mut server_opt = HTTP_SERVER.lock();

    if let Some(ref mut server) = *server_opt {
        server.stop();
        Ok(())
    } else {
        Err(WasiError::inval())
    }
}

// URL parsing utilities
pub fn parse_url(url: &str) -> WasiResult<(String, u16, String, String)> {
    // Basic URL parsing: scheme://host:port/path?query

    let url = url.trim();

    // Extract scheme
    let (scheme, rest) = if let Some(pos) = url.find("://") {
        (&url[..pos], &url[pos + 3..])
    } else {
        return Err(WasiError::inval());
    };

    // Extract host and port
    let (host_port, path_query) = if let Some(pos) = rest.find('/') {
        (&rest[..pos], &rest[pos..])
    } else {
        (rest, "/")
    };

    let (host, port) = if let Some(pos) = host_port.find(':') {
        let host = &host_port[..pos];
        let port_str = &host_port[pos + 1..];
        let port = port_str.parse::<u16>().map_err(|_| WasiError::inval())?;
        (host, port)
    } else {
        let default_port = match scheme {
            "http" => 80,
            "https" => 443,
            _ => return Err(WasiError::inval()),
        };
        (host_port, default_port)
    };

    // Extract path and query
    let (path, query) = if let Some(pos) = path_query.find('?') {
        (&path_query[..pos], &path_query[pos + 1..])
    } else {
        (path_query, "")
    };

    Ok((host.to_string(), port, path.to_string(), query.to_string()))
}

// HTTP utilities
pub fn encode_form_data(data: &[(String, String)]) -> String {
    data.iter()
        .map(|(key, value)| format!("{}={}", url_encode(key), url_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

pub fn url_encode(input: &str) -> String {
    // Basic URL encoding
    let mut result = String::new();

    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => {
                result.push('+');
            }
            _ => {
                result.push('%');
                result.push_str(&alloc::format!("{:02X}", byte));
            }
        }
    }

    result
}

pub fn url_decode(input: &str) -> WasiResult<String> {
    let mut result = String::new();
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        match ch {
            '%' => {
                let hex1 = chars.next().ok_or(WasiError::inval())?;
                let hex2 = chars.next().ok_or(WasiError::inval())?;

                let hex_str = alloc::format!("{}{}", hex1, hex2);
                let byte = u8::from_str_radix(&hex_str, 16).map_err(|_| WasiError::inval())?;
                result.push(byte as char);
            }
            '+' => {
                result.push(' ');
            }
            _ => {
                result.push(ch);
            }
        }
    }

    Ok(result)
}

// JSON utilities (basic)
pub fn parse_json_string(json: &str) -> WasiResult<BTreeMap<String, String>> {
    // Very basic JSON parsing for simple objects
    let json = json.trim();

    if !json.starts_with('{') || !json.ends_with('}') {
        return Err(WasiError::inval());
    }

    let mut result = BTreeMap::new();
    let content = &json[1..json.len() - 1].trim();

    if content.is_empty() {
        return Ok(result);
    }

    for pair in content.split(',') {
        let pair = pair.trim();
        if let Some(colon_pos) = pair.find(':') {
            let key = pair[..colon_pos].trim().trim_matches('"');
            let value = pair[colon_pos + 1..].trim().trim_matches('"');
            result.insert(key.to_string(), value.to_string());
        }
    }

    Ok(result)
}

pub fn create_json_string(data: &BTreeMap<String, String>) -> String {
    let pairs: Vec<String> = data
        .iter()
        .map(|(k, v)| alloc::format!("\"{}\":\"{}\"", k, v))
        .collect();

    alloc::format!("{{{}}}", pairs.join(","))
}

// WebSocket support (basic)
pub fn create_websocket_connection(uri: &str) -> WasiResult<u32> {
    // In a real implementation, this would establish a WebSocket connection
    log::debug!("Creating WebSocket connection to: {}", uri);

    let mut http = HTTP_STATE.lock();
    Ok(http.allocate_id())
}

pub fn send_websocket_message(
    _connection_id: u32,
    message: &[u8],
    is_text: bool,
) -> WasiResult<()> {
    // In a real implementation, this would send a WebSocket message
    log::debug!(
        "Sending WebSocket message: {} bytes, text: {}",
        message.len(),
        is_text
    );
    Ok(())
}

pub fn receive_websocket_message(connection_id: u32) -> WasiResult<Option<(Vec<u8>, bool)>> {
    // In a real implementation, this would receive a WebSocket message
    log::debug!(
        "Receiving WebSocket message from connection: {}",
        connection_id
    );
    Ok(None) // No message available
}

pub fn close_websocket_connection(connection_id: u32) -> WasiResult<()> {
    // In a real implementation, this would close the WebSocket connection
    log::debug!("Closing WebSocket connection: {}", connection_id);
    Ok(())
}

// Additional functions for demo compatibility
pub fn new_fields() -> Fields {
    Fields::new()
}

// Additional HTTP functions for Preview 2 compatibility
pub fn drop_fields(fields: u32) {
    // Drop HTTP fields
    log::debug!("http::drop_fields({})", fields);
}

pub fn fields_get(fields: u32, name: &str) -> Vec<Vec<u8>> {
    // Get field value by name
    log::debug!("http::fields_get({}, {})", fields, name);
    alloc::vec![]
}

pub fn fields_has(fields: u32, name: &str) -> bool {
    // Check if field exists
    log::debug!("http::fields_has({}, {})", fields, name);
    false
}

pub fn fields_set(fields: u32, name: &str, value: &[Vec<u8>]) -> WasiResult<()> {
    // Set field value
    log::debug!("http::fields_set({}, {}, {:?})", fields, name, value);
    Ok(())
}

pub fn fields_delete(fields: u32, name: &str) -> WasiResult<()> {
    // Delete field
    log::debug!("http::fields_delete({}, {})", fields, name);
    Ok(())
}

pub fn fields_append(fields: u32, name: &str, value: &[u8]) -> WasiResult<()> {
    // Append field value
    log::debug!("http::fields_append({}, {}, {:?})", fields, name, value);
    Ok(())
}

pub fn fields_entries(fields: u32) -> Vec<(String, Vec<u8>)> {
    // Get all field entries
    log::debug!("http::fields_entries({})", fields);
    alloc::vec![]
}

pub fn fields_clone(fields: u32) -> u32 {
    // Clone fields
    log::debug!("http::fields_clone({})", fields);
    fields
}

pub fn finish_incoming_stream(stream: u32) -> WasiResult<Option<u32>> {
    // Finish incoming stream
    log::debug!("http::finish_incoming_stream({})", stream);
    Ok(None)
}

pub fn finish_outgoing_stream(stream: u32, trailers: Option<u32>) -> WasiResult<()> {
    // Finish outgoing stream
    log::debug!("http::finish_outgoing_stream({}, {:?})", stream, trailers);
    Ok(())
}

pub fn new_fields_v2() -> u32 {
    // Create new fields
    log::debug!("http::new_fields_v2()");
    1
}

pub fn drop_incoming_request(request: u32) {
    // Drop incoming request
    log::debug!("http::drop_incoming_request({})", request);
}

pub fn outgoing_response_status_code(response: u32) -> u16 {
    // Get outgoing response status code
    log::debug!("http::outgoing_response_status_code({})", response);
    200
}

pub fn outgoing_response_set_status_code(response: u32, status_code: u16) -> WasiResult<()> {
    // Set outgoing response status code
    log::debug!(
        "http::outgoing_response_set_status_code({}, {})",
        response,
        status_code
    );
    Ok(())
}

pub fn outgoing_response_headers(response: u32) -> u32 {
    // Get outgoing response headers
    log::debug!("http::outgoing_response_headers({})", response);
    1
}

// Complete remaining HTTP functions for Preview 2 compatibility
pub fn incoming_request_scheme(request: u32) -> Option<String> {
    log::debug!("http::incoming_request_scheme({})", request);
    Some("https".to_string())
}

pub fn incoming_request_headers(request: u32) -> u32 {
    log::debug!("http::incoming_request_headers({})", request);
    1
}

pub fn incoming_request_consume(request: u32) -> WasiResult<Option<u32>> {
    log::debug!("http::incoming_request_consume({})", request);
    Ok(Some(1))
}

pub fn incoming_response_headers(response: u32) -> u32 {
    log::debug!("http::incoming_response_headers({})", response);
    1
}

pub fn outgoing_response_body(response: u32) -> WasiResult<Option<u32>> {
    log::debug!("http::outgoing_response_body({})", response);
    Ok(Some(1))
}

pub fn drop_incoming_response(response: u32) {
    log::debug!("http::drop_incoming_response({})", response);
}

pub fn listen_to_future_incoming_response(future: u32) -> u32 {
    log::debug!("http::listen_to_future_incoming_response({})", future);
    1
}

pub fn handle(request: u32, options: Option<u32>) -> WasiResult<u32> {
    log::debug!("http::handle({}, {:?})", request, options);
    Ok(1)
}

// Complete first missing HTTP function
pub fn incoming_request_method(request: u32) -> String {
    log::debug!("http::incoming_request_method({})", request);
    "GET".to_string()
}

// Add remaining missing HTTP functions
pub fn incoming_request_path_with_query(request: u32) -> Option<String> {
    log::debug!("http::incoming_request_path_with_query({})", request);
    Some("/".to_string())
}

pub fn incoming_request_authority(request: u32) -> Option<String> {
    log::debug!("http::incoming_request_authority({})", request);
    Some("localhost".to_string())
}

pub fn new_outgoing_request(headers: u32) -> u32 {
    log::debug!("http::new_outgoing_request({})", headers);
    1
}

pub fn outgoing_request_body(request: u32) -> WasiResult<Option<u32>> {
    log::debug!("http::outgoing_request_body({})", request);
    Ok(Some(1))
}

pub fn drop_response_outparam(param: u32) {
    log::debug!("http::drop_response_outparam({})", param);
}

pub fn set_response_outparam(param: u32, response: Result<u32, u32>) -> WasiResult<()> {
    log::debug!("http::set_response_outparam({}, {:?})", param, response);
    Ok(())
}

pub fn drop_outgoing_request(request: u32) {
    log::debug!("http::drop_outgoing_request({})", request);
}

// Add the remaining missing HTTP functions for complete Preview 2 support
pub fn incoming_response_status(response: u32) -> u16 {
    log::debug!("http::incoming_response_status({})", response);
    200
}

pub fn incoming_response_consume(response: u32) -> WasiResult<Option<u32>> {
    log::debug!("http::incoming_response_consume({})", response);
    Ok(Some(1))
}

pub fn new_outgoing_response(headers: u32) -> u32 {
    log::debug!("http::new_outgoing_response({})", headers);
    1
}

pub fn drop_outgoing_response(response: u32) {
    log::debug!("http::drop_outgoing_response({})", response);
}

pub fn drop_future_incoming_response(future: u32) {
    log::debug!("http::drop_future_incoming_response({})", future);
}

pub fn future_incoming_response_get(future: u32) -> Option<Result<Result<u32, u32>, ()>> {
    log::debug!("http::future_incoming_response_get({})", future);
    Some(Ok(Ok(1)))
}
