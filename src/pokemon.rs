use crate::error::{AppError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Serialize, Deserialize)]
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
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://pokeapi.co/api/v2".to_string(),
        }
    }

    pub async fn get_pokemon(&self, name: &str) -> Result<Pokemon> {
        let url = format!("{}/pokemon-species/{}", self.base_url, name.to_lowercase());
        debug!("Fetching pokemon from: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::ExternalApi(format!("Failed to fetch pokemon: {}", e)))?;

        if !response.status().is_success() {
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(AppError::NotFound(format!("Pokemon '{}' not found", name)));
            }
            return Err(AppError::ExternalApi(format!(
                "PokeAPI returned status: {}",
                response.status()
            )));
        }

        let species = response
            .json::<PokeApiSpecies>()
            .await
            .map_err(|e| AppError::ExternalApi(format!("Failed to parse pokemon data: {}", e)))?;

        Ok(self.map_to_pokemon(species))
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
}
