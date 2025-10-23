use crate::error::{AppError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, instrument, warn};

#[derive(Deserialize)]
struct TranslationResponse {
    contents: TranslationContents,
}

#[derive(Deserialize)]
struct TranslationContents {
    translated: String,
}

#[derive(Serialize)]
struct TranslationRequest {
    text: String,
}

#[derive(Debug, Clone, Copy)]
enum Translator {
    Yoda,
    Shakespeare,
}

impl Translator {
    fn as_str(&self) -> &str {
        match self {
            Translator::Yoda => "yoda",
            Translator::Shakespeare => "shakespeare",
        }
    }
}

pub struct TranslationService {
    client: Client,
    base_url: String,
}

impl TranslationService {
    pub fn new(base_url: String, timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url }
    }

    #[instrument(skip(self, text), fields(translator, text_length = text.len()))]
    pub async fn translate(
        &self,
        text: &str,
        habitat: &Option<String>,
        is_legendary: bool,
    ) -> Result<String> {
        let translator =
            self.select_translator(habitat, is_legendary);
        tracing::Span::current()
            .record("translator", translator.as_str());

        let url =
            format!("{}/{}.json", self.base_url, translator.as_str());
        debug!("Translating with {} translator", translator.as_str());

        let response = self
            .client
            .post(&url)
            .json(&TranslationRequest {
                text: text.to_string(),
            })
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AppError::Timeout(format!(
                        "Translation request timed out: {}",
                        e
                    ))
                } else {
                    AppError::ExternalApi(format!(
                        "Translation request failed: {}",
                        e
                    ))
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            warn!("Translation API returned status: {}", status);
            return Err(AppError::ExternalApi(format!(
                "Translation API returned status: {}",
                status
            )));
        }

        let translation = response
            .json::<TranslationResponse>()
            .await
            .map_err(|e| {
                AppError::ExternalApi(format!(
                    "Failed to parse translation response: {}",
                    e
                ))
            })?;

        Ok(translation.contents.translated)
    }

    pub async fn health_check(&self) -> Result<()> {
        // Simple health check - just verify the base URL is reachable
        let url = format!("{}/shakespeare.json", self.base_url);
        self.client
            .post(&url)
            .json(&TranslationRequest {
                text: "test".to_string(),
            })
            .send()
            .await
            .map_err(|e| {
                AppError::ExternalApi(format!(
                    "Health check failed: {}",
                    e
                ))
            })?;
        Ok(())
    }

    fn select_translator(
        &self,
        habitat: &Option<String>,
        is_legendary: bool,
    ) -> Translator {
        if habitat.as_deref() == Some("cave") || is_legendary {
            Translator::Yoda
        } else {
            Translator::Shakespeare
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translator_selection_legendary() {
        let service = TranslationService::new(
            "http://example.com".to_string(),
            Duration::from_secs(10),
        );
        let translator = service
            .select_translator(&Some("forest".to_string()), true);
        assert_eq!(translator.as_str(), "yoda");
    }

    #[test]
    fn test_translator_selection_cave() {
        let service = TranslationService::new(
            "http://example.com".to_string(),
            Duration::from_secs(10),
        );
        let translator = service
            .select_translator(&Some("cave".to_string()), false);
        assert_eq!(translator.as_str(), "yoda");
    }

    #[test]
    fn test_translator_selection_shakespeare() {
        let service = TranslationService::new(
            "http://example.com".to_string(),
            Duration::from_secs(10),
        );
        let translator = service
            .select_translator(&Some("forest".to_string()), false);
        assert_eq!(translator.as_str(), "shakespeare");
    }

    #[test]
    fn test_translator_as_str() {
        assert_eq!(Translator::Yoda.as_str(), "yoda");
        assert_eq!(Translator::Shakespeare.as_str(), "shakespeare");
    }
}
