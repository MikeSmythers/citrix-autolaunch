use crate::{
    extract::{get_attribute_value, get_cookie_value, get_header_attribute},
    storage::{save_ica, Settings},
};
use reqwest::{
    blocking::Client,
    cookie::Jar,
    header::{HeaderMap, HeaderName, HeaderValue, CONTENT_LENGTH, HOST, ORIGIN, REFERER},
    Url,
};
use serde::Deserialize;
use std::{
    str::{from_utf8, FromStr},
    sync::Arc,
};
use sysinfo::System;

/// Simplified header object for Reqwest
struct ProtoHeader(HeaderName, HeaderValue);

/// Response object from Citrix StoreFront
// Commented items are likely present but not used
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceList {
    // is_subscription_enabled: Option<bool>,
    // is_unauthenticated_user: Option<bool>,
    resources: Option<Vec<Resource>>, // List of resources from Citrix StoreFront
}

/// Resource object from Citrix StoreFront
// Commented items are likely present but not used
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Resource {
    // clienttypes: Option<Vec<String>>,
    // description: Option<String>,
    // iconurl: Option<String>,
    // id: Option<String>,
    // launchstatusurl: Option<String>,
    // path: Option<String>,
    // shortcutvalidationurl: Option<String>,
    // subscriptionurl: Option<String>,
    launchurl: Option<String>, // Part of URL for ICA file download
    name: Option<String>,      // Name of resource as seen in Citrix StoreFront
}

/// Headers used for Citrix StoreFront requests\
/// **Note: These headers are required for correct StoreFront interaction**
/// - Includes common headers and optional custom headers
/// - Custom headers are used for CSRF token and Referer
/// - Custom headers are not required for all requests
/// - Accepts an optional vector of ProtoHeader objects provided by function call
/// - Returns a completed Reqwest HeaderMap object
fn common_headers(
    custom: Option<&Vec<ProtoHeader>>,
    settings: &Settings,
) -> Result<HeaderMap, String> {
    let base_uri: Url = match Url::parse(&settings.base_uri) {
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
        for ProtoHeader(name, value) in custom {
            headers.insert(name, value.clone());
        }
    }
    Ok(headers)
}

// TODO: Add URL builder function to provide full URLs for below function

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

/// Get ICA file from Citrix StoreFront
/// - Uses Reqwest to interact with Citrix StoreFront
/// - Requires a Settings object with login and passwd fields
/// - Returns a Result with the file name on success, error message on failure
pub fn get_ica_file(settings: &Settings) -> Result<String, String> {
    let application_name = settings.application_name.clone();
    let base_url = match Url::parse(&settings.base_uri) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to parse base URI: {}", e)),
    };
    // Client setup
    let jar = Arc::new(Jar::default());
    let client = match Client::builder().cookie_provider(Arc::clone(&jar)).build() {
        Ok(c) => c,
        Err(e) => return Err(format!("Failed to build client: {}", e)),
    };

    // Get base URL to set up cookie jar
    let _response = match client.get(base_url.clone()).send() {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to get base URL: {}", e)),
    };

    // Post Login to get more cookies
    let uri = match base_url.join("cgi/login") {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    let credentials = &[
        ("login", settings.login.as_str()),
        ("passwd", settings.passwd.as_str()),
    ];
    let response = match client
        .post(uri)
        .headers(match common_headers(None, settings) {
            Ok(h) => h,
            Err(e) => return Err(format!("Failed to build headers: {}", e)),
        })
        .form(credentials)
        .header(CONTENT_LENGTH, "0")
        .send()
    {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to post login: {}", e)),
    };
    let initial_url = match response.url().join("./") {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };

    // Set cookies for Citrix StoreFront
    let cookie_domain = match base_url.domain() {
        Some(d) => d,
        None => return Err("Failed to parse domain".to_string()),
    };
    jar.add_cookie_str(
        &format!(
            "CtxsDeviceId=CITRIX_AUTOLAUNCH; Domain={}; Path={}",
            cookie_domain,
            initial_url.path()
        ),
        &initial_url,
    );
    jar.add_cookie_str(
        &format!(
            "CtxsClientDetectionDone=true; Domain={}; Path={}",
            cookie_domain,
            initial_url.path()
        ),
        &initial_url,
    );
    jar.add_cookie_str(
        &format!(
            "CtxsHasUpgradeBeenShown=true; Domain={}; Path={}",
            cookie_domain,
            initial_url.path()
        ),
        &initial_url,
    );
    jar.add_cookie_str(
        &format!(
            "CtxsUserPreferredClient=Native; Domain={}; Path={}",
            cookie_domain,
            initial_url.path()
        ),
        &initial_url,
    );

    // Get response URL to get CSRF token et al
    let uri = initial_url.clone();
    let _response = match client.get(uri).send() {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to get base URL: {}", e)),
    };

    // Post to Home/Configuration for csrf token and resource list path
    let uri = match initial_url.join("Home/Configuration") {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    let response = match client
        .post(uri)
        .headers(match common_headers(None, settings) {
            Ok(h) => h,
            Err(e) => return Err(format!("Failed to build headers: {}", e)),
        })
        .header(CONTENT_LENGTH, "0")
        .send()
    {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to post configuration: {}", e)),
    };
    let response_headers = response.headers().clone();
    let body = match response.text() {
        Ok(b) => b,
        Err(e) => return Err(format!("Failed to retrieve configuration: {}", e)),
    };
    let resource_list_path = match get_attribute_value(&body, "resourcesProxy", "listURL") {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to get resource list path: {}", e)),
    };
    let csrf_token = match get_cookie_value(&response_headers, "CsrfToken") {
        Ok(c) => c.to_string(),
        Err(e) => return Err(format!("Failed to get csrf token: {}", e)),
    };

    // Inject CSRF token into headers for further requests
    let csrf_token_header = match HeaderName::from_str("Csrf-Token") {
        Ok(h) => h,
        Err(_) => return Err("Failed to create csrf token header".to_string()),
    };
    let custom_headers: Vec<ProtoHeader> = vec![
        ProtoHeader(
            csrf_token_header,
            HeaderValue::from_str(&csrf_token).unwrap(),
        ),
        ProtoHeader(
            REFERER,
            match HeaderValue::from_str(&initial_url.to_string().as_str()) {
                Ok(h) => h,
                Err(e) => return Err(format!("Failed to create referer header: {}", e)),
            },
        ),
    ];

    // Get list (will fail) for CtxsDeviceId cookie and auth methods path
    let get_list_settings = &[("format", "json"), ("resourceDetails", "Default")];
    let uri = match initial_url.join(&resource_list_path) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    let response = match client
        .post(uri)
        .headers(match common_headers(Some(&custom_headers), settings) {
            Ok(h) => h,
            Err(e) => return Err(format!("Failed to build headers: {}", e)),
        })
        .form(get_list_settings)
        .send()
    {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to get (fake) resource list: {}", e)),
    };
    let auth_methods_path = match get_header_attribute(
        &response.headers(),
        "CitrixWebReceiver-Authenticate",
        "location",
    ) {
        Ok(a) => a,
        Err(e) => return Err(format!("Failed to get auth methods path (2): {}", e)),
    };

    // Call auth methods path to get CitrixAGBasic relative path
    let uri = match initial_url.join(&auth_methods_path) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    let response = match client
        .post(uri)
        .headers(match common_headers(Some(&custom_headers), settings) {
            Ok(h) => h,
            Err(e) => return Err(format!("Failed to build headers: {}", e)),
        })
        .header(CONTENT_LENGTH, "0")
        .send()
    {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to get auth methods: {}", e)),
    };
    let input = match response.text() {
        Ok(i) => i,
        Err(e) => return Err(format!("Failed to get auth methods: {}", e)),
    };
    // TODO: [ISSUE 6] Make this less hacky...
    let auth_login_path = match get_attribute_value(&input, "method name=\"CitrixAGBasic\"", "url")
    {
        Ok(a) => a,
        Err(e) => return Err(format!("Failed to get auth login path: {}", e)),
    };

    // Log in to get CtxsAuthId cookie
    let uri = match initial_url.join(&auth_login_path) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    let _response = match client
        .post(uri)
        .headers(match common_headers(Some(&custom_headers), settings) {
            Ok(h) => h,
            Err(e) => return Err(format!("Failed to build headers: {}", e)),
        })
        .header(CONTENT_LENGTH, "0")
        .send()
    {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to authenticate: {}", e)),
    };

    // Get list (should work) to populate ResponseList object
    let get_list_settings = &[("format", "json"), ("resourceDetails", "Default")];
    let uri = match initial_url.join(&resource_list_path) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    let response = match client
        .post(uri)
        .headers(match common_headers(Some(&custom_headers), settings) {
            Ok(h) => h,
            Err(e) => return Err(format!("Failed to build headers: {}", e)),
        })
        .form(get_list_settings)
        .send()
    {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to get resource list: {}", e)),
    };

    // Parse response into ResourceList object
    let resource_list: Vec<Resource>;
    let response_text = match response.text() {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to get response text: {}", e)),
    };
    match serde_json::from_str::<ResourceList>(response_text.as_str()) {
        Ok(r) => match r.resources {
            Some(r) => resource_list = r,
            None => return Err("No resources found".to_string()),
        },
        Err(e) => return Err(format!("Error parsing response object: {:?}", e)),
    }

    // Get ICA URL for target resource
    let url_result = match resource_list
        .iter()
        .find(|r| r.name == Some(application_name.clone()))
    {
        Some(r) => match r.launchurl.clone() {
            Some(u) => u,
            None => return Err("No ICA URL found".to_string()),
        },
        None => return Err("Resource not found".to_string()),
    };

    // Get ICA file from StoreFront using full URL and validate
    // TODO: Add this url build to the url build function
    let file_name = "AutoLaunch.ica";
    // Create URL for ICA file download
    let url = match base_url.join(&format!("{}{}", &initial_url, &url_result)) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    // Add CsrfToken to URL
    let url = match url.join(&format!("?CsrfToken={}&IsUsingHttps=Yes", csrf_token)) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    // Download ICA file
    let file_response = match client.get(url).send() {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to download file: {:?}", e)),
    };
    // Get file data as bytes
    let file_response = match file_response.bytes() {
        Ok(f) => f,
        Err(e) => return Err(format!("Failed to get file bytes: {:?}", e)),
    };
    // Create string from bytes
    let file_response_string = match from_utf8(&file_response) {
        Ok(f) => f,
        Err(e) => return Err(format!("Failed to convert file bytes: {:?}", e)),
    };
    // Copy data to file if valid ICA file
    if file_response_string.contains("[WFClient]") {
        match save_ica(file_response_string) {
            Ok(_) => return Ok(file_name.to_string()),
            Err(e) => return Err(format!("Failed to save ICA file: {}", e)),
        }
    } else {
        Err("Invalid ICA file".to_string())
    }
}
