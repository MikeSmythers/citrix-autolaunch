use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, HOST, ORIGIN},
    Url,
};

use std::str::FromStr;

use sysinfo::System;

use super::objects::ProtoHeader;

/// Headers used for Citrix StoreFront requests\
/// **Note: These headers are required for correct StoreFront interaction**
/// - Includes common headers and optional custom headers
/// - Custom headers are used for CSRF token and Referer
/// - Custom headers are not required for all requests
/// - Accepts an optional vector of ProtoHeader objects provided by function call
/// - Returns a completed Reqwest HeaderMap object
pub fn common_headers(
    custom: Option<&Vec<ProtoHeader>>,
    base_uri: &str,
) -> Result<HeaderMap, String> {
    let base_uri: Url = match Url::parse(base_uri) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to parse base URI: {}", e)),
    };
    let host_domain = match HeaderValue::from_str(match &base_uri.domain() {
        Some(d) => d,
        None => return Err(String::from("Failed to parse domain for base URI.")),
    }) {
        Ok(h) => h,
        Err(e) => return Err(format!("Failed to create base URI header: {}", e)),
    };
    let origin_base_uri = match HeaderValue::from_str(&base_uri.to_string()) {
        Ok(h) => h,
        Err(e) => return Err(format!("Failed to create base URI header: {}", e)),
    };
    let mut headers: HeaderMap = HeaderMap::new();
    let x_citrix_isusinghttps = match HeaderName::from_str("X-Citrix-Isusinghttps") {
        Ok(h) => h,
        Err(e) => return Err(format!("Failed to create header: {}", e)),
    };
    let x_requested_with: HeaderName = match HeaderName::from_str("X-Requested-With") {
        Ok(h) => h,
        Err(e) => return Err(format!("Failed to create header: {}", e)),
    };
    headers.insert(HOST, host_domain);
    headers.insert(ORIGIN, origin_base_uri);
    headers.insert(x_citrix_isusinghttps, HeaderValue::from_static("Yes"));
    headers.insert(x_requested_with, HeaderValue::from_static("XMLHttpRequest"));
    if let Some(custom) = custom {
        // TODO: Make this use &str instead of actual headername/headervalue objects
        for ProtoHeader(name, value) in custom {
            headers.insert(name, value.clone());
        }
    }
    Ok(headers)
}

/// Check if Citrix Workspace is running
/// - Uses sysinfo crate to check for wfica32.exe process
/// - Returns true if process is found, false otherwise
pub fn ica_is_running() -> bool {
    let mut system = System::new();
    system.refresh_all();
    let processes = system.processes();
    if processes.iter().any(|(_, p)| p.name() == "wfica32.exe") {
        true
    } else {
        false
    }
}
