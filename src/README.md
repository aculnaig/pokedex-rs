# pokedex-rs

## How to run server locally

`cargo run`

## How to run unit tests locally

`cargo test`

## How to run with Docker

```bash
# Build the Docker image
docker build -t pokedex-api .

# Run the container
docker run -p 5000:5000 pokedex-api

# Or run in detached mode
docker run -d -p 5000:5000 --name pokedex pokedex-api

# View logs
docker logs pokedex

# Stop the container
docker stop pokedex

# Remove the container
docker rm pokedex
```
