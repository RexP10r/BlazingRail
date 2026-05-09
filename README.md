# BlazingRail
A blazing fast and easy to configure event router for web applications.

## Requirements
- Docker
- Docker compose
- K6
- cargo

## Configuration
There are variety of possible env vars, all of them presented in `docs/env_example.txt`.
Presented env vars are already configured as defaults in parser code, so one can safely delete unused env vars.

## Launching
There are 2 ways to launch it:
### Manusl 
```bash
cargo build --release
cargo run --release
```
### Runtime
```bash
docker compose build --no-cache
docker compose up -d --force-recreate
```

## Managing
### Metrics 
One can see metrics on the `/metrics` socket like this example below

```terminal 1
watch -n1 'curl -s http://localhost:3000/metrics | grep -E "blazingrail"'
```

```terminal 2
cargo run --release
```

```terminal 3
for i in {1..500}; do \
curl -s -X POST http://127.0.0.1:3000/v1/events \
-H "Content-Type: application/json" \
-d "{\"event_type\":\"bulk\",\"payload\":{}}" & done ; \
wait
```

### Logs
Logs will be able to be seen after `cargo run --release` already. 
But if one prefer to launch by via docker, logs can be seen the way below:
```bash
docker compose logs
```

### Kafka
Kafka launches by `docker compose` so one need to execute commands inside.
```bash
docker compose exec -it kafka /opt/kafka/bin/kafka-topics.sh \
--bootstrap-server localhost:9092 \
--list
```
Or if number commands are needed
```bash
docker compose exec kafka bash
```
