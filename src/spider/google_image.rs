use core::fmt;

use log::debug;
use once_cell::sync::Lazy;
use scraper::{Html, Selector};
use url::form_urlencoded;

use super::CLIENT;

/// Represents an image
#[derive(Debug)]
pub struct Image {
    pub src: String,
    pub alt: String,
}

impl fmt::Display for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.src, self.alt)
    }
}

/// Represents an image from google image search
#[derive(Debug)]
pub struct GoogleImage {
    pub thumb: Image,
    pub full: Image,
    pub title: String,
    pub url: String,
}

impl fmt::Display for GoogleImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} @ {} ({})", self.title, self.url, self.full)
    }
}

impl Image {
    /// Get the bytes of an image
    pub async fn get_bytes(image: &GoogleImage) -> Result<Vec<u8>, &'static str> {
        if let Ok(resp) = CLIENT.get(&image.full.src).send().await {
            if let Ok(bytes) = resp.bytes().await {
                return Ok(bytes.to_vec());
            }
        }
        Err("should have received bytes")
    }
}

impl GoogleImage {
    /// Get the bytes of an image
    pub async fn get_bytes(&self) -> Result<Vec<u8>, &'static str> {
        Image::get_bytes(self).await
    }
}

/**
`parse_google_image` accept a json format that is returned by
parsing json5 from a script element on google image search results page.
It parses the json and returns a `GoogleImage` struct.
 */
fn parse_google_image(x: &serde_json::Value) -> Option<GoogleImage> {
    let l: &serde_json::Value = &x[0][0][x[0][0].as_object()?.keys().next()?];

    let src: &Vec<serde_json::Value> = l[1]
        .as_array()?
        .iter()
        .find_map(|x| {
            if let Some(x) = x.as_object() {
                return Some(x);
            }
            None
        })?
        .values()
        .find(|x| {
            if let Some(x) = x.as_array() {
                return x.iter().any(|x| {
                    if let Some(s) = x.as_str() {
                        if s.starts_with("http") {
                            return true;
                        }
                    }
                    false
                });
            }
            false
        })?
        .as_array()?;

    let url = src[2].as_str()?.to_string();
    let title = src[3].as_str()?.to_string();

    let thumb = Image {
        src: l[1][2][0].as_str()?.to_string(),
        alt: title.clone(),
        // width: l[1][2][1].as_u64()? as u32,
        // height: l[1][2][2].as_u64()? as u32,
    };

    let full = Image {
        src: l[1][3][0].as_str()?.to_string(),
        alt: title.clone(),
        // width: l[1][3][1].as_u64()? as u32,
        // height: l[1][3][2].as_u64()? as u32,
    };

    Some(GoogleImage {
        thumb,
        full,
        title,
        url,
    })
}

/**
`image_search` searches for images on google and returns up to 100 images.
 */
pub async fn image_search(query: &str, offset: u32) -> Result<Vec<GoogleImage>, &'static str> {
    let params = form_urlencoded::Serializer::new(String::new())
        .append_pair("tbm", "isch")
        .append_pair("q", query)
        .append_pair("start", &offset.to_string())
        .append_pair("ijn", &(offset / 100).to_string())
        .finish();
    let url = format!("https://www.google.com/search?{}", params);
    debug!(target: "image_search", "url: {}", url);
    let dom = Html::parse_document(&CLIENT.get(&url).send().await.unwrap().text().await.unwrap());
    let script_selector = Lazy::new(|| Selector::parse("script").unwrap());
    let json = dom
        .select(&script_selector)
        .find_map(|x| {
            let text = x.text().collect::<String>();
            if text.contains("AF_initDataCallback")
                && !text.contains("ds:0")
                && text.contains("ds:1")
            {
                return Some(text);
            }
            None
        })
        .expect("should have a script element");
    let start_prefix = "AF_initDataCallback(";
    let start = json.find(start_prefix).unwrap();
    let end = json[start..].find("});").unwrap();
    let json: serde_json::Value =
        json5::from_str(&json[start + start_prefix.len()..end + 1]).expect("should be valid json");
    let data = &json["data"][56][1][0][0][1][0];
    let images = data
        .as_array()
        .unwrap()
        .iter()
        .filter_map(parse_google_image)
        .collect::<Vec<_>>();
    debug!(target: "image_search", "data parsed");
    Ok(images)
}

/**
`image_search_max` searches for images on google and returns up to `max` images.
 */
pub async fn image_search_max(query: &str, max: u32) -> Result<Vec<GoogleImage>, &'static str> {
    let mut images = Vec::new();
    let mut offset = 0;
    while offset < max {
        let mut new_images = image_search(query, offset).await?;
        if new_images.is_empty() {
            break;
        }
        offset += new_images.len() as u32;
        images.append(&mut new_images);
    }
    if images.len() > max as usize {
        images.truncate(max as usize);
    }
    Ok(images)
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_search() {
        let result = image_search("cat", 0).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.is_empty());
        dbg!(result);
    }
}
