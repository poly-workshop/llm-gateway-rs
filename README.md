# llm-gateway-rs

A lightweight, multi-provider LLM API gateway built with Rust and Axum.

Routes OpenAI-compatible `/v1/chat/completions` requests to multiple upstream providers (OpenAI, OpenRouter, DashScope) based on the requested model name. Manages user-facing API keys with generation, rotation, and revocation.

## Features

- **Multi-provider support** — OpenAI, OpenRouter, DashScope (any OpenAI-compatible API)
- **Model routing** — Map user-facing model names to specific providers with optional name rewriting
- **User Key management** — Generate `sk-{uuid}` keys, rotate (old key instantly invalidated), soft-delete
- **Streaming** — Full SSE streaming passthrough for `stream: true` requests
- **Two-tier caching** — Redis (hot) for O(1) key validation & model routing, PostgreSQL (cold) for persistence
- **Admin API** — Protected by a static admin key; manage providers, models, and user keys

## Architecture

```text
Client ──► Gateway (/v1/chat/completions) ──► Provider (OpenAI / OpenRouter / DashScope)
              │
              ├─ User Key auth (Redis SET → PG fallback)
              ├─ Model resolution (Redis HASH → PG fallback)
              └─ Request rewrite (model name) + proxy
```

```text
src/
├── main.rs              # Entrypoint: init, migrations, server
├── config.rs            # Env-based configuration
├── state.rs             # Shared AppState (PgPool, Redis, HttpClient)
├── error.rs             # Unified error type → HTTP responses
├── middleware/
│   └── auth.rs          # Admin key + User key auth middleware
├── models/
│   ├── user_key.rs      # UserKey, UserKeyInfo, UserKeyCreated
│   ├── provider.rs      # Provider, ProviderInfo, ProviderKind
│   └── model.rs         # Model, ModelInfo, ModelRoute
├── routes/
│   ├── admin.rs         # CRUD for keys, providers, models
│   └── proxy.rs         # /v1/chat/completions proxy
└── services/
    ├── key_service.rs   # Key generation, hashing, validation, rotation
    ├── provider_service.rs  # Provider CRUD
    └── model_service.rs     # Model CRUD, route resolution, Redis cache
```

## Quick Start

### Prerequisites

- Rust 1.75+
- Docker & Docker Compose (for PostgreSQL and Redis)

### 1. Clone and configure

```bash
git clone <repo-url> && cd llm-gateway-rs
cp .env.example .env
```

Edit `.env`:

```dotenv
DATABASE_URL=postgres://postgres:postgres@localhost:5432/llm_gateway
REDIS_URL=redis://127.0.0.1:6379
ADMIN_KEY=my-secret-admin-key
LISTEN_ADDR=0.0.0.0:8080
```

### 2. Start dependencies

```bash
docker compose up -d
```

### 3. Run the gateway

```bash
cargo run
```

The server starts on `http://localhost:8080`. Database migrations run automatically on startup.

## Admin API

All admin endpoints require `Authorization: Bearer <ADMIN_KEY>`.

### Providers

```bash
# Register an OpenAI provider
curl -X POST http://localhost:8080/admin/providers \
  -H "Authorization: Bearer $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "openai-main",
    "kind": "openai",
    "api_key": "sk-your-openai-key"
  }'

# Register an OpenRouter provider
curl -X POST http://localhost:8080/admin/providers \
  -H "Authorization: Bearer $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "openrouter",
    "kind": "openrouter",
    "api_key": "sk-or-your-key"
  }'

# Register a DashScope provider
curl -X POST http://localhost:8080/admin/providers \
  -H "Authorization: Bearer $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "dashscope",
    "kind": "dashscope",
    "api_key": "sk-your-dashscope-key"
  }'

# List all providers
curl http://localhost:8080/admin/providers \
  -H "Authorization: Bearer $ADMIN_KEY"

# Update a provider
curl -X PUT http://localhost:8080/admin/providers/<provider-id> \
  -H "Authorization: Bearer $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{ "api_key": "sk-new-key" }'

# Delete a provider
curl -X DELETE http://localhost:8080/admin/providers/<provider-id> \
  -H "Authorization: Bearer $ADMIN_KEY"
```

Supported `kind` values and their default `base_url`:

| Kind | Default Base URL |
| ---- | --------------- |
| `openai` | `https://api.openai.com/v1` |
| `openrouter` | `https://openrouter.ai/api/v1` |
| `dashscope` | `https://dashscope.aliyuncs.com/compatible-mode/v1` |

You can override `base_url` when creating a provider.

### Models

```bash
# Map "gpt-4o" to the OpenAI provider (same name on provider side)
curl -X POST http://localhost:8080/admin/models \
  -H "Authorization: Bearer $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "gpt-4o",
    "provider_id": "<openai-provider-uuid>"
  }'

# Map "qwen-max" to DashScope with a different provider-side name
curl -X POST http://localhost:8080/admin/models \
  -H "Authorization: Bearer $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "qwen-max",
    "provider_id": "<dashscope-provider-uuid>",
    "provider_model_name": "qwen-max-latest"
  }'

# List all models
curl http://localhost:8080/admin/models \
  -H "Authorization: Bearer $ADMIN_KEY"

# Delete a model
curl -X DELETE http://localhost:8080/admin/models/<model-id> \
  -H "Authorization: Bearer $ADMIN_KEY"
```

### User Keys

```bash
# Create a new user key (plaintext shown only once!)
curl -X POST http://localhost:8080/admin/keys \
  -H "Authorization: Bearer $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{ "name": "my-app" }'
# → { "id": "...", "key": "sk-550e8400-e29b-41d4-a716-446655440000", ... }

# List all keys (prefix only, no plaintext)
curl http://localhost:8080/admin/keys \
  -H "Authorization: Bearer $ADMIN_KEY"

# Rotate a key (old key immediately invalidated, new plaintext returned)
curl -X POST http://localhost:8080/admin/keys/<key-id>/rotate \
  -H "Authorization: Bearer $ADMIN_KEY"

# Revoke a key
curl -X DELETE http://localhost:8080/admin/keys/<key-id> \
  -H "Authorization: Bearer $ADMIN_KEY"
```

## Proxy API

Use the gateway just like the OpenAI API, replacing the base URL and using a gateway-issued user key.

```bash
# Non-streaming
curl http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer sk-550e8400-e29b-41d4-a716-446655440000" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [{ "role": "user", "content": "Hello!" }]
  }'

# Streaming
curl http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer sk-550e8400-e29b-41d4-a716-446655440000" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [{ "role": "user", "content": "Hello!" }],
    "stream": true
  }'
```

The gateway will:

1. Validate the user key (Redis `SISMEMBER` → PG fallback)
2. Resolve the model name to a provider (Redis `HGET` → PG fallback)
3. Rewrite the `model` field if `provider_model_name` differs
4. Proxy the request to the upstream provider with the provider's API key
5. Stream or return the response as-is

## API Reference

| Method | Path | Auth | Description |
| ------ | ---- | ---- | ----------- |
| `POST` | `/admin/providers` | Admin | Register a provider |
| `GET` | `/admin/providers` | Admin | List all providers |
| `PUT` | `/admin/providers/{id}` | Admin | Update a provider |
| `DELETE` | `/admin/providers/{id}` | Admin | Delete a provider |
| `POST` | `/admin/models` | Admin | Register a model mapping |
| `GET` | `/admin/models` | Admin | List all models |
| `DELETE` | `/admin/models/{id}` | Admin | Delete a model |
| `POST` | `/admin/keys` | Admin | Create a user key |
| `GET` | `/admin/keys` | Admin | List all user keys |
| `POST` | `/admin/keys/{id}/rotate` | Admin | Rotate a user key |
| `DELETE` | `/admin/keys/{id}` | Admin | Revoke a user key |
| `POST` | `/v1/chat/completions` | User Key | Proxy chat completions |

## Environment Variables

| Variable | Required | Default | Description |
| -------- | -------- | ------- | ----------- |
| `DATABASE_URL` | Yes | — | PostgreSQL connection string |
| `REDIS_URL` | No | `redis://127.0.0.1:6379` | Redis connection string |
| `ADMIN_KEY` | Yes | — | Secret key for admin API access |
| `LISTEN_ADDR` | No | `0.0.0.0:8080` | Server listen address |

## Design Decisions

- **Key format**: `sk-{uuid v4}` — 39 characters, recognizable prefix
- **Key storage**: Only SHA-256 hashes stored; plaintext returned once on create/rotate (like GitHub PATs)
- **Redis strategy**: `SET` for key hashes (`SISMEMBER` O(1)), `HASH` for model routes (`HGET` O(1))
- **Cache warm-up**: On startup, all active keys and model routes are loaded from PG into Redis
- **Streaming**: Raw byte-stream passthrough — no SSE parsing, minimal latency
- **Provider API keys**: Stored in PG, listed with masked preview (`sk-x...xxxx`), never cached in plaintext outside the routing lookup

## License

MIT
