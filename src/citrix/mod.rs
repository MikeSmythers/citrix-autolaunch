mod functions;
mod objects;
use crate::{
    extract::{get_attribute_value, get_cookie_value, get_element_value, get_header_attribute},
    storage::Settings,
};
use functions::common_headers;
pub use functions::ica_is_running;
use objects::{ProtoHeader, Resource, ResourceList};
use reqwest::{
    blocking::{self, Client},
    cookie::Jar,
    header::{HeaderName, HeaderValue, CONTENT_LENGTH, REFERER},
    Url,
};
use std::{
    fs::File,
    io::copy,
    str::{from_utf8, FromStr},
    sync::Arc,
};

// TODO: Add URL builder function to provide full URLs for below function

/// Get ICA file from Citrix StoreFront
/// - Uses Reqwest to interact with Citrix StoreFront
/// - Requires a Settings object with login and passwd fields
/// - Returns a Result with the file name on success, error message on failure
pub fn get_ica_file(settings: &Settings) -> Result<String, String> {
    let base_url =
        Url::parse(&settings.base_uri).map_err(|e| format!("Failed to parse base URI: {}", e))?;
    let jar = Arc::new(Jar::default());
    let client = Client::builder()
        .cookie_provider(Arc::clone(&jar))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    // [Initial] Get Initial URL from base URL (usually Logon/LogonPoint)
    let response = blocking::get(base_url.clone())
        .map_err(|e| format!("Failed to build URI [Initial]: {}", e))?;
    let initial_url = response
        .url()
        .join("./")
        .map_err(|e| format!("Failed to request [Initial]: {}", e))?;

    // [Home1] Call to (this is a default) Home/Configuration for Resource List path
    let uri = initial_url
        .join("Home/Configuration")
        .map_err(|e| format!("Failed to build URI [Home1]: {}", e))?;
    let response = client
        .post(uri)
        .headers(
            common_headers(
                Some(&vec![ProtoHeader(
                    CONTENT_LENGTH,
                    HeaderValue::from_static("0"),
                )]),
                &settings.base_uri,
            )
            .map_err(|e| format!("Failed to build headers [Home1]: {}", e))?,
        )
        .send()
        .map_err(|e| format!("Failed to request [Home1]: {}", e))?;
    let body = response
        .text()
        .map_err(|e| format!("Failed to retrieve configuration [Home1]: {}", e))?;
    let resource_list_path = get_attribute_value(&body, "resourcesProxy", "listURL")
        .map_err(|e| format!("Failed to retrieve path [Home1]: {}", e))?;

    // Call to Resource List for Auth Methods path
    let uri = match initial_url.join(&resource_list_path) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    let response = match client
        .post(uri)
        .headers(match common_headers(None, &settings.base_uri) {
            Ok(h) => h,
            Err(e) => return Err(e),
        })
        .header(CONTENT_LENGTH, "0")
        .send()
    {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to post resource list: {}", e)),
    };
    let auth_methods_path = match get_header_attribute(
        &response.headers(),
        "CitrixWebReceiver-Authenticate",
        "location",
    ) {
        Ok(a) => a,
        Err(e) => return Err(format!("Failed to get auth methods path: {}", e)),
    };

    // Call to Auth Methods Init for proper auth methods path
    let uri = match initial_url.join(&auth_methods_path) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    let response = match client
        .post(uri)
        .headers(match common_headers(None, &settings.base_uri) {
            Ok(h) => h,
            Err(e) => return Err(e),
        })
        .header(CONTENT_LENGTH, "0")
        .send()
    {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to post auth methods: {}", e)),
    };
    // TODO: [ISSUE 6] Make this less hacky...
    let body = match response.text() {
        Ok(b) => b,
        Err(e) => return Err(format!("Failed to retrieve auth methods: {}", e)),
    };
    let auth_methods_proper_path =
        match get_attribute_value(&body, "method name=\"ExplicitForms\"", "url") {
            Ok(a) => a,
            Err(e) => return Err(format!("Failed to get proper auth methods path: {}", e)),
        };

    // Pull auth methods for state_context
    let uri = match base_url.join(&auth_methods_proper_path) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    let response = match client
        .post(uri)
        .headers(match common_headers(None, &settings.base_uri) {
            Ok(h) => h,
            Err(e) => return Err(e),
        })
        .send()
    {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to post auth requirements (initial): {}", e)),
    };
    let input = match response.text() {
        Ok(i) => i,
        Err(e) => return Err(format!("Failed to retrieve state context: {}", e)),
    };
    let state_context = match get_element_value(&input, "StateContext") {
        Ok(s) => s.to_string(),
        Err(e) => return Err(format!("Failed to parse state context: {}", e)),
    };
    let auth_path = match get_element_value(&input, "Postback") {
        Ok(a) => a,
        Err(e) => return Err(format!("Failed to get auth path: {}", e)),
    };

    // Authenticate to StoreFront for AAAC cookie
    let credentials = &[
        ("login", settings.login.as_str()),
        ("passwd", settings.passwd.as_str()),
        ("savecredentials", "false"),
        ("nsg-x1-logon-button", "Log On"),
        ("StateContext", state_context.as_str()),
    ];
    let uri = match base_url.join(&auth_path) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    let response = match client
        .post(uri)
        .headers(match common_headers(None, &settings.base_uri) {
            Ok(h) => h,
            Err(e) => return Err(e),
        })
        .form(credentials)
        .send()
    {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to authenticate: {}", e)),
    };
    let input = match response.text() {
        Ok(i) => i,
        Err(e) => return Err(format!("Failed to retrieve auth response: {}", e)),
    };
    let set_client_path = match get_element_value(&input, "Postback") {
        Ok(p) => p,
        Err(_) => match get_element_value(&input, "RedirectURL") {
            Ok(r) => r,
            Err(e) => return Err(format!("Failed to get set client path: {}", e)),
        },
    };

    // Set client (useful for who knows what)
    // TODO: Figure out what this does
    let set_client_settings = &[
        ("nsg-setclient", "wica"),
        ("StateContext", state_context.as_str()),
    ];
    let uri = match base_url.join(&set_client_path) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    match client
        .post(uri)
        .headers(match common_headers(None, &settings.base_uri) {
            Ok(h) => h,
            Err(e) => return Err(e),
        })
        .form(set_client_settings)
        .send()
    {
        Ok(_) => (),
        Err(e) => return Err(format!("Failed to set client: {}", e)),
    };

    // Get base_rui redirect for internal path
    let response = match client.get(base_url.clone()).send() {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to get base URL: {}", e)),
    };
    let internal_path = response.url().path();
    let internal_url = match base_url.join(internal_path) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };

    // Get request to internal URL to set up Home Configuration
    let uri = internal_url.clone();
    match client
        .post(uri)
        .headers(match common_headers(None, &settings.base_uri) {
            Ok(h) => h,
            Err(e) => return Err(e),
        })
        .header(CONTENT_LENGTH, "0")
        .send()
    {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to get internal URL: {}", e)),
    };

    // Get config for csrf_token
    let uri = match internal_url.join("Home/Configuration") {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    let response = match client
        .post(uri)
        .headers(match common_headers(None, &settings.base_uri) {
            Ok(h) => h,
            Err(e) => return Err(e),
        })
        .header(CONTENT_LENGTH, "0")
        .send()
    {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to retrieve csrf token: {}", e)),
    };
    let csrf_token = match get_cookie_value(response.headers(), "CsrfToken") {
        Ok(c) => c.to_string(),
        Err(e) => return Err(format!("Failed to get csrf token: {}", e)),
    };
    let input = match response.text() {
        Ok(i) => i,
        Err(e) => return Err(format!("Failed to get csrf token: {}", e)),
    };
    let resource_list_path = match get_attribute_value(&input, "resourcesProxy", "listURL") {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to get resource list path: {}", e)),
    };

    // Modify request schema
    // Required for further StoreFront interaction
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
            match HeaderValue::from_str(&internal_url.to_string().as_str()) {
                Ok(h) => h,
                Err(e) => return Err(format!("Failed to create referer header: {}", e)),
            },
        ),
    ];
    let cookie_domain = match base_url.domain() {
        Some(d) => d,
        None => return Err("Failed to parse domain".to_string()),
    };
    jar.add_cookie_str(
        &format!(
            "CtxsClientDetectionDone=true; Domain={}; Path={}",
            cookie_domain, internal_path
        ),
        &internal_url,
    );
    jar.add_cookie_str(
        &format!(
            "CtxsHasUpgradeBeenShown=true; Domain={}; Path={}",
            cookie_domain, internal_path
        ),
        &internal_url,
    );
    jar.add_cookie_str(
        &format!(
            "CtxsUserPreferredClient=Native; Domain={}; Path={}",
            cookie_domain, internal_path
        ),
        &internal_url,
    );

    // Get list (will fail) for CtxsDeviceId cookie
    let get_list_settings = &[("format", "json"), ("resourceDetails", "Default")];
    let uri = match internal_url.join(&resource_list_path) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    let response = match client
        .post(uri)
        .headers(
            match common_headers(Some(&custom_headers), &settings.base_uri) {
                Ok(h) => h,
                Err(e) => return Err(e),
            },
        )
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
        Err(e) => return Err(format!("Failed to get auth methods path: {}", e)),
    };

    // Get auth methods (real) for CitrixAGBasic relative path
    let uri = match internal_url.join(&auth_methods_path) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    let response = match client
        .post(uri)
        .headers(
            match common_headers(Some(&custom_headers), &settings.base_uri) {
                Ok(h) => h,
                Err(e) => return Err(e),
            },
        )
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
    let uri = match internal_url.join(&auth_login_path) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    match client
        .post(uri)
        .headers(
            match common_headers(Some(&custom_headers), &settings.base_uri) {
                Ok(h) => h,
                Err(e) => return Err(e),
            },
        )
        .header(CONTENT_LENGTH, "0")
        .send()
    {
        Ok(_) => (),
        Err(e) => return Err(format!("Failed to log in: {}", e)),
    };

    // Get list (should work) to populate ResponseList object
    let get_list_settings = &[("format", "json"), ("resourceDetails", "Default")];
    let uri = match internal_url.join(&resource_list_path) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    let response = match client
        .post(uri)
        .headers(
            match common_headers(Some(&custom_headers), &settings.base_uri) {
                Ok(h) => h,
                Err(e) => return Err(e),
            },
        )
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
        Err(e) => return Err(format!("Error: {:?}", e)),
    }

    // Get ICA URL for target resource
    let url_result = match resource_list
        .iter()
        .find(|r| r.name == Some(settings.application_name.clone()))
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
    let url = match base_url.join(&format!("{}{}", &internal_url, &url_result)) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    let url = match url.join(&format!("?CsrfToken={}&IsUsingHttps=Yes", csrf_token)) {
        Ok(u) => u,
        Err(e) => return Err(format!("Failed to build URI: {}", e)),
    };
    let file_response = match client.get(url).send() {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to download file: {:?}", e)),
    };
    let mut file = match File::create(file_name) {
        Ok(f) => f,
        Err(e) => return Err(format!("Failed to create file: {:?}", e)),
    };
    let file_response = match file_response.bytes() {
        Ok(f) => f,
        Err(e) => return Err(format!("Failed to get file bytes: {:?}", e)),
    };
    let file_response_string = match from_utf8(&file_response) {
        Ok(f) => f,
        Err(e) => return Err(format!("Failed to convert file bytes: {:?}", e)),
    };
    if file_response_string.contains("[WFClient]") {
        match copy(&mut file_response.as_ref(), &mut file) {
            Ok(_) => Ok(file_name.to_string()),
            Err(e) => Err(format!("Failed to write file: {:?}", e)),
        }
    } else {
        Err("Invalid ICA file".to_string())
    }
}
