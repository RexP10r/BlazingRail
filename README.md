# BlazingRail

A minimal event ingestion router: validate -> queue -> batch -> sink.  
Built with Axum, Tokio, and `rdkafka`. 

## Usage

**Manual**
```bash
cargo run --release
```

**Docker**
```bash
docker compose up -d
```

**Ingest an event**
```bash
curl -X POST http://localhost:3000/v1/events \
  -H "Content-Type: application/json" \
  -d '{"event_type":"user_action","payload":{"id":42}}'
```

## Configuration

All settings via env vars. Defaults are built in — override only what you need.
Ensure that your override values do not conflict with `docker-compose.yml` environment.
Full list: [`docs/env_example.txt`](docs/env_example.txt)

## API

| Endpoint | Method | Response |
|----------|--------|----------|
| `/v1/events` | POST | `202 Accepted` / `400` / `503` |
| `/health` | GET | `200 OK` |
| `/ready` | GET | `200 OK` / `503 Service Unavailable` |
| `/metrics` | GET | Prometheus text format |

Event constraints (setting them via env vars is comming):
- `event_type`: non-empty, <=64 chars
- `payload`: serialized JSON <=4096 bytes

## Observability

```bash
# Metrics
## Look once
curl -s http://localhost:3000/metrics | grep blazingrail
## Watch stream
watch -n1 'curl -s http://localhost:3000/metrics | grep -E "blazingrail"'

# Logs (Docker)
docker compose logs -f app

# Logs (manual)
RUST_LOG=info cargo run --release
```

Metrics:
- `blazingrail_queue_depth` — current channel occupancy
- `blazingrail_batch_flush_duration_seconds` — flush latency
- `blazingrail_sink_errors_total` — failed sends

## Architecture (brief)
```
> tree --gitignore
.
├── Cargo.toml
├── Dockerfile
├── README.md
├── crates
│   ├── api
│   │   ├── Cargo.toml
│   │   └── src
│   │       ├── error.rs
│   │       ├── handler.rs
│   │       ├── main.rs
│   │       └── state.rs
│   ├── common
│   │   ├── Cargo.toml
│   │   └── src
│   │       ├── config.rs
│   │       ├── lib.rs
│   │       └── models.rs
│   └── pipeline
│       ├── Cargo.toml
│       └── src
│           ├── batcher.rs
│           ├── circuit_breaker.rs
│           ├── lib.rs
│           ├── sink.rs
│           └── sinks
│               ├── file.rs
│               ├── kafka.rs
│               └── mod.rs
├── docker-compose.yml
├── docs
│   └── env_example.txt
├── kafka_routing.yaml
└── tests
    └── k6
        └── load.js

```

```
HTTP POST /v1/events
        │
   [ Validate ] ──> 400 on error
        │
   [ mpsc::Channel ] <── backpressure when full (503)
        │
   [ Batcher ]
   └─ flush on: reached batch_size OR flush_timeout_ms
        │
   [ CircuitBreaker ] (optional)
   ├─ primary: KafkaSink
   ├─ fallback: FileSink
   └─ opens after N failures, retries after timeout
        │
   [ Sink ]
   ├─ KafkaSink: async publish with routing with respect to event_type
   └─ FileSink: buffered JSONL append (tokio::task::spawn_blocking)
```

## Kafka

When `ENABLE_KAFKA=true`, routing is controlled by `kafka_routing.yaml`:

```yaml
default_topic: "blazing_events"
topic_mapping:
  "load_test": "test"
```

Debug commands (examples):
```bash
# List topics
docker compose exec kafka /opt/kafka/bin/kafka-topics.sh \
  --bootstrap-server localhost:9092 --list

# Interactive shell
docker compose exec kafka bash
```
The `/opt/kafka/bin` directory contains a number of scripts, so you should consult the official repository.

## Benchmark
Load tests use `k6` located in `tests/k6/load.js`(sorry for non rust code in this repo :3).
Before running, ensure your shell can handle many concurrent connections via `ulimit`.
Minimum pipeline:
```bash
ulimit -n 4096
k6 run tests/k6/load.js
```
