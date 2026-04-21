# shuriken-quickstart-rs

Quickstart examples repo for the `shuriken-sdk` Rust SDK.

## Structure

- `src/lib.rs` — shared helpers (client constructors, formatters, error handling)
- `examples/` — 20 numbered example scripts

### Example categories

- **01–06** Basic read-only examples (account, tokens, portfolio, perps markets)
- **07–09** Trading examples that execute writes (swaps, triggers, perp orders)
- **10–13** WebSocket streaming examples
- **14–20** Advanced composite examples combining multiple SDK features

## Quick start

    cp .env.example .env       # add your API key
    cargo run --example 01_account_info

## Adding new examples

Add a new file `examples/NN_name.rs` with a `#[tokio::main]` async main.
Use helpers from `shuriken_quickstart_rs` (the lib crate).

## Build & lint

    cargo check
    cargo clippy -- -D warnings
    cargo fmt --check
