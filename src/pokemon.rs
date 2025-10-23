use crate::error::{AppError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, instrument};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pokemon {
    pub name: String,
    pub description: Option<String>,
    pub habitat: Option<String>,
    pub is_legendary: bool,
}

#[derive(Deserialize)]
struct PokeApiSpecies {
    name: String,
    habitat: Option<Habitat>,
    flavor_text_entries: Vec<FlavorTextEntry>,
    is_legendary: bool,
}

#[derive(Deserialize)]
struct FlavorTextEntry {
    flavor_text: String,
    language: Language,
}

#[derive(Deserialize)]
struct Language {
    name: String,
}

#[derive(Deserialize)]
struct Habitat {
    name: String,
}

pub struct PokemonService {
    client: Client,
    base_url: String,
}

impl PokemonService {
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

    #[instrument(skip(self), fields(pokemon_name = %name))]
    pub async fn get_pokemon(&self, name: &str) -> Result<Pokemon> {
        let url = format!(
            "{}/pokemon-species/{}",
            self.base_url,
            name.to_lowercase()
        );
        debug!("Fetching pokemon from: {}", url);

        let response =
            self.client.get(&url).send().await.map_err(|e| {
                if e.is_timeout() {
                    AppError::Timeout(format!(
                        "Request to PokeAPI timed out: {}",
                        e
                    ))
                } else if e.is_connect() {
                    AppError::ExternalApi(format!(
                        "Failed to connect to PokeAPI: {}",
                        e
                    ))
                } else {
                    AppError::ExternalApi(format!(
                        "Failed to fetch pokemon: {}",
                        e
                    ))
                }
            })?;

        if !response.status().is_success() {
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(AppError::NotFound(format!(
                    "Pokemon '{}' not found",
                    name
                )));
            }
            return Err(AppError::ExternalApi(format!(
                "PokeAPI returned status: {}",
                response.status()
            )));
        }

        let species =
            response.json::<PokeApiSpecies>().await.map_err(|e| {
                AppError::ExternalApi(format!(
                    "Failed to parse pokemon data: {}",
                    e
                ))
            })?;

        Ok(self.map_to_pokemon(species))
    }

    pub async fn health_check(&self) -> Result<()> {
        let url = format!("{}/pokemon-species/1", self.base_url);
        self.client.get(&url).send().await.map_err(|e| {
            AppError::ExternalApi(format!(
                "Health check failed: {}",
                e
            ))
        })?;
        Ok(())
    }

    fn map_to_pokemon(&self, species: PokeApiSpecies) -> Pokemon {
        let description = species
            .flavor_text_entries
            .iter()
            .find(|entry| entry.language.name == "en")
            .map(|entry| clean_description(&entry.flavor_text));

        Pokemon {
            name: species.name,
            description,
            habitat: species.habitat.map(|h| h.name),
            is_legendary: species.is_legendary,
        }
    }
}

fn clean_description(text: &str) -> String {
    text.replace('\n', " ")
        .replace('\r', " ")
        .replace('\u{000C}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_description() {
        let input = "Line one\nLine two\u{000C}Line three";
        let expected = "Line one Line two Line three";
        assert_eq!(clean_description(input), expected);
    }

    #[test]
    fn test_clean_description_multiple_spaces() {
        let input = "Word1   Word2     Word3";
        let expected = "Word1 Word2 Word3";
        assert_eq!(clean_description(input), expected);
    }

    #[test]
    fn test_pokemon_equality() {
        let p1 = Pokemon {
            name: "pikachu".to_string(),
            description: Some("Electric mouse".to_string()),
            habitat: Some("forest".to_string()),
            is_legendary: false,
        };
        let p2 = p1.clone();
        assert_eq!(p1, p2);
    }
}
