use reqwest::header::HeaderMap;

/// Get the value of an element from an HTML body
/// - Accepts the body as a string and the element name as a string
/// - Returns the value of the element as a String
pub fn get_element_value(body: &str, element: &str) -> Result<String, String> {
    let element_tag = format!("<{}", element.to_lowercase());
    let closing_tag = format!("</{}", element.to_lowercase());
    // find index of element in body
    let start = match body.to_lowercase().find(&element_tag) {
        Some(i) => i + element_tag.len(),
        None => return Err(format!("Element not found: {}", element).to_string()),
    };
    let start = match body[start..].find('>') {
        Some(i) => start + i + 1,
        None => return Err(format!("Element tag failed to terminate: {}", element).to_string()),
    };
    // find index of closing tag in body
    let end = match body[start..].to_lowercase().find(&closing_tag) {
        Some(i) => start + i,
        None => {
            return Err(format!("Element value \"{}\" failed to terminate", element).to_string())
        }
    };
    // return element value
    Ok(body[start..end].to_string())
}

/// Get the value of an attribute from an HTML or XML element
/// - Accepts a response body as String, the element name as a String, and the attribute name as a String
/// - Returns the value of the attribute as a String
/// - Example: get_attribute_value("<a href='https://example.com'>", "href") -> "https://example.com"
pub fn get_attribute_value(body: &str, element: &str, attribute: &str) -> Result<String, String> {
    let element_tag = format!("<{} ", element);
    // find index of element in body
    let start = match body.find(&element_tag) {
        Some(i) => i + element_tag.len(),
        None => return Err(format!("Element not found: {}", element).to_string()),
    };
    // find index of attribute in body
    let start = match body[start..].find(&attribute) {
        Some(i) => start + i + attribute.len(),
        None => return Err(format!("Attribute not found: {}", attribute).to_string()),
    };
    // find index of attribute value in body
    let start = match body[start..].find('"') {
        Some(i) => start + i + 1,
        None => return Err(format!("Attribute value not found: {}", attribute).to_string()),
    };
    // find index of attribute value end in body
    let end = match body[start..].find('"') {
        Some(i) => start + i,
        None => {
            return Err(format!("Attribute value failed to terminate: {}", attribute).to_string())
        }
    };
    // return attribute value
    Ok(body[start..end].to_string())
}

/// Get the value of a cookie from a set-cookie header
/// - Accepts the headers as a HeaderMap and the cookie name as a string
/// - Returns the value of the cookie as a String
pub fn get_cookie_value(headers: &HeaderMap, cookie_name: &str) -> Result<String, String> {
    headers
        .iter()
        .find_map(|(key, value)| {
            if key == "set-cookie" {
                let cookie = match value.to_str() {
                    Ok(c) => c,
                    Err(_) => return None,
                };
                if cookie.contains(cookie_name) {
                    // get value of cookie
                    let start = match cookie.find(cookie_name) {
                        Some(i) => i + cookie_name.len() + 1,
                        None => return None,
                    };
                    let end = match cookie[start..].find(';') {
                        Some(i) => i + start,
                        None => cookie.len(),
                    };
                    return Some(cookie[start..end].to_string());
                }
            }
            None
        })
        .ok_or_else(|| format!("Cookie not found: {}", cookie_name))
}

/// Get the value of one attribute of a particular header
/// - Accepts the headers as a HeaderMap, the header name as a string, and the attribute name as a string
/// - Returns the value of the attribute as a String
pub fn get_header_attribute(
    headers: &HeaderMap,
    header: &str,
    attribute: &str,
) -> Result<String, String> {
    headers
        .iter()
        .find_map(|(key, value)| {
            if key == header {
                let value = match value.to_str() {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                let start = match value.find(attribute) {
                    Some(i) => i + attribute.len() + 2,
                    None => return None,
                };
                let end = match value[start..].find('"') {
                    Some(i) => i + start,
                    None => value.len(),
                };
                return Some(value[start..end].to_string());
            }
            None
        })
        .ok_or_else(|| format!("Attribute not found: {}", attribute))
}
