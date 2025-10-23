use crate::error::{AppError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

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

pub struct TranslationService {
    client: Client,
    base_url: String,
}

impl TranslationService {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://api.funtranslations.com/translate".to_string(),
        }
    }

    pub async fn translate(
        &self,
        text: &str,
        habitat: &Option<String>,
        is_legendary: bool,
    ) -> Result<String> {
        let translator = self.select_translator(habitat, is_legendary);
        let url = format!("{}/{}.json", self.base_url, translator);

        debug!("Translating with {} translator", translator);

        let response = self
            .client
            .post(&url)
            .json(&TranslationRequest {
                text: text.to_string(),
            })
            .send()
            .await
            .map_err(|e| AppError::ExternalApi(format!("Translation request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::ExternalApi(format!(
                "Translation API returned status: {}",
                response.status()
            )));
        }

        let translation = response
            .json::<TranslationResponse>()
            .await
            .map_err(|e| {
                AppError::ExternalApi(format!("Failed to parse translation response: {}", e))
            })?;

        Ok(translation.contents.translated)
    }

    fn select_translator(&self, habitat: &Option<String>, is_legendary: bool) -> &str {
        if habitat.as_deref() == Some("cave") || is_legendary {
            "yoda"
        } else {
            "shakespeare"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translator_selection_legendary() {
        let service = TranslationService::new();
        assert_eq!(
            service.select_translator(&Some("forest".to_string()), true),
            "yoda"
        );
    }

    #[test]
    fn test_translator_selection_cave() {
        let service = TranslationService::new();
        assert_eq!(
            service.select_translator(&Some("cave".to_string()), false),
            "yoda"
        );
    }

    #[test]
    fn test_translator_selection_shakespeare() {
        let service = TranslationService::new();
        assert_eq!(
            service.select_translator(&Some("forest".to_string()), false),
            "shakespeare"
        );
    }

    #[test]
    fn test_translator_selection_no_habitat() {
        let service = TranslationService::new();
        assert_eq!(service.select_translator(&None, false), "shakespeare");
    }
}
