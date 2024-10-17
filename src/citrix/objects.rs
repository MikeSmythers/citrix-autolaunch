use reqwest::header::{HeaderName, HeaderValue};
use serde::Deserialize;

/// Simplified header object for Reqwest
pub struct ProtoHeader(pub HeaderName, pub HeaderValue);

/// Response object from Citrix StoreFront
// Commented items are likely present but not used
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceList {
    // is_subscription_enabled: Option<bool>,
    // is_unauthenticated_user: Option<bool>,
    pub resources: Option<Vec<Resource>>, // List of resources from Citrix StoreFront
}

/// Resource object from Citrix StoreFront
// Commented items are likely present but not used
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    // clienttypes: Option<Vec<String>>,
    // description: Option<String>,
    // iconurl: Option<String>,
    // id: Option<String>,
    // launchstatusurl: Option<String>,
    // path: Option<String>,
    // shortcutvalidationurl: Option<String>,
    // subscriptionurl: Option<String>,
    pub launchurl: Option<String>, // Part of URL for ICA file download
    pub name: Option<String>,      // Name of resource as seen in Citrix StoreFront
}
