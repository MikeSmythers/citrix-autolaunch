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

    // [List1] Call to Resource List for Auth Methods path
    let uri = initial_url
        .join(&resource_list_path)
        .map_err(|e| format!("Failed to build URI [List1]: {}", e))?;
    let response = client
        .post(uri)
        .headers(
            common_headers(None, &settings.base_uri)
                .map_err(|e| format!("Failed to build headers [List1]: {}", e))?,
        )
        .header(CONTENT_LENGTH, "0")
        .send()
        .map_err(|e| format!("Failed to request [List1]: {}", e))?;
    let auth_methods_path = get_header_attribute(
        &response.headers(),
        "CitrixWebReceiver-Authenticate",
        "location",
    )
    .map_err(|e| format!("Failed to retrieve path [List1]: {}", e))?;

    // [AuthMethods1] Call to Auth Methods for proper auth methods path
    let uri = initial_url
        .join(&auth_methods_path)
        .map_err(|e| format!("Failed to build URI [AuthMethods1]: {}", e))?;
    let response = client
        .post(uri)
        .headers(
            common_headers(None, &settings.base_uri)
                .map_err(|e| format!("Failed to build headers [AuthMethods1]: {}", e))?,
        )
        .header(CONTENT_LENGTH, "0")
        .send()
        .map_err(|e| format!("Failed to request [AuthMethods1]: {}", e))?;
    // TODO: [ISSUE 6] Make this less hacky...
    let body = response
        .text()
        .map_err(|e| format!("Failed to retrieve body [AuthMethods1]: {}", e))?;
    let auth_methods_proper_path =
        get_attribute_value(&body, "method name=\"ExplicitForms\"", "url")
            .map_err(|e| format!("Failed to retrieve path [AuthMethods1]: {}", e))?;

    // [DoAuthMethods1] Pull auth methods for state_context
    let uri = base_url
        .join(&auth_methods_proper_path)
        .map_err(|e| format!("Failed to build URI [DoAuthMethods1]: {}", e))?;
    let response = client
        .post(uri)
        .headers(
            common_headers(None, &settings.base_uri)
                .map_err(|e| format!("Failed to build headers [DoAuthMethods1]: {}", e))?,
        )
        .send()
        .map_err(|e| format!("Failed to request [DoAuthMethods1]: {}", e))?;
    let input = response
        .text()
        .map_err(|e| format!("Failed to retrieve body [DoAuthMethods1]: {}", e))?;
    let state_context = get_element_value(&input, "StateContext")
        .map_err(|e| format!("Failed to retrieve state context [DoAuthMethods1]: {}", e))?;
    let auth_path = get_element_value(&input, "Postback")
        .map_err(|e| format!("Failed to retrieve auth path [DoAuthMethods1]: {}", e))?;

    // [DoAuth] Authenticate to StoreFront for AAAC cookie
    let credentials = &[
        ("login", settings.login.as_str()),
        ("passwd", settings.passwd.as_str()),
        ("savecredentials", "false"),
        ("nsg-x1-logon-button", "Log On"),
        ("StateContext", state_context.as_str()),
    ];
    let uri = base_url
        .join(&auth_path)
        .map_err(|e| format!("Failed to build URI [DoAuth]: {}", e))?;
    let response = client
        .post(uri)
        .headers(
            common_headers(None, &settings.base_uri)
                .map_err(|e| format!("Failed to build headers [DoAuth]: {}", e))?,
        )
        .form(credentials)
        .send()
        .map_err(|e| format!("Failed to request [DoAuth]: {}", e))?;
    let input = response
        .text()
        .map_err(|e| format!("Failed to retrieve auth response [DoAuth]: {}", e))?;
    let set_client_path = get_element_value(&input, "Postback")
        .or_else(|_| get_element_value(&input, "RedirectURL"))
        .map_err(|e| format!("Failed to retrieve set client path [DoAuth]: {}", e))?;

    // [SetClient] Set client (useful for who knows what)
    // TODO: Figure out what this does
    let set_client_settings = &[
        ("nsg-setclient", "wica"),
        ("StateContext", state_context.as_str()),
    ];
    let uri = base_url
        .join(&set_client_path)
        .map_err(|e| format!("Failed to build URI [SetClient]: {}", e))?;
    client
        .post(uri)
        .headers(
            common_headers(None, &settings.base_uri)
                .map_err(|e| format!("Failed to build headers [SetClient]: {}", e))?,
        )
        .form(set_client_settings)
        .send()
        .map_err(|e| format!("Failed to request [SetClient]: {}", e))?;

    // [Internal] Get base_rui redirect for internal path
    let response = client
        .get(base_url.clone())
        .send()
        .map_err(|e| format!("Failed to request [Internal]: {}", e))?;
    let internal_path = response.url().path();
    let internal_url = base_url
        .join(internal_path)
        .map_err(|e| format!("Failed to build URI [Internal]: {}", e))?;

    // [Home2] Get request to internal URL to set up Home Configuration
    let uri = internal_url.clone();
    client
        .post(uri)
        .headers(
            common_headers(None, &settings.base_uri)
                .map_err(|e| format!("Failed to build headers [Home2]: {}", e))?,
        )
        .header(CONTENT_LENGTH, "0")
        .send()
        .map_err(|e| format!("Failed to request [Home2]: {}", e))?;

    // [Config] Get config for csrf_token
    let uri = internal_url
        .join("Home/Configuration")
        .map_err(|e| format!("Failed to build URI [Config]: {}", e))?;
    let response = client
        .post(uri)
        .headers(
            common_headers(None, &settings.base_uri)
                .map_err(|e| format!("Failed to build headers [Config]: {}", e))?,
        )
        .header(CONTENT_LENGTH, "0")
        .send()
        .map_err(|e| format!("Failed to request [Config]: {}", e))?;
    let csrf_token = get_cookie_value(response.headers(), "CsrfToken")
        .map_err(|e| format!("Failed to retrieve csrf token [Config]: {}", e))?;
    let input = response
        .text()
        .map_err(|e| format!("Failed to parse csrf token [Config]: {}", e))?;
    let resource_list_path = get_attribute_value(&input, "resourcesProxy", "listURL")
        .map_err(|e| format!("Failed to retrieve path [Config]: {}", e))?;

    // [Schema] Modify request schema
    // Required for further StoreFront interaction
    let csrf_token_header = HeaderName::from_str("Csrf-Token")
        .map_err(|e| format!("Failed to create csrf token header [Schema]: {}", e))?;
    let custom_headers: Vec<ProtoHeader> = vec![
        ProtoHeader(
            csrf_token_header,
            HeaderValue::from_str(&csrf_token).unwrap(),
        ),
        ProtoHeader(
            REFERER,
            HeaderValue::from_str(&internal_url.to_string().as_str())
                .map_err(|e| format!("Failed to create referer header [Schema]: {}", e))?,
        ),
    ];
    let cookie_domain = base_url
        .domain()
        .ok_or("Base URL returned no domain".to_string())
        .map_err(|e| format!("Failed to parse cookie domain [Schema]: {}", e))?;
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

    // [ListDoomed] Get list (will fail) for CtxsDeviceId cookie
    let get_list_settings = &[("format", "json"), ("resourceDetails", "Default")];
    let uri = internal_url
        .join(&resource_list_path)
        .map_err(|e| format!("Failed to build URI [ListDoomed]: {}", e))?;
    let response = client
        .post(uri)
        .headers(
            common_headers(Some(&custom_headers), &settings.base_uri)
                .map_err(|e| format!("Failed to build headers [ListDoomed]: {}", e))?,
        )
        .form(get_list_settings)
        .send()
        .map_err(|e| format!("Failed to request [ListDoomed]: {}", e))?;
    let auth_methods_path = get_header_attribute(
        &response.headers(),
        "CitrixWebReceiver-Authenticate",
        "location",
    )
    .map_err(|e| format!("Failed to retrieve path [ListDoomed]: {}", e))?;

    // [AuthMethods2] Get auth methods (real) for CitrixAGBasic relative path
    let uri = internal_url
        .join(&auth_methods_path)
        .map_err(|e| format!("Failed to build URI [AuthMethods2]: {}", e))?;
    let response = client
        .post(uri)
        .headers(
            common_headers(Some(&custom_headers), &settings.base_uri)
                .map_err(|e| format!("Failed to build headers [AuthMethods2]: {}", e))?,
        )
        .header(CONTENT_LENGTH, "0")
        .send()
        .map_err(|e| format!("Failed to request [AuthMethods2]: {}", e))?;
    let input = response
        .text()
        .map_err(|e| format!("Failed to retrieve auth methods [AuthMethods2]: {}", e))?;
    // TODO: [ISSUE 6] Make this less hacky...
    let auth_login_path = get_attribute_value(&input, "method name=\"CitrixAGBasic\"", "url")
        .map_err(|e| format!("Failed to retrieve path [AuthMethods2]: {}", e))?;

    // [Login] Log in to get CtxsAuthId cookie
    let uri = internal_url
        .join(&auth_login_path)
        .map_err(|e| format!("Failed to build URI [Login]: {}", e))?;
    client
        .post(uri)
        .headers(
            common_headers(Some(&custom_headers), &settings.base_uri)
                .map_err(|e| format!("Failed to build headers [Login]: {}", e))?,
        )
        .header(CONTENT_LENGTH, "0")
        .send()
        .map_err(|e| format!("Failed to request [Login]: {}", e))?;

    // [List2] Get list (should work) to populate ResponseList object
    let get_list_settings = &[("format", "json"), ("resourceDetails", "Default")];
    let uri = internal_url
        .join(&resource_list_path)
        .map_err(|e| format!("Failed to build URI [List2]: {}", e))?;
    let response = client
        .post(uri)
        .headers(
            common_headers(Some(&custom_headers), &settings.base_uri)
                .map_err(|e| format!("Failed to build headers [List2]: {}", e))?,
        )
        .form(get_list_settings)
        .send()
        .map_err(|e| format!("Failed to request [List2]: {}", e))?;

    // [ParseList] Parse response into ResourceList object
    let mut resource_list: Vec<Resource> = vec![];
    let response_text = response
        .text()
        .map_err(|e| format!("Failed to get response text: {}", e))?;
    serde_json::from_str::<ResourceList>(response_text.as_str())
        .map_err(|e| format!("Failed to parse response text [ParseList]: {}", e))
        .and_then(|r| r.resources.ok_or("No resources found".to_string()))
        .map(|r| resource_list = r)?;

    // Get ICA URL for target resource
    let url_result = resource_list
        .iter()
        .find(|r| r.name == Some(settings.application_name.clone()))
        .and_then(|r| r.launchurl.clone())
        .ok_or("No ICA URL found".to_string())
        .map_err(|e| format!("Failed to get ICA URL: {}", e))?;

    // [ICA] Get ICA file from StoreFront using full URL and validate
    // TODO: Add this url build to the url build function
    let file_name = "AutoLaunch.ica";
    let url = base_url
        .join(&format!("{}{}", &internal_url, &url_result))
        .map_err(|e| format!("Failed to build base URI [ICA]: {}", e))?;
    let url = url
        .join(&format!("?CsrfToken={}&IsUsingHttps=Yes", csrf_token))
        .map_err(|e| format!("Failed to build full URI [ICA]: {}", e))?;
    let file_response = client
        .get(url)
        .send()
        .map_err(|e| format!("Failed to request [ICA]: {}", e))?;
    let mut file =
        File::create(file_name).map_err(|e| format!("Failed to create file: {:?}", e))?;
    let file_response = file_response
        .bytes()
        .map_err(|e| format!("Failed to get file bytes: {:?}", e))?;
    let file_response_string =
        from_utf8(&file_response).map_err(|e| format!("Failed to convert file bytes: {:?}", e))?;
    if file_response_string.contains("[WFClient]") {
        copy(&mut file_response.as_ref(), &mut file)
            .and_then(|_| Ok(file_name.to_string()))
            .map_err(|e| format!("Failed to write file: {:?}", e))
    } else {
        Err("Invalid ICA file".to_string())
    }
}
