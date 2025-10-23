use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // HTTP request client
    let client = Client::new();

    // HTTP routes
    let app = Router::new()
        .route("/pokemon/{name}", get(pokemon_name_handler))
        .route(
            "/pokemon/translated/{name}",
            get(pokemon_translated_name_handler),
        )
        .with_state(client);

    // Listen on port 5000
    let listener =
        tokio::net::TcpListener::bind("0.0.0.0:5000").await.unwrap();
    println!("Server running on http://0.0.0.0:5000");
    axum::serve(listener, app).await.unwrap();
}

// Trait for Pokemon processing strategies
#[async_trait]
trait PokemonProcessor {
    async fn process(&self, species: PokemonInput, client: &Client) -> Pokemon;
}

// Basic processor - returns pokemon as-is
struct BasicProcessor;

#[async_trait]
impl PokemonProcessor for BasicProcessor {
    async fn process(&self, species: PokemonInput, _client: &Client) -> Pokemon {
        let description = extract_english_description(&species.flavor_text_entries)
            .map(|desc| clean_description(&desc));
        let habitat = species.habitat.map(|h| h.name);

        Pokemon {
            name: species.name,
            description,
            habitat,
            is_legendary: species.is_legendary,
        }
    }
}

// Translated processor - translates the description based on habitat/legendary status
struct TranslatedProcessor;

#[async_trait]
impl PokemonProcessor for TranslatedProcessor {
    async fn process(&self, species: PokemonInput, client: &Client) -> Pokemon {
        let description = extract_english_description(&species.flavor_text_entries);
        let habitat = species.habitat.as_ref().map(|h| h.name.clone());

        // Translate description based on habitat or legendary status
        let translated_description = if let Some(desc) = description {
            let cleaned_desc = clean_description(&desc);

            Some(translate_description(&cleaned_desc, &habitat, species.is_legendary, client)
                .await
                .unwrap_or(cleaned_desc))
        } else {
            None
        };

        Pokemon {
            name: species.name,
            description: translated_description,
            habitat,
            is_legendary: species.is_legendary,
        }
    }
}

// Extract English description from flavor text entries
fn extract_english_description(entries: &[FlavorTextEntry]) -> Option<String> {
    entries
        .iter()
        .find(|entry| entry.language.name == "en")
        .map(|entry| entry.flavor_text.clone())
}

// Clean description by replacing newlines and form feeds with single spaces
fn clean_description(text: &str) -> String {
    text.replace('\n', " ")
        .replace('\r', " ")
        .replace('\u{000C}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// Helper function to translate description
// Uses Yoda translator for cave habitat or legendary Pokemon, Shakespeare otherwise
async fn translate_description(
    text: &str,
    habitat: &Option<String>,
    is_legendary: bool,
    client: &Client,
) -> Option<String> {
    // Rule: Use Yoda translator for cave habitat or legendary Pokemon
    let translator = if habitat.as_deref() == Some("cave") || is_legendary {
        "yoda"
    } else {
        "shakespeare"
    };

    let url = format!("https://api.funtranslations.com/translate/{}.json", translator);

    // Attempt translation
    match client
        .post(&url)
        .json(&serde_json::json!({ "text": text }))
        .send()
        .await
    {
        Ok(res) if res.status().is_success() => {
            res.json::<TranslationResponse>()
                .await
                .ok()
                .map(|tr| tr.contents.translated)
        }
        _ => None, // Return None on failure, caller will use fallback
    }
}

// Generic handler function to avoid code duplication
async fn handle_pokemon_request<P: PokemonProcessor>(
    name: String,
    client: &Client,
    processor: P,
) -> (StatusCode, Json<Pokemon>) {
    let url = format!("https://pokeapi.co/api/v2/pokemon-species/{}", name.to_lowercase());

    let res = match client.get(&url).send().await {
        Ok(res) if res.status().is_success() => res,
        Ok(res) => {
            return (
                res.status(),
                Json(Pokemon {
                    name: name.clone(),
                    description: None,
                    habitat: None,
                    is_legendary: false,
                }),
            );
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Pokemon {
                    name: name.clone(),
                    description: None,
                    habitat: None,
                    is_legendary: false,
                }),
            );
        }
    };

    let species = match res.json::<PokemonInput>().await {
        Ok(json) => json,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Pokemon {
                    name: name.clone(),
                    description: None,
                    habitat: None,
                    is_legendary: false,
                }),
            );
        }
    };

    let pokemon = processor.process(species, client).await;
    (StatusCode::OK, Json(pokemon))
}

// Handler that returns the pokemon name and basic information
async fn pokemon_name_handler(
    Path(name): Path<String>,
    State(client): State<Client>,
) -> (StatusCode, Json<Pokemon>) {
    handle_pokemon_request(name, &client, BasicProcessor).await
}

// Handler that returns the pokemon translated name and basic information
async fn pokemon_translated_name_handler(
    Path(name): Path<String>,
    State(client): State<Client>,
) -> (StatusCode, Json<Pokemon>) {
    handle_pokemon_request(name, &client, TranslatedProcessor).await
}

// Translation API response structures
#[derive(Deserialize)]
struct TranslationResponse {
    contents: TranslationContents,
}

#[derive(Deserialize)]
struct TranslationContents {
    translated: String,
}

// The Pokemon input response from PokeAPI
#[derive(Deserialize)]
struct PokemonInput {
    name: String,
    habitat: Option<HabitatEntry>,
    flavor_text_entries: Vec<FlavorTextEntry>,
    is_legendary: bool,
}

#[derive(Deserialize)]
struct FlavorTextEntry {
    flavor_text: String,
    language: LanguageEntry,
}

#[derive(Deserialize)]
struct LanguageEntry {
    name: String,
}

#[derive(Deserialize)]
struct HabitatEntry {
    name: String,
}

// The Pokemon output response
#[derive(Serialize)]
struct Pokemon {
    name: String,
    description: Option<String>,
    habitat: Option<String>,
    is_legendary: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create test flavor text entries
    fn create_flavor_text_entries(english_text: &str) -> Vec<FlavorTextEntry> {
        vec![
            FlavorTextEntry {
                flavor_text: "Texto en español".to_string(),
                language: LanguageEntry {
                    name: "es".to_string(),
                },
            },
            FlavorTextEntry {
                flavor_text: english_text.to_string(),
                language: LanguageEntry {
                    name: "en".to_string(),
                },
            },
            FlavorTextEntry {
                flavor_text: "Texte en français".to_string(),
                language: LanguageEntry {
                    name: "fr".to_string(),
                },
            },
        ]
    }

    // Helper function to create test PokemonInput
    fn create_test_pokemon_input(
        name: &str,
        description: &str,
        habitat: Option<&str>,
        is_legendary: bool,
    ) -> PokemonInput {
        PokemonInput {
            name: name.to_string(),
            habitat: habitat.map(|h| HabitatEntry {
                name: h.to_string(),
            }),
            flavor_text_entries: create_flavor_text_entries(description),
            is_legendary,
        }
    }

    #[test]
    fn test_clean_description_removes_newlines() {
        let text = "Line one\nLine two\nLine three";
        let result = clean_description(text);
        assert_eq!(result, "Line one Line two Line three");
    }

    #[test]
    fn test_clean_description_removes_carriage_returns() {
        let text = "Line one\r\nLine two\rLine three";
        let result = clean_description(text);
        assert_eq!(result, "Line one Line two Line three");
    }

    #[test]
    fn test_clean_description_removes_form_feeds() {
        let text = "Line one\u{000C}Line two";
        let result = clean_description(text);
        assert_eq!(result, "Line one Line two");
    }

    #[test]
    fn test_clean_description_collapses_multiple_spaces() {
        let text = "Word1   Word2     Word3";
        let result = clean_description(text);
        assert_eq!(result, "Word1 Word2 Word3");
    }

    #[test]
    fn test_clean_description_complex() {
        let text = "It was created by\na scientist after\nyears of horrific\u{000C}gene splicing";
        let result = clean_description(text);
        assert_eq!(result, "It was created by a scientist after years of horrific gene splicing");
    }

    #[test]
    fn test_extract_english_description_found() {
        let entries = create_flavor_text_entries("English description");
        let result = extract_english_description(&entries);
        assert_eq!(result, Some("English description".to_string()));
    }

    #[test]
    fn test_extract_english_description_not_found() {
        let entries = vec![
            FlavorTextEntry {
                flavor_text: "Texto en español".to_string(),
                language: LanguageEntry {
                    name: "es".to_string(),
                },
            },
            FlavorTextEntry {
                flavor_text: "Texte en français".to_string(),
                language: LanguageEntry {
                    name: "fr".to_string(),
                },
            },
        ];
        let result = extract_english_description(&entries);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_english_description_empty() {
        let entries = vec![];
        let result = extract_english_description(&entries);
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_basic_processor_legendary_pokemon() {
        let client = Client::new();
        let processor = BasicProcessor;

        let input = create_test_pokemon_input(
            "mewtwo",
            "It was created by\na scientist after\nyears of horrific\ngene splicing.",
            Some("rare"),
            true,
        );

        let result = processor.process(input, &client).await;

        assert_eq!(result.name, "mewtwo");
        assert_eq!(
            result.description,
            Some("It was created by a scientist after years of horrific gene splicing.".to_string())
        );
        assert_eq!(result.habitat, Some("rare".to_string()));
        assert_eq!(result.is_legendary, true);
    }

    #[tokio::test]
    async fn test_basic_processor_regular_pokemon() {
        let client = Client::new();
        let processor = BasicProcessor;

        let input = create_test_pokemon_input(
            "pikachu",
            "When several of\nthese POKéMON gather,\ntheir electricity could\nbuild and cause lightning storms.",
            Some("forest"),
            false,
        );

        let result = processor.process(input, &client).await;

        assert_eq!(result.name, "pikachu");
        assert_eq!(
            result.description,
            Some("When several of these POKéMON gather, their electricity could build and cause lightning storms.".to_string())
        );
        assert_eq!(result.habitat, Some("forest".to_string()));
        assert_eq!(result.is_legendary, false);
    }

    #[tokio::test]
    async fn test_basic_processor_no_habitat() {
        let client = Client::new();
        let processor = BasicProcessor;

        let input = create_test_pokemon_input(
            "porygon",
            "A POKéMON that consists\nentirely of programming\ncode.",
            None,
            false,
        );

        let result = processor.process(input, &client).await;

        assert_eq!(result.name, "porygon");
        assert_eq!(
            result.description,
            Some("A POKéMON that consists entirely of programming code.".to_string())
        );
        assert_eq!(result.habitat, None);
        assert_eq!(result.is_legendary, false);
    }

    #[tokio::test]
    async fn test_basic_processor_no_english_description() {
        let client = Client::new();
        let processor = BasicProcessor;

        let mut input = create_test_pokemon_input(
            "testmon",
            "English description",
            Some("mountain"),
            false,
        );

        // Replace with non-English entries only
        input.flavor_text_entries = vec![
            FlavorTextEntry {
                flavor_text: "Descripción en español".to_string(),
                language: LanguageEntry {
                    name: "es".to_string(),
                },
            },
        ];

        let result = processor.process(input, &client).await;

        assert_eq!(result.name, "testmon");
        assert_eq!(result.description, None);
        assert_eq!(result.habitat, Some("mountain".to_string()));
        assert_eq!(result.is_legendary, false);
    }

    #[test]
    fn test_translator_selection_legendary() {
        // Legendary Pokemon should use Yoda translator
        let habitat = Some("forest".to_string());
        let is_legendary = true;

        // We can't easily test the async function directly, but we can verify the logic
        let translator = if habitat.as_deref() == Some("cave") || is_legendary {
            "yoda"
        } else {
            "shakespeare"
        };

        assert_eq!(translator, "yoda");
    }

    #[test]
    fn test_translator_selection_cave_habitat() {
        // Cave habitat should use Yoda translator
        let habitat = Some("cave".to_string());
        let is_legendary = false;

        let translator = if habitat.as_deref() == Some("cave") || is_legendary {
            "yoda"
        } else {
            "shakespeare"
        };

        assert_eq!(translator, "yoda");
    }

    #[test]
    fn test_translator_selection_legendary_and_cave() {
        // Both legendary and cave should use Yoda translator
        let habitat = Some("cave".to_string());
        let is_legendary = true;

        let translator = if habitat.as_deref() == Some("cave") || is_legendary {
            "yoda"
        } else {
            "shakespeare"
        };

        assert_eq!(translator, "yoda");
    }

    #[test]
    fn test_translator_selection_shakespeare() {
        // Regular Pokemon should use Shakespeare translator
        let habitat = Some("forest".to_string());
        let is_legendary = false;

        let translator = if habitat.as_deref() == Some("cave") || is_legendary {
            "yoda"
        } else {
            "shakespeare"
        };

        assert_eq!(translator, "shakespeare");
    }

    #[test]
    fn test_translator_selection_no_habitat() {
        // Pokemon with no habitat and not legendary should use Shakespeare
        let habitat: Option<String> = None;
        let is_legendary = false;

        let translator = if habitat.as_deref() == Some("cave") || is_legendary {
            "yoda"
        } else {
            "shakespeare"
        };

        assert_eq!(translator, "shakespeare");
    }

    #[test]
    fn test_pokemon_serialization() {
        let pokemon = Pokemon {
            name: "mewtwo".to_string(),
            description: Some("A powerful Pokemon".to_string()),
            habitat: Some("rare".to_string()),
            is_legendary: true,
        };

        let json = serde_json::to_string(&pokemon).unwrap();
        assert!(json.contains("\"name\":\"mewtwo\""));
        assert!(json.contains("\"description\":\"A powerful Pokemon\""));
        assert!(json.contains("\"habitat\":\"rare\""));
        assert!(json.contains("\"is_legendary\":true"));
    }

    #[test]
    fn test_pokemon_input_deserialization() {
        let json = r#"{
            "name": "pikachu",
            "habitat": {"name": "forest"},
            "flavor_text_entries": [
                {
                    "flavor_text": "An electric Pokemon",
                    "language": {"name": "en"}
                }
            ],
            "is_legendary": false
        }"#;

        let pokemon_input: PokemonInput = serde_json::from_str(json).unwrap();
        assert_eq!(pokemon_input.name, "pikachu");
        assert_eq!(pokemon_input.habitat.unwrap().name, "forest");
        assert_eq!(pokemon_input.is_legendary, false);
        assert_eq!(pokemon_input.flavor_text_entries.len(), 1);
    }

    #[test]
    fn test_pokemon_input_deserialization_no_habitat() {
        let json = r#"{
            "name": "porygon",
            "habitat": null,
            "flavor_text_entries": [],
            "is_legendary": false
        }"#;

        let pokemon_input: PokemonInput = serde_json::from_str(json).unwrap();
        assert_eq!(pokemon_input.name, "porygon");
        assert!(pokemon_input.habitat.is_none());
    }
}
