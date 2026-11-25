use hyper::{header, Body, Client, Method, Request};
use hyper_tls::HttpsConnector;
use serde::Serialize;
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use url::Url;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct PlaceImportPreview {
    pub name: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug)]
pub(super) enum ImportError {
    Invalid,
    NoDetails,
    Upstream,
    RateLimited,
}

#[derive(Clone, Default)]
pub(super) struct MapsImport {
    requests: Arc<Mutex<HashMap<Uuid, (Instant, u8)>>>,
}

enum Link {
    Short(String),
    Place(Url),
}

fn recognize(value: &str) -> Result<Link, ImportError> {
    if value.len() > 4096 {
        return Err(ImportError::Invalid);
    }
    let url = Url::parse(value.trim()).map_err(|_| ImportError::Invalid)?;
    if url.scheme() != "https"
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(ImportError::Invalid);
    }
    let path = url.path();
    match url.host_str() {
        Some("maps.app.goo.gl") => {
            let token = path.trim_start_matches('/');
            if token.is_empty()
                || token.len() > 128
                || !token.bytes().all(|byte| byte.is_ascii_alphanumeric())
            {
                return Err(ImportError::Invalid);
            }
            Ok(Link::Short(token.to_owned()))
        }
        Some("maps.google.com") if path == "/" || path.starts_with("/maps/place/") => {
            Ok(Link::Place(url))
        }
        Some("www.google.com" | "google.com") if path.starts_with("/maps/place/") => {
            Ok(Link::Place(url))
        }
        _ => Err(ImportError::Invalid),
    }
}

fn field(value: &str, max: usize) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && value.chars().count() <= max && !value.chars().any(char::is_control))
        .then(|| value.to_owned())
}

fn preview(url: &Url) -> Result<PlaceImportPreview, ImportError> {
    let query = url.query_pairs().collect::<Vec<_>>();
    // Only a place-qualified share link may supply a free-form name/address.
    let is_place_share = query
        .iter()
        .any(|(key, value)| key == "ftid" && !value.is_empty());
    if is_place_share {
        if let Some((_, text)) = query.iter().find(|(key, _)| key == "q") {
            // Recognize a street segment followed by a postal-code/city segment.
            // Everything before that boundary is the name, including internal commas.
            let parts: Vec<_> = text.split(',').map(str::trim).collect();
            if let Some(index) = (1..parts.len().saturating_sub(1)).find(|&index| {
                parts[index].chars().any(char::is_alphabetic)
                    && parts[index].chars().any(|c| c.is_ascii_digit())
                    && parts[index + 1]
                        .split_whitespace()
                        .next()
                        .is_some_and(|postal| {
                            postal.len() == 5 && postal.bytes().all(|c| c.is_ascii_digit())
                        })
                    && parts[index + 1].chars().any(char::is_alphabetic)
            }) {
                return nonempty(PlaceImportPreview {
                    name: field(&parts[..index].join(", "), 120),
                    address: field(&parts[index..].join(", "), 300),
                });
            }
            if parts.len() == 1 && text.chars().any(char::is_alphabetic) {
                return nonempty(PlaceImportPreview {
                    name: field(text, 120),
                    address: None,
                });
            }
        }
    }
    let segment = url
        .path()
        .strip_prefix("/maps/place/")
        .and_then(|path| path.split('/').next())
        .ok_or(ImportError::NoDetails)?;
    let decoded =
        url::form_urlencoded::parse(format!("name={}", segment.replace('&', "%26")).as_bytes())
            .next()
            .map(|(_, value)| value.into_owned())
            .unwrap_or_default();
    nonempty(PlaceImportPreview {
        name: field(&decoded, 120).filter(|name| name.chars().any(char::is_alphabetic)),
        address: None,
    })
}

fn nonempty(value: PlaceImportPreview) -> Result<PlaceImportPreview, ImportError> {
    if value.name.is_none() && value.address.is_none() {
        Err(ImportError::NoDetails)
    } else {
        Ok(value)
    }
}

impl MapsImport {
    pub(super) async fn resolve(
        &self,
        user_id: Uuid,
        input: &str,
    ) -> Result<PlaceImportPreview, ImportError> {
        let mut token = match recognize(input)? {
            Link::Place(url) => return preview(&url),
            Link::Short(token) => token,
        };
        self.reserve_request(user_id)?;
        tokio::time::timeout(Duration::from_secs(3), async {
            let mut connector = HttpsConnector::new();
            connector.https_only(true);
            let client = Client::builder().build::<_, Body>(connector);
            for _ in 0..3 {
                // Never send a request to a user-provided authority or forward query data.
                let request = Request::builder()
                    .method(Method::HEAD)
                    .uri(format!("https://maps.app.goo.gl/{token}"))
                    .body(Body::empty())
                    .map_err(|_| ImportError::Invalid)?;
                let response = client
                    .request(request)
                    .await
                    .map_err(|_| ImportError::Upstream)?;
                if ![301, 302, 303, 307, 308].contains(&response.status().as_u16()) {
                    return Err(ImportError::Upstream);
                }
                let location = response
                    .headers()
                    .get(header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or(ImportError::Upstream)?;
                match recognize(location).map_err(|_| ImportError::NoDetails)? {
                    Link::Place(url) => return preview(&url),
                    Link::Short(next) => token = next,
                }
            }
            Err(ImportError::Upstream)
        })
        .await
        .unwrap_or(Err(ImportError::Upstream))
    }

    fn reserve_request(&self, user_id: Uuid) -> Result<(), ImportError> {
        let mut requests = self.requests.lock().map_err(|_| ImportError::Upstream)?;
        requests.retain(|_, (start, _)| start.elapsed() < Duration::from_secs(60));
        if requests.len() >= 1024 && !requests.contains_key(&user_id) {
            return Err(ImportError::RateLimited);
        }
        match requests.entry(user_id) {
            Entry::Vacant(entry) => {
                entry.insert((Instant::now(), 1));
            }
            Entry::Occupied(mut entry) if entry.get().1 < 8 => entry.get_mut().1 += 1,
            Entry::Occupied(_) => return Err(ImportError::RateLimited),
        }
        Ok(())
    }
}
