mod error;
mod pokemon;
mod translation;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use tokio::signal;
use tracing::info;

use error::Result;
use pokemon::{Pokemon, PokemonService};
use translation::TranslationService;

#[derive(Clone)]
struct AppState {
    pokemon_service: Arc<PokemonService>,
    translation_service: Arc<TranslationService>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    info!("Starting Pokedex API server");

    let pokemon_service = Arc::new(PokemonService::new());
    let translation_service = Arc::new(TranslationService::new());

    let state = AppState {
        pokemon_service,
        translation_service,
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/pokemon/{name}", get(get_pokemon))
        .route("/pokemon/translated/{name}", get(get_translated_pokemon))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:5000")
        .await
        .map_err(|e| error::AppError::Internal(format!("Failed to bind: {}", e)))?;

    info!("Server listening on http://0.0.0.0:5000");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| error::AppError::Internal(format!("Server error: {}", e)))?;

    info!("Server shutdown complete");
    Ok(())
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "pokedex-api"
    }))
}

async fn get_pokemon(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Pokemon>> {
    info!("Fetching pokemon: {}", name);
    let pokemon = state.pokemon_service.get_pokemon(&name).await?;
    Ok(Json(pokemon))
}

async fn get_translated_pokemon(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Pokemon>> {
    info!("Fetching translated pokemon: {}", name);
    let mut pokemon = state.pokemon_service.get_pokemon(&name).await?;

    if let Some(description) = &pokemon.description {
        if let Ok(translated) = state
            .translation_service
            .translate(description, &pokemon.habitat, pokemon.is_legendary)
            .await
        {
            pokemon.description = Some(translated);
        }
    }

    Ok(Json(pokemon))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl+C, starting graceful shutdown"),
        _ = terminate => info!("Received SIGTERM, starting graceful shutdown"),
    }
}
