pub mod google_image;
pub mod spanish_dict;

use log::info;
use once_cell::sync::Lazy;

/// The user agent used for all requests
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/113.0.0.0 Safari/537.36 Edg/113.0.1774.42";

/// The HTTP client used for all requests
pub static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    info!(target: "google_image", "creating client");
    reqwest::ClientBuilder::new()
        .user_agent(USER_AGENT)
        .cookie_store(true)
        .deflate(true)
        .brotli(true)
        .gzip(true)
        .build()
        .expect("should be able to create client")
});

/// Spider error
#[derive(Debug)]
pub struct SpiderError {
    message: String,
}

impl SpiderError {
    /// Create a new spider error
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl std::fmt::Display for SpiderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self.message)
    }
}

impl std::error::Error for SpiderError {}
