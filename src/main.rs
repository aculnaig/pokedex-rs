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
