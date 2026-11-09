# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build                    # Debug build
cargo build --release          # Release build

# Test
cargo test                     # Unit tests
./test test_all                # Full integration test suite (requires mosquitto)
./test test_<name>             # Single integration test (e.g. ./test test_mqtt, ./test test_api)

# Lint / Format
cargo fmt                      # Auto-format
cargo fmt --check              # Check formatting (used in CI)
cargo clippy                   # Lint
```

CI runs `cargo fmt --check`, `cargo clippy`, then `./test test_all`.

## Architecture

hvents is a configuration-driven home automation event system. Users define events in YAML; the system triggers and chains them based on time, MQTT messages, HTTP requests, file changes, or input devices.

### Core Concepts

**Event** — a named unit in the YAML config with a type (trigger or action) and optional `next_event` for chaining. Data flows through chains via merge policies (`yes`, `no`, `overwrite`, `yes_if_empty`).

**Queue** (`src/executors/queue.rs`) — the central hub. All executor threads send events to a shared `mpsc` channel; the queue processes them in order: render templates → validate → execute → push `next_event` if any → optionally persist state.

**Executors** (`src/executors/`) — one thread per input source:
- `mqtt.rs` — MQTT subscriptions (uses `rumqttc`)
- `http.rs` — HTTP listen endpoints (uses `tiny_http`)
- `time.rs` — scheduled and recurring events
- `file.rs` — file watching (uses `notify`)
- `evdev.rs` — scan code reader (Linux only, `evdev` feature)

**Pools** (`src/pools/`) — shared connection pools for MQTT clients, HTTP servers, and `reqwest` clients. Multiple events can share one connection.

**Renderer** (`src/renderer.rs`) — Handlebars templating applied to topics, URLs, bodies, commands, and file paths before execution. Includes custom helpers for date/time and metrics lookup.

**Database** (`src/database.rs`) — simple key-value store for persisting event state across restarts.

**Events** (`src/events/`) — one file per event type (21+ types). Each implements serialization via serde and is dispatched by the queue.

**Gherkin** (`src/gherkin/`) — BDD `.feature` file support for testing; converts scenarios to event sequences.

### Data Flow

```
Trigger (time/MQTT/HTTP/file/device)
  → executor thread → queue channel
  → queue: render templates with Handlebars
  → execute action (publish/write/call/command)
  → if next_event: push with merged data
  → optionally persist state to database
```

### Feature Flags

Most capabilities are behind optional Cargo features (all enabled by default): `metrics`, `reqwest`, `notify`, `evdev`, `rumqttc`, `handlebars`, `tiny_http`. Use `--no-default-features` to build a minimal binary.
