/// HTTP client/server implementation for Agave OS
use crate::sys::error::AgaveResult;
use alloc::{string::{String, ToString}, vec::Vec, collections::BTreeMap, format, boxed::Box};

/// HTTP method
#[derive(Debug, Clone, PartialEq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
}

/// HTTP status code
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HttpStatus {
    Ok = 200,
    NotFound = 404,
    InternalServerError = 500,
}

/// HTTP request
#[derive(Debug)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

/// HTTP response
#[derive(Debug)]
pub struct HttpResponse {
    pub status: HttpStatus,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn new(status: HttpStatus) -> Self {
        let mut headers = BTreeMap::new();
        headers.insert("Server".to_string(), "Agave-OS/1.0".to_string());
        headers.insert("Content-Type".to_string(), "text/html".to_string());
        
        Self {
            status,
            headers,
            body: Vec::new(),
        }
    }

    pub fn with_body(mut self, body: String) -> Self {
        self.body = body.into_bytes();
        self.headers.insert("Content-Length".to_string(), self.body.len().to_string());
        self
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut response = Vec::new();
        
        // Status line
        let status_line = format!("HTTP/1.1 {} OK\r\n", self.status as u16);
        response.extend_from_slice(status_line.as_bytes());
        
        // Headers
        for (key, value) in &self.headers {
            let header = format!("{}: {}\r\n", key, value);
            response.extend_from_slice(header.as_bytes());
        }
        
        // Empty line
        response.extend_from_slice(b"\r\n");
        
        // Body
        response.extend_from_slice(&self.body);
        
        response
    }
}

/// Simple HTTP server
pub struct HttpServer {
    port: u16,
    routes: BTreeMap<String, Box<dyn Fn(&HttpRequest) -> HttpResponse>>,
}

impl HttpServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            routes: BTreeMap::new(),
        }
    }

    pub fn route<F>(&mut self, path: &str, handler: F) 
    where 
        F: Fn(&HttpRequest) -> HttpResponse + 'static
    {
        self.routes.insert(path.to_string(), Box::new(handler));
    }

    pub fn start(&self) -> AgaveResult<()> {
        log::info!("Starting HTTP server on port {}", self.port);
        
        // TODO: Implement actual TCP socket listening
        // For now, just log that the server would be running
        log::info!("HTTP server would be listening on 0.0.0.0:{}", self.port);
        
        Ok(())
    }

    pub fn handle_request(&self, request: &HttpRequest) -> HttpResponse {
        if let Some(handler) = self.routes.get(&request.path) {
            handler(request)
        } else {
            HttpResponse::new(HttpStatus::NotFound)
                .with_body("<h1>404 Not Found</h1>".to_string())
        }
    }
}

/// HTTP client
pub struct HttpClient;

impl HttpClient {
    pub fn get(url: &str) -> AgaveResult<HttpResponse> {
        log::info!("HTTP GET request to: {}", url);
        
        // TODO: Implement actual HTTP client
        // For now, return a mock response
        let response = HttpResponse::new(HttpStatus::Ok)
            .with_body("<html><body>Mock HTTP response</body></html>".to_string());
            
        Ok(response)
    }

    pub fn post(url: &str, body: &[u8]) -> AgaveResult<HttpResponse> {
        log::info!("HTTP POST request to: {} ({} bytes)", url, body.len());
        
        // TODO: Implement actual HTTP client
        let response = HttpResponse::new(HttpStatus::Ok)
            .with_body("<html><body>POST received</body></html>".to_string());
            
        Ok(response)
    }
}

/// Demo HTTP server setup
pub fn setup_demo_server() -> AgaveResult<()> {
    let mut server = HttpServer::new(8080);
    
    // Add some demo routes
    server.route("/", |_req| {
        HttpResponse::new(HttpStatus::Ok)
            .with_body("<h1>Welcome to Agave OS!</h1><p>This is a demo HTTP server.</p>".to_string())
    });
    
    server.route("/status", |_req| {
        HttpResponse::new(HttpStatus::Ok)
            .with_body("{\"status\": \"running\", \"os\": \"Agave OS\"}".to_string())
    });
    
    server.route("/api/info", |_req| {
        let mut response = HttpResponse::new(HttpStatus::Ok);
        response.headers.insert("Content-Type".to_string(), "application/json".to_string());
        response.with_body("{\"version\": \"1.0.0\", \"kernel\": \"rust\"}".to_string())
    });
    
    server.start()
}
