use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderValue, Method, header},
    response::IntoResponse,
    routing::get,
};
use std::{sync::Arc, time::Duration};
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{
    LatencyUnit,
    compression::CompressionLayer,
    cors::CorsLayer,
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::{Level, info};

mod config;
mod error;
mod pokemon;
mod translation;

use config::Config;
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
    // Initialize tracing with JSON formatting for production
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_line_number(true)
        .json()
        .init();

    info!("Starting Pokedex API server");

    // Load configuration
    let config = Config::from_env();
    info!("Configuration loaded: {:?}", config);

    // Initialize services with configuration
    let pokemon_service = Arc::new(PokemonService::new(
        config.pokeapi_base_url.clone(),
        config.http_timeout,
    ));

    let translation_service = Arc::new(TranslationService::new(
        config.translation_api_base_url.clone(),
        config.http_timeout,
    ));

    let state = AppState {
        pokemon_service,
        translation_service,
    };

    // Build router with middleware stack
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/readiness", get(readiness_check))
        .route("/pokemon/:name", get(get_pokemon))
        .route(
            "/pokemon/translated/:name",
            get(get_translated_pokemon),
        )
        .layer(
            ServiceBuilder::new()
                // Logging layer
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(
                            DefaultMakeSpan::new().level(Level::INFO),
                        )
                        .on_response(
                            DefaultOnResponse::new()
                                .level(Level::INFO)
                                .latency_unit(LatencyUnit::Millis),
                        ),
                )
                // Timeout layer
                .layer(TimeoutLayer::new(Duration::from_secs(
                    config.request_timeout,
                )))
                // Compression layer
                .layer(CompressionLayer::new())
                // CORS layer
                .layer(
                    CorsLayer::new()
                        .allow_origin(
                            "*".parse::<HeaderValue>().unwrap(),
                        )
                        .allow_methods([Method::GET])
                        .allow_headers([header::CONTENT_TYPE]),
                ),
        )
        .with_state(state);

    // Bind server
    let addr = format!("{}:{}", config.host, config.port);
    let listener =
        tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
            error::AppError::Internal(format!(
                "Failed to bind to {}: {}",
                addr, e
            ))
        })?;

    info!("Server listening on http://{}", addr);

    // Start server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| {
            error::AppError::Internal(format!("Server error: {}", e))
        })?;

    info!("Server shutdown complete");
    Ok(())
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "pokedex-api",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn readiness_check(
    State(state): State<AppState>,
) -> Result<impl IntoResponse> {
    // Check if external services are reachable
    let pokemon_ready =
        state.pokemon_service.health_check().await.is_ok();
    let translation_ready =
        state.translation_service.health_check().await.is_ok();

    if pokemon_ready && translation_ready {
        Ok(Json(serde_json::json!({
            "status": "ready",
            "services": {
                "pokeapi": "up",
                "translation": "up"
            }
        })))
    } else {
        Err(error::AppError::Internal(
            "Service not ready".to_string(),
        ))
    }
}

async fn get_pokemon(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Pokemon>> {
    info!(pokemon_name = %name, "Fetching pokemon");
    let pokemon = state.pokemon_service.get_pokemon(&name).await?;
    Ok(Json(pokemon))
}

async fn get_translated_pokemon(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Pokemon>> {
    info!(pokemon_name = %name, "Fetching translated pokemon");
    let mut pokemon =
        state.pokemon_service.get_pokemon(&name).await?;

    if let Some(description) = &pokemon.description {
        if let Ok(translated) = state
            .translation_service
            .translate(
                description,
                &pokemon.habitat,
                pokemon.is_legendary,
            )
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
