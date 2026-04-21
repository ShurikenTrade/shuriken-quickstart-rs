# Shuriken Rust SDK Redesign (v0.3.0) & Quickstart Bootstrap

## Context

The TypeScript SDK (`@shuriken/sdk-ts` v0.5.0) is the mature reference implementation. The Rust SDK (`shuriken-sdk` v0.2.0) has full REST API coverage and WebSocket support, but its API design has issues that are unidiomatic in Rust:

- A single `ShurikenClient` struct owns both HTTP and WS resources, creating ownership conflicts
- WebSocket uses 7 `Arc<Mutex/RwLock<...>>` fields to work around shared `&self`
- WS subscriptions are callback-based with untyped `serde_json::Value` payloads
- WsHandle duplicates HTTP error handling logic from the main client
- No namespace accessors for API modules (flat method names like `get_swap_quote`)

The `shuriken-api-types` crate (v0.3.1) already has full type parity with the TS SDK's stream payload types across all 22 streams (SVM, EVM, Alpha, Portfolio/Automation).

The quickstart repo (`shuriken-quickstart-rs`) needs bootstrapping from scratch to mirror the TS quickstart's 20 examples.

## Goals

1. Redesign the Rust SDK to v0.3.0 with idiomatic Rust patterns
2. Bootstrap the quickstart repo with all 20 examples
3. Achieve feature parity with the TS SDK

## Non-Goals

- Unifying REST API types into `shuriken-api-types` (future work)
- Shared config struct / builder pattern (add later if needed)
- Backwards compatibility with v0.2.0 (pre-1.0, clean break)

---

## Part 1: SDK Redesign

### 1.1 Client Architecture

Two independent clients, each with simple constructors:

```rust
// REST client — Clone-able, cheap to pass around
let client = ShurikenHttpClient::new("sk_...")?;
let client = ShurikenHttpClient::with_base_url("sk_...", "https://custom.api.com")?;

// WS client — owns the connection, independent lifecycle
let mut ws = ShurikenWsClient::new("sk_...")?;
let mut ws = ShurikenWsClient::with_base_url("sk_...", "https://custom.api.com")?;
```

No shared config struct for now. Each client constructs its own `reqwest::Client` with auth headers. Both support `::new(api_key)` for defaults and `::with_base_url(api_key, url)` for custom endpoints. A shared `ShurikenConfig` can be added later as a backwards-compatible addition if divergent config needs arise.

### 1.2 REST Client: `ShurikenHttpClient`

**Structure:**
- Holds `reqwest::Client` + `base_url: Arc<str>`
- Implements `Clone` (cheap — both fields are Arc'd internally)
- HTTP verb helpers (`get`, `post`, `put`, `patch`, `delete`) stay as `pub(crate)` methods
- `handle_response` stays as-is for error extraction

**Namespace accessors** return lightweight borrowed structs:

```rust
impl ShurikenHttpClient {
    pub fn account(&self) -> AccountApi<'_> { AccountApi(self) }
    pub fn tokens(&self) -> TokensApi<'_> { TokensApi(self) }
    pub fn swap(&self) -> SwapApi<'_> { SwapApi(self) }
    pub fn portfolio(&self) -> PortfolioApi<'_> { PortfolioApi(self) }
    pub fn trigger(&self) -> TriggerApi<'_> { TriggerApi(self) }
    pub fn perps(&self) -> PerpsApi<'_> { PerpsApi(self) }
}
```

Each API struct is a zero-cost wrapper:

```rust
pub struct SwapApi<'a>(&'a ShurikenHttpClient);

impl SwapApi<'_> {
    pub async fn get_quote(&self, params: &GetSwapQuoteParams) -> Result<SwapQuote, ShurikenError> {
        self.0.post("/api/v2/swap/quote", params).await
    }
    pub async fn execute(&self, params: &ExecuteSwapParams) -> Result<SwapStatus, ShurikenError> {
        self.0.post("/api/v2/swap/execute", params).await
    }
    // ... etc
}
```

Method names are shortened from the current flat style: `get_swap_quote` becomes `swap().get_quote()`, `search_tokens` becomes `tokens().search()`, etc.

**Request/response types** stay defined inline in each `http/*.rs` module (same as today's `api/*.rs` files).

### 1.3 WebSocket Client: `ShurikenWsClient`

**Structure:**
- Owns connection state directly (no Arc/Mutex wrapping since `&mut self` is used for mutating operations)
- `connect(&mut self)` establishes WebSocket connection
- `subscribe(...)` returns a `Subscription<T>` that implements `Stream<Item = T>`
- `disconnect(&mut self)` tears down the connection

**Stream-based subscriptions** instead of callbacks:

```rust
ws.connect().await?;

let mut swaps = ws.subscribe(streams::SVM_TOKEN_SWAPS, SvmTokenFilter {
    token_address: "So11...".into(),
}).await?;

// Composes with async ecosystem
while let Some(event) = swaps.next().await {
    println!("swap: {} SOL @ ${}", event.size_sol, event.price_usd);
}

// Multiple concurrent streams via tokio::select!
tokio::select! {
    Some(swap) = swaps.next() => { /* handle */ }
    Some(grad) = graduations.next() => { /* handle */ }
}
```

**`Subscription<T>`:**
- Wraps a `tokio::sync::mpsc::UnboundedReceiver<T>`
- Implements `futures_core::Stream<Item = T>`
- Holds a subscription ID; dropping it unsubscribes automatically (RAII)
- The internal event loop dispatches deserialized, typed events into the correct channel

**State changes** also use the Stream pattern:

```rust
let mut state = ws.on_state_change();
tokio::spawn(async move {
    while let Some(event) = state.next().await {
        println!("WS state: {:?}", event.state);
    }
});
```

**Internals:**
- Session bootstrap (`fetch_session`) and expansion (`expand_session`) logic stays the same
- Pusher protocol handling stays the same
- The event loop (spawned task) dispatches messages by matching channel + event, deserializes into `T`, sends into the subscription's mpsc channel
- `http_post` is now a private method on `ShurikenWsClient` (still duplicates some logic from HTTP client — acceptable since they're independent)

### 1.4 Typed Stream Definitions

A `StreamDef<P, F>` struct binds stream name, payload type, and filter type at compile time:

```rust
pub struct StreamDef<P, F> {
    pub name: &'static str,
    _phantom: PhantomData<(P, F)>,
}
```

Constants for all 22 streams:

```rust
pub mod streams {
    use shuriken_api_types::*;

    // SVM streams
    pub const SVM_TOKEN_SWAPS: StreamDef<svm::SwapEvent, SvmTokenFilter> = StreamDef::new("svm.token.swaps");
    pub const SVM_TOKEN_POOL_INFO: StreamDef<svm::TokenPoolEvent, SvmTokenFilter> = StreamDef::new("svm.token.poolInfo");
    pub const SVM_TOKEN_BALANCES: StreamDef<svm::TokenBalanceEvent, SvmTokenFilter> = StreamDef::new("svm.token.balances");
    pub const SVM_TOKEN_DISTRIBUTION_STATS: StreamDef<analytics::TokenDistributionStatsEvent, SvmTokenFilter> = StreamDef::new("svm.token.distributionStats");
    pub const SVM_TOKEN_HOLDER_STATS: StreamDef<analytics::HolderStatsEvent, SvmTokenFilter> = StreamDef::new("svm.token.holderStats");
    pub const SVM_WALLET_NATIVE_BALANCE: StreamDef<wallet::SvmNativeBalanceEvent, SvmWalletFilter> = StreamDef::new("svm.wallet.nativeBalance");
    pub const SVM_WALLET_TOKEN_BALANCES: StreamDef<wallet::SvmTokenBalanceEvent, SvmWalletFilter> = StreamDef::new("svm.wallet.tokenBalances");
    pub const SVM_BONDING_CURVE_CREATIONS: StreamDef<svm::BondingCurveCreationEvent, NoFilter> = StreamDef::new("svm.bondingCurve.creations");
    pub const SVM_BONDING_CURVE_GRADUATIONS: StreamDef<svm::BondingCurveGraduationEvent, NoFilter> = StreamDef::new("svm.bondingCurve.graduations");

    // EVM streams
    pub const EVM_TOKEN_SWAPS: StreamDef<evm::SwapEvent, EvmTokenFilter> = StreamDef::new("evm.token.swaps");
    pub const EVM_TOKEN_POOL_INFO: StreamDef<evm::TokenPoolEvent, EvmTokenFilter> = StreamDef::new("evm.token.poolInfo");
    pub const EVM_TOKEN_BALANCES: StreamDef<evm::TokenBalanceEvent, EvmTokenFilter> = StreamDef::new("evm.token.balances");
    pub const EVM_WALLET_NATIVE_BALANCE: StreamDef<wallet::EvmNativeBalanceEvent, EvmWalletFilter> = StreamDef::new("evm.wallet.nativeBalance");
    pub const EVM_WALLET_TOKEN_BALANCES: StreamDef<evm::TokenBalanceEvent, EvmWalletFilter> = StreamDef::new("evm.wallet.tokenBalances");

    // Alpha streams
    pub const ALPHA_SIGNAL_FEED_GLOBAL: StreamDef<alpha::SignalFeedUpdateEvent, NoFilter> = StreamDef::new("alpha.signalFeedGlobal");
    pub const ALPHA_SIGNAL_FEED_PERSONAL: StreamDef<alpha::SignalFeedUpdateEvent, NoFilter> = StreamDef::new("alpha.signalFeedPersonal");
    pub const ALPHA_SIGNAL_FEED_PROFILE: StreamDef<alpha::SignalFeedUpdateEvent, AlphaProfileFilter> = StreamDef::new("alpha.signalFeedProfile");
    pub const ALPHA_SIGNAL_FEED_NAMED: StreamDef<alpha::SignalFeedUpdateEvent, AlphaNamedFeedFilter> = StreamDef::new("alpha.signalFeedNamed");
    pub const ALPHA_PERSONAL: StreamDef<alpha::MessageEvent, NoFilter> = StreamDef::new("alpha.personal");

    // Portfolio / Automation streams
    pub const PORTFOLIO_NOTIFICATIONS: StreamDef<notification::NotificationEvent, NoFilter> = StreamDef::new("portfolio.notifications");
    pub const AUTOMATION_UPDATES: StreamDef<automation::AutomationEvent, NoFilter> = StreamDef::new("automation.updates");
}
```

**Filter types:**

```rust
pub struct SvmTokenFilter { pub token_address: String }
pub struct SvmWalletFilter { pub wallet_address: String }
pub struct EvmTokenFilter { pub chain_id: String, pub token_address: String }
pub struct EvmWalletFilter { pub wallet_address: String }
pub struct AlphaProfileFilter { pub profile_id: String }
pub struct AlphaNamedFeedFilter { pub feed_id: String }
pub struct NoFilter;
```

Each filter type implements a `IntoFilterMap` trait that converts to `HashMap<String, String>` for the session API.

**Subscribe signature:**

```rust
impl ShurikenWsClient {
    /// Type-safe subscription — stream ID, payload, and filter enforced at compile time
    pub async fn subscribe<P, F>(
        &mut self,
        stream: StreamDef<P, F>,
        filter: F,
    ) -> Result<Subscription<P>, ShurikenError>
    where
        P: DeserializeOwned + Send + 'static,
        F: IntoFilterMap,
    { ... }

    /// Raw subscription — untyped, for advanced/unknown streams
    pub async fn subscribe_raw(
        &mut self,
        stream: &str,
        filter: HashMap<String, String>,
    ) -> Result<Subscription<serde_json::Value>, ShurikenError>
    { ... }
}
```

### 1.5 Error Handling

`ShurikenError` stays the same — it's already clean:

```rust
pub enum ShurikenError {
    Auth(String),
    Api { status: u16, message: String, request_id: Option<String> },
    Session(String),
    Request(reqwest::Error),
    Decode(serde_json::Error),
}
```

Shared between both clients.

### 1.6 Crate Structure

```
shuriken-sdk/
├── src/
│   ├── lib.rs              # Re-exports, feature gates
│   ├── error.rs            # ShurikenError (shared)
│   ├── http/
│   │   ├── mod.rs          # ShurikenHttpClient struct + constructors + HTTP helpers
│   │   ├── account.rs      # AccountApi<'_> + request/response types
│   │   ├── tokens.rs       # TokensApi<'_> + types
│   │   ├── swap.rs         # SwapApi<'_> + types
│   │   ├── portfolio.rs    # PortfolioApi<'_> + types
│   │   ├── trigger.rs      # TriggerApi<'_> + types
│   │   └── perps.rs        # PerpsApi<'_> + types
│   └── ws/                 # Feature-gated behind "ws"
│       ├── mod.rs          # ShurikenWsClient struct + constructors
│       ├── connection.rs   # Pusher protocol, event loop, session management
│       ├── streams.rs      # StreamDef constants, filter types, IntoFilterMap
│       └── subscription.rs # Subscription<T> implementing Stream
├── tests/
│   └── serialization.rs    # Existing tests, updated for new module paths
```

**Feature flags:**
- Default: HTTP client only (`reqwest`, `serde`, `serde_json`, `tokio`, `thiserror`)
- `ws`: WebSocket client (`tokio-tungstenite`, `futures-util`, `tracing`, `futures-core`)

**Cargo.toml version:** `0.3.0`

### 1.7 Public API Surface

```rust
// Always available
pub use error::ShurikenError;
pub use http::ShurikenHttpClient;
pub use http::account::*;   // AccountApi, AccountInfo, AccountWallet, etc.
pub use http::tokens::*;    // TokensApi, TokenInfo, TokenPrice, etc.
pub use http::swap::*;      // SwapApi, SwapQuote, SwapStatus, etc.
pub use http::portfolio::*; // PortfolioApi, WalletBalance, etc.
pub use http::trigger::*;   // TriggerApi, TriggerOrder, etc.
pub use http::perps::*;     // PerpsApi, PerpMarket, etc.
pub use shuriken_api_types as types;

// Behind "ws" feature
pub use ws::ShurikenWsClient;
pub use ws::streams;
pub use ws::subscription::Subscription;
pub use ws::streams::{StreamDef, NoFilter, SvmTokenFilter, SvmWalletFilter, ...};
```

---

## Part 2: Quickstart Repo

### 2.1 Structure

```
shuriken-quickstart-rs/
├── Cargo.toml
├── Cargo.lock
├── .env.example
├── .gitignore
├── README.md
├── CLAUDE.md
├── rustfmt.toml
├── .github/
│   └── workflows/
│       └── ci.yml
├── src/
│   └── lib.rs              # Shared helpers
└── examples/
    ├── 01_account_info.rs
    ├── 02_search_tokens.rs
    ├── 03_token_analytics.rs
    ├── 04_swap_quote.rs
    ├── 05_portfolio_overview.rs
    ├── 06_browse_perp_markets.rs
    ├── 07_execute_swap.rs
    ├── 08_trigger_orders.rs
    ├── 09_perp_trading.rs
    ├── 10_stream_token_swaps.rs
    ├── 11_stream_wallet.rs
    ├── 12_stream_new_tokens.rs
    ├── 13_stream_graduated_tokens.rs
    ├── 14_token_sniper.rs
    ├── 15_whale_copy_trader.rs
    ├── 16_portfolio_rebalancer.rs
    ├── 17_new_token_screener.rs
    ├── 18_perps_hedger.rs
    ├── 19_trailing_stop.rs
    └── 20_watchlist_dashboard.rs
```

### 2.2 Dependencies

```toml
[package]
name = "shuriken-quickstart-rs"
version = "0.1.0"
edition = "2021"

[dependencies]
shuriken-sdk = { version = "0.3", features = ["ws"] }
tokio = { version = "1", features = ["full"] }
dotenvy = "0.15"
serde_json = "1"
futures-util = "0.3"
```

### 2.3 Shared Helpers (`src/lib.rs`)

```rust
pub fn create_http_client() -> ShurikenHttpClient { ... }
pub fn create_ws_client() -> ShurikenWsClient { ... }
pub fn format_usd(value: f64) -> String { ... }
pub fn format_token(value: f64, symbol: &str) -> String { ... }
pub fn format_pct(value: f64) -> String { ... }
pub fn log_section(title: &str) { ... }
pub fn log_json(label: &str, data: &impl serde::Serialize) { ... }
pub fn handle_error(err: ShurikenError) { ... }
```

Loads `.env` via `dotenvy`, reads `SHURIKEN_API_KEY` and optional `SHURIKEN_API_URL`.

### 2.4 Example Categories

**Basic read-only (01-06):**

| # | Name | SDK Features Used |
|---|------|-------------------|
| 01 | Account Info | `account().get_me()`, `get_wallets()`, `get_settings()`, `get_usage()` |
| 02 | Search Tokens | `tokens().search()`, `get()`, `get_price()` |
| 03 | Token Analytics | `tokens().get_price()`, `get_chart()`, `get_stats()`, `get_pools()` |
| 04 | Swap Quote | `swap().get_quote()` |
| 05 | Portfolio Overview | `portfolio().get_balances()`, `get_pnl()`, `get_positions()`, `get_history()` |
| 06 | Browse Perp Markets | `perps().get_markets()`, `get_market()` |

**Trading (07-09):**

| # | Name | SDK Features Used |
|---|------|-------------------|
| 07 | Execute Swap | `swap().execute()`, `get_status()` — polls until complete |
| 08 | Trigger Orders | `trigger().create()`, `list()` |
| 09 | Perp Trading | `perps().place_order()`, `get_orders()`, `cancel_order()` with TP/SL |

**Streaming (10-13):**

| # | Name | SDK Features Used |
|---|------|-------------------|
| 10 | Stream Token Swaps | `ws.subscribe(SVM_TOKEN_SWAPS, ...)` |
| 11 | Stream Wallet | `ws.subscribe(SVM_WALLET_NATIVE_BALANCE, ...)` |
| 12 | Stream New Tokens | `ws.subscribe(SVM_BONDING_CURVE_CREATIONS, ...)` |
| 13 | Stream Graduated Tokens | `ws.subscribe(SVM_BONDING_CURVE_GRADUATIONS, ...)` |

**Advanced composites (14-20):**

| # | Name | SDK Features Used |
|---|------|-------------------|
| 14 | Token Sniper | WS streams + token stats + swap execution |
| 15 | Whale Copy Trader | WS wallet monitoring + swap execution |
| 16 | Portfolio Rebalancer | Portfolio positions + token prices + swap quotes |
| 17 | New Token Screener | WS new tokens + stats, live leaderboard |
| 18 | Perps Hedger | Portfolio positions + perp positions + order placement |
| 19 | Trailing Stop | WS price stream + trigger order create/cancel |
| 20 | Watchlist Dashboard | Batch token lookup + stats + prices, periodic refresh |

**Trading examples (07-09, 14-15, 18-19)** use a `const DRY_RUN: bool = true` pattern for safety.

### 2.5 CI

```yaml
# .github/workflows/ci.yml
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo check
      - run: cargo clippy -- -D warnings
      - run: cargo fmt --check
```

### 2.6 Running Examples

```bash
cp .env.example .env
# Add your API key to .env

cargo run --example 01_account_info
cargo run --example 10_stream_token_swaps
cargo run --example 14_token_sniper
```
