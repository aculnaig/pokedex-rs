use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub pokeapi_base_url: String,
    pub translation_api_base_url: String,
    pub http_timeout: Duration,
    pub request_timeout: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "5000".to_string())
                .parse()
                .expect("PORT must be a valid u16"),
            pokeapi_base_url: std::env::var("POKEAPI_BASE_URL")
                .unwrap_or_else(|_| {
                    "https://pokeapi.co/api/v2".to_string()
                }),
            translation_api_base_url: std::env::var(
                "TRANSLATION_API_BASE_URL",
            )
            .unwrap_or_else(|_| {
                "https://api.funtranslations.com/translate"
                    .to_string()
            }),
            http_timeout: Duration::from_secs(
                std::env::var("HTTP_TIMEOUT_SECS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .expect("HTTP_TIMEOUT_SECS must be a valid u64"),
            ),
            request_timeout: std::env::var("REQUEST_TIMEOUT_SECS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .expect("REQUEST_TIMEOUT_SECS must be a valid u64"),
        }
    }
}
