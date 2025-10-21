use axum::{extract::Path, http::StatusCode, routing::get, Json, Router};
use serde::Serialize;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // HTTP routes
    let app = Router::new()
        .route("/pokemon/{name}", get(pokemon_name_handler))
        .route("/pokemon/translated/{name}", get(pokemon_translated_name_handler));

    // Listen on port 5000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:5000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Handler that returns the pokemon name and basic information
async fn pokemon_name_handler(Path(name): Path<String>) -> (StatusCode, Json<Pokemon>) {
    todo!()
}

// Handler that returns the pokemon translated name and basic information
async fn pokemon_translated_name_handler(Path(name): Path<String>) -> (StatusCode, Json<Pokemon>) {
    todo!()
}

// The Pokemon output response
#[derive(Serialize)]
struct Pokemon {
    name: String,
    description: String,
    habitat: String,
    is_legendary: bool,
}
