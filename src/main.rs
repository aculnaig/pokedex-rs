use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

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
    axum::serve(listener, app).await.unwrap();
}

// Handler that returns the pokemon name and basic information
async fn pokemon_name_handler(
    Path(name): Path<String>,
    State(client): State<Client>,
) -> (StatusCode, Json<Pokemon>) {
    let url =
        format!("https://pokeapi.co/api/v2/pokemon-species/{name}");
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

    let description = species
        .flavor_text_entries
        .iter()
        .find(|entry| entry.language.name == "en")
        .map(|entry| entry.flavor_text.clone());

    let habitat = species.habitat.map(|h| h.name);

    let pokemon = Pokemon {
        name: species.name,
        description,
        habitat,
        is_legendary: species.is_legendary,
    };

    (StatusCode::OK, Json(pokemon))
}

// Handler that returns the pokemon translated name and basic information
async fn pokemon_translated_name_handler(
    Path(name): Path<String>,
    State(client): State<Client>,
) -> (StatusCode, Json<Pokemon>) {
    todo!()
}

// The Pokemon input response
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
