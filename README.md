# Pokedex API

A production-grade REST API for Pokemon information with fun translations.

## Features

- 🚀 High-performance async Rust implementation
- 🔄 Graceful shutdown support
- 📊 Structured JSON logging
- 🔍 Health check and readiness endpoints
- ⚡ Request timeout and compression
- 🌐 CORS support
- 🐳 Docker and docker-compose ready
- 📈 HTTP tracing with latency metrics
- ⚙️ Environment-based configuration
- 🧪 Comprehensive test coverage

## Endpoints

### Health Check
```bash
GET /health
```
Returns server health status.

### Readiness Check
```bash
GET /readiness
```
Checks if external services are reachable.

### Get Pokemon
```bash
GET /pokemon/{name}
```
Returns basic Pokemon information.

### Get Translated Pokemon
```bash
GET /pokemon/translated/{name}
```
Returns Pokemon information with translated description.

## Configuration

Configuration is done via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `0.0.0.0` | Server host |
| `PORT` | `5000` | Server port |
| `POKEAPI_BASE_URL` | `https://pokeapi.co/api/v2` | PokeAPI base URL |
| `TRANSLATION_API_BASE_URL` | `https://api.funtranslations.com/translate` | Translation API base URL |
| `HTTP_TIMEOUT_SECS` | `10` | HTTP client timeout |
| `REQUEST_TIMEOUT_SECS` | `30` | Request timeout |
| `RUST_LOG` | `info` | Log level |

## Development

### Prerequisites
- Rust 1.75 or later
- Docker (optional)

### Build
```bash
cargo build --release
```

### Run
```bash
cargo run
```

### Test
```bash
cargo test
```

### Lint
```bash
cargo clippy -- -D warnings
```

## Docker

### Build
```bash
docker build -t pokedex-api .
```

### Run
```bash
docker run -p 5000:5000 pokedex-api
```

### Docker Compose
```bash
docker-compose up -d
```

## Production Considerations

1. **Environment Variables**: Set appropriate timeouts and URLs
2. **Logging**: Use `RUST_LOG=info` or higher for production
3. **Health Checks**: Configure Kubernetes/Docker health checks
4. **Rate Limiting**: Consider adding rate limiting middleware
5. **Caching**: Add Redis/in-memory cache for Pokemon data
6. **Metrics**: Integrate Prometheus metrics
7. **Observability**: Add distributed tracing (OpenTelemetry)

## Architecture
```
src/
├── main.rs           # Application entry point and HTTP handlers
├── config.rs         # Configuration management
├── error.rs          # Error types and handling
├── pokemon.rs        # Pokemon service
└── translation.rs    # Translation service
```

## Performance

- Binary size: ~15-20MB (with strip and LTO)
- Memory usage: ~10-20MB at rest
- Request latency: <100ms (depends on external APIs)
- Concurrent requests: 1000+ (with default tokio runtime)
