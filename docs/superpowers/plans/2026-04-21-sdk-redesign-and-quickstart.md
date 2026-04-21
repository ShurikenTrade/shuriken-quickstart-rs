# Shuriken Rust SDK v0.3.0 Redesign & Quickstart Bootstrap — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign the Rust SDK to use idiomatic Rust patterns (separate HTTP/WS clients, namespace accessors, typed stream subscriptions) and bootstrap a quickstart repo with 20 examples.

**Architecture:** Two independent clients (`ShurikenHttpClient` for REST, `ShurikenWsClient` for WebSocket) with simple constructors. REST client uses namespace accessors (`client.swap().get_quote()`). WS client returns `Stream<Item = T>` subscriptions with compile-time type safety via `StreamDef<P, F>` constants.

**Tech Stack:** Rust 2021 edition, reqwest 0.12, tokio 1, serde 1, tokio-tungstenite 0.26, futures-util 0.3, shuriken-api-types 0.3.1, thiserror 2, tracing 0.1

**Spec:** `docs/superpowers/specs/2026-04-21-sdk-redesign-and-quickstart-design.md`

**Repos:**
- SDK: `/Users/nik/Projects/shuriken/shuriken-sdk-rs/`
- Quickstart: `/Users/nik/Projects/shuriken/shuriken-quickstart-rs/`

---

## File Structure — SDK (`shuriken-sdk-rs`)

| File | Responsibility |
|------|---------------|
| `src/lib.rs` | Crate root: re-exports, feature gates |
| `src/error.rs` | `ShurikenError` enum (shared by both clients) |
| `src/http/mod.rs` | `ShurikenHttpClient` struct, constructors, HTTP verb helpers, namespace accessors |
| `src/http/account.rs` | `AccountApi<'_>` + request/response types |
| `src/http/tokens.rs` | `TokensApi<'_>` + request/response types |
| `src/http/swap.rs` | `SwapApi<'_>` + request/response types |
| `src/http/portfolio.rs` | `PortfolioApi<'_>` + request/response types |
| `src/http/trigger.rs` | `TriggerApi<'_>` + request/response types |
| `src/http/perps.rs` | `PerpsApi<'_>` + request/response types |
| `src/ws/mod.rs` | `ShurikenWsClient` struct, constructors, connect/disconnect/subscribe |
| `src/ws/connection.rs` | Pusher protocol, event loop, session management |
| `src/ws/streams.rs` | `StreamDef<P,F>` struct, 22 stream constants, filter types, `IntoFilterMap` trait |
| `src/ws/subscription.rs` | `Subscription<T>` implementing `Stream<Item = T>` |
| `tests/serialization.rs` | Updated deserialization/serialization tests for new module paths |

## File Structure — Quickstart (`shuriken-quickstart-rs`)

| File | Responsibility |
|------|---------------|
| `Cargo.toml` | Package manifest with `[[example]]` entries |
| `.gitignore` | Standard Rust gitignore |
| `.env.example` | API key template |
| `README.md` | Setup guide + example table |
| `CLAUDE.md` | Dev guidelines |
| `.github/workflows/ci.yml` | CI: check, clippy, fmt |
| `src/lib.rs` | Shared helpers: client constructors, formatters, error handling |
| `examples/01_account_info.rs` | Account profile, wallets, settings, usage |
| `examples/02_search_tokens.rs` | Search + get + price |
| `examples/03_token_analytics.rs` | Price, chart, stats, pools deep-dive |
| `examples/04_swap_quote.rs` | Read-only swap quote |
| `examples/05_portfolio_overview.rs` | Balances, PnL, positions, history |
| `examples/06_browse_perp_markets.rs` | List markets, deep-dive into one |
| `examples/07_execute_swap.rs` | Execute swap + poll status |
| `examples/08_trigger_orders.rs` | Create + list trigger orders |
| `examples/09_perp_trading.rs` | Place order with TP/SL, list, cancel |
| `examples/10_stream_token_swaps.rs` | WS: svm.token.swaps |
| `examples/11_stream_wallet.rs` | WS: svm.wallet.nativeBalance |
| `examples/12_stream_new_tokens.rs` | WS: svm.bondingCurve.creations |
| `examples/13_stream_graduated_tokens.rs` | WS: svm.bondingCurve.graduations |
| `examples/14_token_sniper.rs` | WS new tokens + stats + swap |
| `examples/15_whale_copy_trader.rs` | WS wallet monitor + copy trades |
| `examples/16_portfolio_rebalancer.rs` | Rebalance calculator (read-only) |
| `examples/17_new_token_screener.rs` | WS new tokens + live leaderboard |
| `examples/18_perps_hedger.rs` | Delta-hedge spot with perp shorts |
| `examples/19_trailing_stop.rs` | WS price stream + dynamic trigger orders |
| `examples/20_watchlist_dashboard.rs` | Multi-token periodic refresh dashboard |

---

## Part 1: SDK Redesign

All tasks in Part 1 are executed in `/Users/nik/Projects/shuriken/shuriken-sdk-rs/`.

### Task 1: Restructure crate layout and create ShurikenHttpClient

Move from flat `src/client.rs` + `src/api/` to `src/http/` module. Create `ShurikenHttpClient` as the new REST client.

**Files:**
- Delete: `src/client.rs`, `src/api/mod.rs`
- Create: `src/http/mod.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create `src/http/mod.rs` with `ShurikenHttpClient`**

```rust
// src/http/mod.rs
pub mod account;
pub mod perps;
pub mod portfolio;
pub mod swap;
pub mod tokens;
pub mod trigger;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::error::ShurikenError;

const DEFAULT_BASE_URL: &str = "https://api.shuriken.trade";

#[derive(Clone)]
pub struct ShurikenHttpClient {
    pub(crate) http: Client,
    pub(crate) base_url: String,
}

impl ShurikenHttpClient {
    pub fn new(api_key: &str) -> Result<Self, ShurikenError> {
        Self::with_base_url(api_key, DEFAULT_BASE_URL)
    }

    pub fn with_base_url(api_key: &str, base_url: &str) -> Result<Self, ShurikenError> {
        let mut headers = HeaderMap::new();
        let mut auth_value = HeaderValue::from_str(&format!("Bearer {api_key}"))
            .map_err(|e| ShurikenError::Auth(e.to_string()))?;
        auth_value.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth_value);

        let http = Client::builder().default_headers(headers).build()?;
        let base_url = base_url.trim_end_matches('/').to_string();

        Ok(Self { http, base_url })
    }

    // Namespace accessors
    pub fn account(&self) -> account::AccountApi<'_> {
        account::AccountApi(self)
    }

    pub fn tokens(&self) -> tokens::TokensApi<'_> {
        tokens::TokensApi(self)
    }

    pub fn swap(&self) -> swap::SwapApi<'_> {
        swap::SwapApi(self)
    }

    pub fn portfolio(&self) -> portfolio::PortfolioApi<'_> {
        portfolio::PortfolioApi(self)
    }

    pub fn trigger(&self) -> trigger::TriggerApi<'_> {
        trigger::TriggerApi(self)
    }

    pub fn perps(&self) -> perps::PerpsApi<'_> {
        perps::PerpsApi(self)
    }

    // Internal HTTP helpers
    pub(crate) fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    pub(crate) async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ShurikenError> {
        let resp = self.http.get(self.url(path)).send().await?;
        self.handle_response(resp).await
    }

    pub(crate) async fn get_with_query<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, ShurikenError> {
        let resp = self.http.get(self.url(path)).query(query).send().await?;
        self.handle_response(resp).await
    }

    pub(crate) async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<T, ShurikenError> {
        let resp = self.http.post(self.url(path)).json(body).send().await?;
        self.handle_response(resp).await
    }

    pub(crate) async fn put<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<T, ShurikenError> {
        let resp = self.http.put(self.url(path)).json(body).send().await?;
        self.handle_response(resp).await
    }

    pub(crate) async fn patch<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<T, ShurikenError> {
        let resp = self.http.patch(self.url(path)).json(body).send().await?;
        self.handle_response(resp).await
    }

    pub(crate) async fn delete<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, ShurikenError> {
        let resp = self.http.delete(self.url(path)).send().await?;
        self.handle_response(resp).await
    }

    pub(crate) async fn delete_with_body<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<T, ShurikenError> {
        let resp = self.http.delete(self.url(path)).json(body).send().await?;
        self.handle_response(resp).await
    }

    async fn handle_response<T: DeserializeOwned>(
        &self,
        resp: reqwest::Response,
    ) -> Result<T, ShurikenError> {
        let status = resp.status();
        let request_id = resp
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        if status == reqwest::StatusCode::UNAUTHORIZED {
            let text = resp.text().await.unwrap_or_default();
            return Err(ShurikenError::Auth(text));
        }

        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ShurikenError::Api {
                status: status.as_u16(),
                message: text,
                request_id,
            });
        }

        Ok(resp.json().await?)
    }
}
```

- [ ] **Step 2: Update `src/lib.rs`**

```rust
// src/lib.rs
pub mod http;
mod error;

#[cfg(feature = "ws")]
pub mod ws;

pub use error::ShurikenError;
pub use http::ShurikenHttpClient;

pub use shuriken_api_types as types;

// Re-export API modules at crate root for convenience
pub use http::account;
pub use http::perps;
pub use http::portfolio;
pub use http::swap;
pub use http::tokens;
pub use http::trigger;
```

- [ ] **Step 3: Delete old files**

```bash
rm src/client.rs src/api/mod.rs
rmdir src/api 2>/dev/null || true
```

Note: do not delete `src/api/*.rs` yet — they will be moved in the next tasks.

- [ ] **Step 4: Verify it compiles (will fail — API modules not yet moved)**

```bash
cargo check 2>&1 | head -5
```

Expected: compilation errors about missing `src/http/account.rs` etc. This confirms the structure is wired up correctly. We fix these in the next tasks.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor: restructure crate layout with ShurikenHttpClient and http/ module"
```

---

### Task 2: Move account API to namespace pattern

**Files:**
- Create: `src/http/account.rs` (from `src/api/account.rs`)
- Delete: `src/api/account.rs`

- [ ] **Step 1: Create `src/http/account.rs`**

Copy all types from `src/api/account.rs` unchanged. Replace the `impl ShurikenClient` block with `AccountApi<'_>`:

```rust
// src/http/account.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ShurikenHttpClient;
use crate::error::ShurikenError;

// ── Response types ──────────────────────────────────────────────────────────
// (all type definitions identical to src/api/account.rs — AccountInfo, AccountWallet,
//  SwapPreset, ChainPresets, WalletGroup, OneClickModeSettings, SelectedWallets,
//  DefaultWallets, TradeSettings, AccountSettings, AgentKeyConstraints,
//  AccountUsage, EnableMultisendResponse)

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    pub user_id: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountWallet {
    pub wallet_id: String,
    pub address: String,
    pub chain: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SwapPreset {
    #[serde(rename = "solana")]
    Solana {
        slippage_bps: u32,
        mev_protection_enabled: bool,
        custom_priority_fee_sol: Option<String>,
        bribe_amount_sol: Option<String>,
        max_price_impact_pct: Option<f64>,
    },
    #[serde(rename = "evm")]
    Evm {
        slippage_bps: u32,
        mev_protection_enabled: bool,
        max_price_impact_pct: Option<f64>,
        max_priority_fee_per_gas_gwei: Option<String>,
        bribe_amount_native: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainPresets {
    pub auto: SwapPreset,
    pub p1: SwapPreset,
    pub p2: SwapPreset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletGroup {
    pub id: String,
    pub name: String,
    pub wallet_ids: Vec<String>,
    pub network_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OneClickModeSettings {
    pub enabled: bool,
    pub buy_presets: Vec<String>,
    pub sell_presets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedWallets {
    pub wallet_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultWallets {
    pub default_wallet_by_network: HashMap<String, String>,
    pub selected_wallet_ids_by_network: HashMap<String, SelectedWallets>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeSettings {
    pub auto_enable_multisend: bool,
    pub chain_presets_buy: HashMap<String, ChainPresets>,
    pub chain_presets_sell: HashMap<String, ChainPresets>,
    pub default_wallets: DefaultWallets,
    pub one_click_mode: HashMap<String, OneClickModeSettings>,
    pub wallet_groups: Vec<WalletGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSettings {
    pub trade_settings: TradeSettings,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentKeyConstraints {
    pub buys_enabled: bool,
    pub sells_enabled: bool,
    pub max_executions_per_hour: u32,
    pub max_executions_per_day: u32,
    pub max_concurrent_executions: u32,
    pub max_limit_orders_per_day: u32,
    pub allow_custom_gas: bool,
    pub allow_bribes: bool,
    pub allowed_networks: Vec<u32>,
    pub allowed_wallet_ids: Vec<String>,
    pub max_buy_usd_per_trade: Option<f64>,
    pub max_buy_usd_per_day: Option<f64>,
    pub max_sell_usd_per_trade: Option<f64>,
    pub max_sell_usd_per_day: Option<f64>,
    pub max_limit_order_usd_per_order: Option<f64>,
    pub max_slippage_bps: Option<u32>,
    pub max_price_impact_pct: Option<f64>,
    pub max_sell_position_pct: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountUsage {
    pub key_id: String,
    pub scopes: Vec<String>,
    pub constraints: AgentKeyConstraints,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnableMultisendResponse {
    pub task_id: String,
    pub message: String,
}

// ── API ─────────────────────────────────────────────────────────────────────

pub struct AccountApi<'a>(pub(crate) &'a ShurikenHttpClient);

impl AccountApi<'_> {
    pub async fn get_me(&self) -> Result<AccountInfo, ShurikenError> {
        self.0.get("/api/v2/account/me").await
    }

    pub async fn get_settings(&self) -> Result<AccountSettings, ShurikenError> {
        self.0.get("/api/v2/account/settings").await
    }

    pub async fn update_settings(
        &self,
        settings: &AccountSettings,
    ) -> Result<AccountSettings, ShurikenError> {
        self.0.put("/api/v2/account/settings", settings).await
    }

    pub async fn get_usage(&self) -> Result<AccountUsage, ShurikenError> {
        self.0.get("/api/v2/account/usage").await
    }

    pub async fn get_wallets(&self) -> Result<Vec<AccountWallet>, ShurikenError> {
        self.0.get("/api/v2/account/wallets").await
    }

    pub async fn enable_multisend(
        &self,
        wallet_id: &str,
    ) -> Result<EnableMultisendResponse, ShurikenError> {
        self.0
            .post(
                &format!("/api/v2/account/wallets/{wallet_id}/enable-multisend"),
                &serde_json::Value::Object(Default::default()),
            )
            .await
    }
}
```

- [ ] **Step 2: Delete `src/api/account.rs`**

```bash
rm src/api/account.rs
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "refactor: move account API to namespace pattern (AccountApi)"
```

---

### Task 3: Move tokens API to namespace pattern

**Files:**
- Create: `src/http/tokens.rs` (from `src/api/tokens.rs`)
- Delete: `src/api/tokens.rs`

- [ ] **Step 1: Create `src/http/tokens.rs`**

Copy all types from `src/api/tokens.rs` unchanged. Replace the `impl ShurikenClient` block with `TokensApi<'_>`. Method renames: `get_token` → `get`, `search_tokens` → `search`, `batch_tokens` → `batch`, `get_token_price` → `get_price`, `get_token_chart` → `get_chart`, `get_token_stats` → `get_stats`, `get_token_pools` → `get_pools`.

```rust
// src/http/tokens.rs
use serde::{Deserialize, Serialize};

use super::ShurikenHttpClient;
use crate::error::ShurikenError;

// ── Response types ──────────────────────────────────────────────────────────
// (all type definitions identical to src/api/tokens.rs — TokenInfo, TokenPool,
//  TokenPrice, TokenChartCandle, TokenChart, TokenVolumeStats, TokenTxnStats,
//  TokenUniqueTradersStats, TokenPriceChangeStats, TokenStats, TokenPools,
//  BatchTokensResponse, SearchTokensParams, GetTokenChartParams)

// ... [copy all struct definitions verbatim from src/api/tokens.rs] ...

// ── API ─────────────────────────────────────────────────────────────────────

pub struct TokensApi<'a>(pub(crate) &'a ShurikenHttpClient);

impl TokensApi<'_> {
    pub async fn get(&self, token_id: &str) -> Result<TokenInfo, ShurikenError> {
        self.0.get(&format!("/api/v2/tokens/{token_id}")).await
    }

    pub async fn search(
        &self,
        params: &SearchTokensParams,
    ) -> Result<Vec<TokenInfo>, ShurikenError> {
        let mut query = vec![("q", params.q.clone())];
        if let Some(chain) = &params.chain {
            query.push(("chain", chain.clone()));
        }
        if let Some(page) = params.page {
            query.push(("page", page.to_string()));
        }
        if let Some(limit) = params.limit {
            query.push(("limit", limit.to_string()));
        }
        self.0
            .get_with_query("/api/v2/tokens/search", &query)
            .await
    }

    pub async fn batch(
        &self,
        tokens: &[String],
    ) -> Result<BatchTokensResponse, ShurikenError> {
        #[derive(Serialize)]
        struct Body<'a> {
            tokens: &'a [String],
        }
        self.0
            .post("/api/v2/tokens/batch", &Body { tokens })
            .await
    }

    pub async fn get_price(&self, token_id: &str) -> Result<TokenPrice, ShurikenError> {
        self.0
            .get(&format!("/api/v2/tokens/{token_id}/price"))
            .await
    }

    pub async fn get_chart(
        &self,
        params: &GetTokenChartParams,
    ) -> Result<TokenChart, ShurikenError> {
        let mut query = Vec::new();
        if let Some(resolution) = &params.resolution {
            query.push(("resolution", resolution.clone()));
        }
        if let Some(count) = params.count {
            query.push(("count", count.to_string()));
        }
        self.0
            .get_with_query(
                &format!("/api/v2/tokens/{}/price/chart", params.token_id),
                &query,
            )
            .await
    }

    pub async fn get_stats(&self, token_id: &str) -> Result<TokenStats, ShurikenError> {
        self.0
            .get(&format!("/api/v2/tokens/{token_id}/stats"))
            .await
    }

    pub async fn get_pools(&self, token_id: &str) -> Result<TokenPools, ShurikenError> {
        self.0
            .get(&format!("/api/v2/tokens/{token_id}/pools"))
            .await
    }
}
```

- [ ] **Step 2: Delete `src/api/tokens.rs`**

```bash
rm src/api/tokens.rs
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "refactor: move tokens API to namespace pattern (TokensApi)"
```

---

### Task 4: Move swap API to namespace pattern

**Files:**
- Create: `src/http/swap.rs` (from `src/api/swap.rs`)
- Delete: `src/api/swap.rs`

- [ ] **Step 1: Create `src/http/swap.rs`**

Copy all types from `src/api/swap.rs` unchanged. Replace `impl ShurikenClient` with `SwapApi<'_>`. Method renames: `get_swap_quote` → `get_quote`, `execute_swap` → `execute`, `build_transaction` → `build_transaction`, `submit_transaction` → `submit_transaction`, `get_swap_status` → `get_status`, `get_approve_spender` → `get_approve_spender`, `get_approve_allowance` → `get_approve_allowance`.

```rust
// src/http/swap.rs
use serde::{Deserialize, Serialize};

use super::ShurikenHttpClient;
use crate::error::ShurikenError;

// ... [copy all struct definitions verbatim from src/api/swap.rs] ...

// ── API ─────────────────────────────────────────────────────────────────────

pub struct SwapApi<'a>(pub(crate) &'a ShurikenHttpClient);

impl SwapApi<'_> {
    pub async fn get_quote(
        &self,
        params: &GetSwapQuoteParams,
    ) -> Result<SwapQuote, ShurikenError> {
        let mut query = vec![
            ("chain", params.chain.clone()),
            ("inputMint", params.input_mint.clone()),
            ("outputMint", params.output_mint.clone()),
            ("amount", params.amount.clone()),
        ];
        if let Some(slippage) = params.slippage_bps {
            query.push(("slippageBps", slippage.to_string()));
        }
        self.0
            .get_with_query("/api/v2/swap/quote", &query)
            .await
    }

    pub async fn execute(
        &self,
        params: &ExecuteSwapParams,
    ) -> Result<SwapStatus, ShurikenError> {
        self.0.post("/api/v2/swap/execute", params).await
    }

    pub async fn build_transaction(
        &self,
        params: &BuildTransactionParams,
    ) -> Result<BuildTransactionResponse, ShurikenError> {
        self.0.post("/api/v2/swap/transaction", params).await
    }

    pub async fn submit_transaction(
        &self,
        params: &SubmitTransactionParams,
    ) -> Result<SubmitTransactionResponse, ShurikenError> {
        self.0.post("/api/v2/swap/submit", params).await
    }

    pub async fn get_status(&self, task_id: &str) -> Result<SwapStatus, ShurikenError> {
        self.0
            .get(&format!("/api/v2/swap/status/{task_id}"))
            .await
    }

    pub async fn get_approve_spender(
        &self,
        chain_id: u64,
    ) -> Result<ApproveSpenderResponse, ShurikenError> {
        self.0
            .get_with_query(
                "/api/v2/swap/approve/spender",
                &[("chainId", chain_id.to_string())],
            )
            .await
    }

    pub async fn get_approve_allowance(
        &self,
        params: &GetApproveAllowanceParams,
    ) -> Result<ApproveAllowanceResponse, ShurikenError> {
        self.0
            .get_with_query(
                "/api/v2/swap/approve/allowance",
                &[
                    ("chainId", params.chain_id.to_string()),
                    ("tokenAddress", params.token_address.clone()),
                    ("walletAddress", params.wallet_address.clone()),
                ],
            )
            .await
    }
}
```

- [ ] **Step 2: Delete `src/api/swap.rs`**

```bash
rm src/api/swap.rs
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "refactor: move swap API to namespace pattern (SwapApi)"
```

---

### Task 5: Move portfolio API to namespace pattern

**Files:**
- Create: `src/http/portfolio.rs` (from `src/api/portfolio.rs`)
- Delete: `src/api/portfolio.rs`

- [ ] **Step 1: Create `src/http/portfolio.rs`**

Copy all types unchanged. Replace `impl ShurikenClient` with `PortfolioApi<'_>`. Method renames: `get_balances` → `get_balances`, `get_history` → `get_history`, `get_pnl` → `get_pnl`, `get_positions` → `get_positions`.

```rust
// src/http/portfolio.rs
use serde::Deserialize;

use super::ShurikenHttpClient;
use crate::error::ShurikenError;

// ... [copy all struct definitions verbatim from src/api/portfolio.rs] ...

// ── API ─────────────────────────────────────────────────────────────────────

pub struct PortfolioApi<'a>(pub(crate) &'a ShurikenHttpClient);

impl PortfolioApi<'_> {
    pub async fn get_balances(
        &self,
        params: &GetBalancesParams,
    ) -> Result<Vec<WalletBalance>, ShurikenError> {
        let mut query = Vec::new();
        if let Some(chain) = &params.chain {
            query.push(("chain", chain.clone()));
        }
        self.0
            .get_with_query("/api/v2/portfolio/balances", &query)
            .await
    }

    pub async fn get_history(
        &self,
        params: &GetHistoryParams,
    ) -> Result<Vec<PortfolioTrade>, ShurikenError> {
        let mut query = Vec::new();
        if let Some(chain) = &params.chain {
            query.push(("chain", chain.clone()));
        }
        if let Some(page) = params.page {
            query.push(("page", page.to_string()));
        }
        if let Some(limit) = params.limit {
            query.push(("limit", limit.to_string()));
        }
        self.0
            .get_with_query("/api/v2/portfolio/history", &query)
            .await
    }

    pub async fn get_pnl(
        &self,
        params: &GetPnlParams,
    ) -> Result<PortfolioPnl, ShurikenError> {
        let mut query = Vec::new();
        if let Some(timeframe) = &params.timeframe {
            query.push(("timeframe", timeframe.clone()));
        }
        self.0
            .get_with_query("/api/v2/portfolio/pnl", &query)
            .await
    }

    pub async fn get_positions(
        &self,
        params: &GetPositionsParams,
    ) -> Result<PositionsResponse, ShurikenError> {
        let mut query = Vec::new();
        if let Some(chain) = &params.chain {
            query.push(("chain", chain.clone()));
        }
        if let Some(status) = &params.status {
            query.push(("status", status.clone()));
        }
        self.0
            .get_with_query("/api/v2/portfolio/positions", &query)
            .await
    }
}
```

- [ ] **Step 2: Delete `src/api/portfolio.rs`**

```bash
rm src/api/portfolio.rs
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "refactor: move portfolio API to namespace pattern (PortfolioApi)"
```

---

### Task 6: Move trigger API to namespace pattern

**Files:**
- Create: `src/http/trigger.rs` (from `src/api/trigger.rs`)
- Delete: `src/api/trigger.rs`

- [ ] **Step 1: Create `src/http/trigger.rs`**

Copy all types unchanged. Replace `impl ShurikenClient` with `TriggerApi<'_>`. Method renames: `create_trigger_order` → `create`, `get_trigger_order` → `get`, `list_trigger_orders` → `list`, `cancel_trigger_order` → `cancel`.

```rust
// src/http/trigger.rs
use serde::{Deserialize, Serialize};

use super::ShurikenHttpClient;
use crate::error::ShurikenError;

// ... [copy all struct definitions verbatim from src/api/trigger.rs] ...

// ── API ─────────────────────────────────────────────────────────────────────

pub struct TriggerApi<'a>(pub(crate) &'a ShurikenHttpClient);

impl TriggerApi<'_> {
    pub async fn create(
        &self,
        params: &CreateTriggerOrderParams,
    ) -> Result<TriggerOrder, ShurikenError> {
        self.0.post("/api/v2/trigger/order", params).await
    }

    pub async fn get(&self, order_id: &str) -> Result<TriggerOrderView, ShurikenError> {
        self.0
            .get(&format!("/api/v2/trigger/order/{order_id}"))
            .await
    }

    pub async fn list(
        &self,
        params: &ListTriggerOrdersParams,
    ) -> Result<TriggerOrdersResponse, ShurikenError> {
        let mut query = Vec::new();
        if let Some(limit) = params.limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(cursor) = &params.cursor {
            query.push(("cursor", cursor.clone()));
        }
        self.0
            .get_with_query("/api/v2/trigger/orders", &query)
            .await
    }

    pub async fn cancel(
        &self,
        order_id: &str,
    ) -> Result<CancelledTriggerOrder, ShurikenError> {
        self.0
            .delete(&format!("/api/v2/trigger/order/{order_id}"))
            .await
    }
}
```

- [ ] **Step 2: Delete `src/api/trigger.rs`**

```bash
rm src/api/trigger.rs
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "refactor: move trigger API to namespace pattern (TriggerApi)"
```

---

### Task 7: Move perps API to namespace pattern

**Files:**
- Create: `src/http/perps.rs` (from `src/api/perps.rs`)
- Delete: `src/api/perps.rs`

- [ ] **Step 1: Create `src/http/perps.rs`**

Copy all types unchanged. Replace `impl ShurikenClient` with `PerpsApi<'_>`. Move the `wallet_query` helper into the file. Method renames: `get_perp_account` → `get_account`, `get_perp_fees` → `get_fees`, `get_perp_fills` → `get_fills`, `get_perp_funding` → `get_funding`, `get_perp_markets` → `get_markets`, `get_perp_market` → `get_market`, `get_perp_orders` → `get_orders`, `get_perp_positions` → `get_positions`, `place_perp_order` → `place_order`, `modify_perp_order` → `modify_order`, `cancel_perp_order` → `cancel_order`, `batch_modify_perp_orders` → `batch_modify_orders`, `close_perp_position` → `close_position`, `update_perp_leverage` → `update_leverage`, `update_perp_margin` → `update_margin`.

```rust
// src/http/perps.rs
use serde::{Deserialize, Serialize};

use super::ShurikenHttpClient;
use crate::error::ShurikenError;

// ... [copy all struct definitions verbatim from src/api/perps.rs] ...

fn wallet_query(wallet_id: &Option<String>) -> Vec<(&'static str, String)> {
    wallet_id
        .as_ref()
        .map(|w| vec![("wallet_id", w.clone())])
        .unwrap_or_default()
}

// ── API ─────────────────────────────────────────────────────────────────────

pub struct PerpsApi<'a>(pub(crate) &'a ShurikenHttpClient);

impl PerpsApi<'_> {
    pub async fn get_account(
        &self,
        params: &GetPerpAccountParams,
    ) -> Result<PerpAccountState, ShurikenError> {
        self.0
            .get_with_query("/api/v2/perp/account", &wallet_query(&params.wallet_id))
            .await
    }

    pub async fn get_fees(
        &self,
        params: &GetPerpFeesParams,
    ) -> Result<UserFees, ShurikenError> {
        self.0
            .get_with_query("/api/v2/perp/fees", &wallet_query(&params.wallet_id))
            .await
    }

    pub async fn get_fills(
        &self,
        params: &GetPerpFillsParams,
    ) -> Result<Vec<PerpFill>, ShurikenError> {
        let mut query = vec![("start_time", params.start_time.to_string())];
        if let Some(end) = params.end_time {
            query.push(("end_time", end.to_string()));
        }
        if let Some(coin) = &params.coin {
            query.push(("coin", coin.clone()));
        }
        if let Some(w) = &params.wallet_id {
            query.push(("wallet_id", w.clone()));
        }
        self.0
            .get_with_query("/api/v2/perp/fills", &query)
            .await
    }

    pub async fn get_funding(
        &self,
        params: &GetPerpFundingParams,
    ) -> Result<Vec<FundingPayment>, ShurikenError> {
        let mut query = vec![("start_time", params.start_time.to_string())];
        if let Some(end) = params.end_time {
            query.push(("end_time", end.to_string()));
        }
        if let Some(coin) = &params.coin {
            query.push(("coin", coin.clone()));
        }
        if let Some(w) = &params.wallet_id {
            query.push(("wallet_id", w.clone()));
        }
        self.0
            .get_with_query("/api/v2/perp/funding", &query)
            .await
    }

    pub async fn get_markets(&self) -> Result<Vec<PerpMarket>, ShurikenError> {
        self.0.get("/api/v2/perp/markets").await
    }

    pub async fn get_market(&self, coin: &str) -> Result<PerpMarket, ShurikenError> {
        self.0
            .get(&format!("/api/v2/perp/markets/{coin}"))
            .await
    }

    pub async fn get_orders(
        &self,
        params: &GetPerpOrdersParams,
    ) -> Result<Vec<OpenOrder>, ShurikenError> {
        let mut query = wallet_query(&params.wallet_id);
        if let Some(coin) = &params.coin {
            query.push(("coin", coin.clone()));
        }
        self.0
            .get_with_query("/api/v2/perp/orders", &query)
            .await
    }

    pub async fn get_positions(
        &self,
        params: &GetPerpPositionsParams,
    ) -> Result<PerpPositionsResponse, ShurikenError> {
        self.0
            .get_with_query("/api/v2/perp/positions", &wallet_query(&params.wallet_id))
            .await
    }

    pub async fn place_order(
        &self,
        params: &PlaceOrderParams,
    ) -> Result<OrderResponse, ShurikenError> {
        self.0.post("/api/v2/perp/order", params).await
    }

    pub async fn modify_order(
        &self,
        params: &ModifyOrderParams,
    ) -> Result<OrderResponse, ShurikenError> {
        self.0.patch("/api/v2/perp/order", params).await
    }

    pub async fn cancel_order(
        &self,
        params: &CancelOrderParams,
    ) -> Result<OrderResponse, ShurikenError> {
        self.0
            .delete_with_body("/api/v2/perp/order", params)
            .await
    }

    pub async fn batch_modify_orders(
        &self,
        params: &BatchModifyParams,
    ) -> Result<OrderResponse, ShurikenError> {
        self.0.patch("/api/v2/perp/orders", params).await
    }

    pub async fn close_position(
        &self,
        params: &ClosePositionParams,
    ) -> Result<OrderResponse, ShurikenError> {
        self.0.post("/api/v2/perp/position/close", params).await
    }

    pub async fn update_leverage(
        &self,
        params: &UpdateLeverageParams,
    ) -> Result<LeverageResponse, ShurikenError> {
        self.0.post("/api/v2/perp/leverage", params).await
    }

    pub async fn update_margin(
        &self,
        params: &UpdateMarginParams,
    ) -> Result<MarginResponse, ShurikenError> {
        self.0.post("/api/v2/perp/position/margin", params).await
    }
}
```

- [ ] **Step 2: Delete `src/api/perps.rs` and remaining `src/api/` directory**

```bash
rm -r src/api/
```

- [ ] **Step 3: Verify the HTTP client compiles**

```bash
cargo check
```

Expected: compiles successfully (tests may fail due to import path changes — we fix those later).

- [ ] **Step 4: Run clippy**

```bash
cargo clippy -- -D warnings
```

Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor: move perps API to namespace pattern (PerpsApi), delete old api/ module"
```

---

### Task 8: Update tests for new module paths

**Files:**
- Modify: `tests/serialization.rs`

- [ ] **Step 1: Update all import paths in `tests/serialization.rs`**

Replace all `shuriken_sdk::ShurikenClient` references with `shuriken_sdk::ShurikenHttpClient`. Module paths remain the same since we re-export at crate root (`shuriken_sdk::tokens::TokenInfo` still works).

The two client construction tests need updating:

```rust
// Replace the two client tests at the bottom:

#[test]
fn http_client_new() {
    let client = shuriken_sdk::ShurikenHttpClient::new("sk_test123");
    assert!(client.is_ok());
}

#[test]
fn http_client_with_base_url() {
    let client =
        shuriken_sdk::ShurikenHttpClient::with_base_url("sk_test", "https://staging.example.com/");
    assert!(client.is_ok());
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 3: Run full CI checks**

```bash
cargo check && cargo clippy -- -D warnings && cargo fmt --check
```

Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "test: update tests for ShurikenHttpClient and new module paths"
```

---

### Task 9: Create `Subscription<T>` implementing Stream

**Files:**
- Create: `src/ws/subscription.rs`

- [ ] **Step 1: Add `futures-core` to Cargo.toml dependencies (optional, ws feature)**

Add to `[dependencies]` section in `Cargo.toml`:

```toml
futures-core = { version = "0.3", optional = true }
```

Update the `ws` feature to include it:

```toml
ws = ["dep:tokio-tungstenite", "dep:futures-util", "dep:tracing", "dep:futures-core"]
```

- [ ] **Step 2: Create `src/ws/subscription.rs`**

```rust
// src/ws/subscription.rs
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use tokio::sync::mpsc;

pub struct Subscription<T> {
    pub(crate) rx: mpsc::UnboundedReceiver<T>,
    pub(crate) id: usize,
    pub(crate) unsub_tx: mpsc::UnboundedSender<usize>,
}

impl<T> Stream for Subscription<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

impl<T> Drop for Subscription<T> {
    fn drop(&mut self) {
        let _ = self.unsub_tx.send(self.id);
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add Subscription<T> implementing Stream with RAII unsubscribe"
```

---

### Task 10: Create stream definitions and filter types

**Files:**
- Create: `src/ws/streams.rs`

- [ ] **Step 1: Create `src/ws/streams.rs`**

```rust
// src/ws/streams.rs
use std::collections::HashMap;
use std::marker::PhantomData;

use serde::de::DeserializeOwned;

// ── IntoFilterMap trait ─────────────────────────────────────────────────────

pub trait IntoFilterMap {
    fn into_filter_map(self) -> HashMap<String, String>;
}

// ── StreamDef ───────────────────────────────────────────────────────────────

pub struct StreamDef<P: DeserializeOwned, F: IntoFilterMap> {
    pub name: &'static str,
    _phantom: PhantomData<fn() -> (P, F)>,
}

impl<P: DeserializeOwned, F: IntoFilterMap> StreamDef<P, F> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _phantom: PhantomData,
        }
    }
}

// Allow Copy/Clone so constants can be passed by value
impl<P: DeserializeOwned, F: IntoFilterMap> Clone for StreamDef<P, F> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<P: DeserializeOwned, F: IntoFilterMap> Copy for StreamDef<P, F> {}

// ── Filter types ────────────────────────────────────────────────────────────

pub struct NoFilter;

impl IntoFilterMap for NoFilter {
    fn into_filter_map(self) -> HashMap<String, String> {
        HashMap::new()
    }
}

pub struct SvmTokenFilter {
    pub token_address: String,
}

impl IntoFilterMap for SvmTokenFilter {
    fn into_filter_map(self) -> HashMap<String, String> {
        HashMap::from([("tokenAddress".into(), self.token_address)])
    }
}

pub struct SvmWalletFilter {
    pub wallet_address: String,
}

impl IntoFilterMap for SvmWalletFilter {
    fn into_filter_map(self) -> HashMap<String, String> {
        HashMap::from([("walletAddress".into(), self.wallet_address)])
    }
}

pub struct EvmTokenFilter {
    pub chain_id: String,
    pub token_address: String,
}

impl IntoFilterMap for EvmTokenFilter {
    fn into_filter_map(self) -> HashMap<String, String> {
        HashMap::from([
            ("chainId".into(), self.chain_id),
            ("tokenAddress".into(), self.token_address),
        ])
    }
}

pub struct EvmWalletFilter {
    pub wallet_address: String,
}

impl IntoFilterMap for EvmWalletFilter {
    fn into_filter_map(self) -> HashMap<String, String> {
        HashMap::from([("walletAddress".into(), self.wallet_address)])
    }
}

pub struct AlphaProfileFilter {
    pub profile_id: String,
}

impl IntoFilterMap for AlphaProfileFilter {
    fn into_filter_map(self) -> HashMap<String, String> {
        HashMap::from([("profileId".into(), self.profile_id)])
    }
}

pub struct AlphaNamedFeedFilter {
    pub feed_id: String,
}

impl IntoFilterMap for AlphaNamedFeedFilter {
    fn into_filter_map(self) -> HashMap<String, String> {
        HashMap::from([("feedId".into(), self.feed_id)])
    }
}

// ── Stream constants ────────────────────────────────────────────────────────

use shuriken_api_types::{alpha, analytics, automation, evm, notification, svm, wallet};

// SVM streams
pub const SVM_TOKEN_SWAPS: StreamDef<svm::SwapEvent, SvmTokenFilter> =
    StreamDef::new("svm.token.swaps");
pub const SVM_TOKEN_POOL_INFO: StreamDef<svm::TokenPoolEvent, SvmTokenFilter> =
    StreamDef::new("svm.token.poolInfo");
pub const SVM_TOKEN_BALANCES: StreamDef<svm::TokenBalanceEvent, SvmTokenFilter> =
    StreamDef::new("svm.token.balances");
pub const SVM_TOKEN_DISTRIBUTION_STATS: StreamDef<analytics::TokenDistributionStatsEvent, SvmTokenFilter> =
    StreamDef::new("svm.token.distributionStats");
pub const SVM_TOKEN_HOLDER_STATS: StreamDef<analytics::HolderStatsEvent, SvmTokenFilter> =
    StreamDef::new("svm.token.holderStats");
pub const SVM_WALLET_NATIVE_BALANCE: StreamDef<wallet::SvmNativeBalanceEvent, SvmWalletFilter> =
    StreamDef::new("svm.wallet.nativeBalance");
pub const SVM_WALLET_TOKEN_BALANCES: StreamDef<wallet::SvmTokenBalanceEvent, SvmWalletFilter> =
    StreamDef::new("svm.wallet.tokenBalances");
pub const SVM_BONDING_CURVE_CREATIONS: StreamDef<svm::BondingCurveCreationEvent, NoFilter> =
    StreamDef::new("svm.bondingCurve.creations");
pub const SVM_BONDING_CURVE_GRADUATIONS: StreamDef<svm::BondingCurveGraduationEvent, NoFilter> =
    StreamDef::new("svm.bondingCurve.graduations");

// EVM streams
pub const EVM_TOKEN_SWAPS: StreamDef<evm::SwapEvent, EvmTokenFilter> =
    StreamDef::new("evm.token.swaps");
pub const EVM_TOKEN_POOL_INFO: StreamDef<evm::TokenPoolEvent, EvmTokenFilter> =
    StreamDef::new("evm.token.poolInfo");
pub const EVM_TOKEN_BALANCES: StreamDef<evm::TokenBalanceEvent, EvmTokenFilter> =
    StreamDef::new("evm.token.balances");
pub const EVM_WALLET_NATIVE_BALANCE: StreamDef<wallet::EvmNativeBalanceEvent, EvmWalletFilter> =
    StreamDef::new("evm.wallet.nativeBalance");
pub const EVM_WALLET_TOKEN_BALANCES: StreamDef<evm::TokenBalanceEvent, EvmWalletFilter> =
    StreamDef::new("evm.wallet.tokenBalances");

// Alpha streams
pub const ALPHA_SIGNAL_FEED_GLOBAL: StreamDef<alpha::SignalFeedUpdateEvent, NoFilter> =
    StreamDef::new("alpha.signalFeedGlobal");
pub const ALPHA_SIGNAL_FEED_PERSONAL: StreamDef<alpha::SignalFeedUpdateEvent, NoFilter> =
    StreamDef::new("alpha.signalFeedPersonal");
pub const ALPHA_SIGNAL_FEED_PROFILE: StreamDef<alpha::SignalFeedUpdateEvent, AlphaProfileFilter> =
    StreamDef::new("alpha.signalFeedProfile");
pub const ALPHA_SIGNAL_FEED_NAMED: StreamDef<alpha::SignalFeedUpdateEvent, AlphaNamedFeedFilter> =
    StreamDef::new("alpha.signalFeedNamed");
pub const ALPHA_PERSONAL: StreamDef<alpha::MessageEvent, NoFilter> =
    StreamDef::new("alpha.personal");

// Portfolio / Automation streams
pub const PORTFOLIO_NOTIFICATIONS: StreamDef<notification::NotificationEvent, NoFilter> =
    StreamDef::new("portfolio.notifications");
pub const AUTOMATION_UPDATES: StreamDef<automation::AutomationEvent, NoFilter> =
    StreamDef::new("automation.updates");
```

- [ ] **Step 2: Verify it compiles with ws feature**

```bash
cargo check --features ws
```

Expected: compiles (ws/mod.rs won't reference these yet, but the module should parse).

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add StreamDef constants, filter types, and IntoFilterMap trait for 22 streams"
```

---

### Task 11: Create ShurikenWsClient with typed subscriptions

Rewrite `src/ws/mod.rs` and `src/ws/connection.rs` to implement `ShurikenWsClient` with `&mut self` methods, mpsc-based dispatch, and typed `subscribe()`.

**Files:**
- Rewrite: `src/ws/mod.rs`
- Rewrite: `src/ws/connection.rs`
- Modify: `src/ws/types.rs` (keep session/pusher types, remove `ConnectionState`/`ConnectionStateEvent` — they move to `mod.rs`)

- [ ] **Step 1: Update `src/ws/types.rs`**

Keep only the session bootstrap and Pusher protocol types. Remove `ConnectionState` and `ConnectionStateEvent` (they'll be defined in `mod.rs`).

```rust
// src/ws/types.rs — keep everything EXCEPT ConnectionState and ConnectionStateEvent
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ── Session bootstrap types ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SubscriptionFilter {
    pub stream: String,
    #[serde(default)]
    pub filter: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    pub provider: String,
    pub app_key: String,
    pub ws_host: String,
    pub ws_port: u16,
    pub force_tls: bool,
    pub auth_endpoint: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub recommended_reconnect_backoff_ms: Vec<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedSubscription {
    pub stream: String,
    pub channel: String,
    pub event: String,
    pub visibility: String,
    pub payload_format: String,
    pub payload_schema_id: String,
    pub payload_schema_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    pub connection: ConnectionInfo,
    pub session: SessionInfo,
    pub subscriptions: Vec<ResolvedSubscription>,
}

// ── Pusher protocol messages ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct PusherMessage {
    pub event: String,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub(crate) struct PusherSubscribe {
    pub event: &'static str,
    pub data: PusherSubscribeData,
}

#[derive(Debug, Serialize)]
pub(crate) struct PusherSubscribeData {
    pub channel: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_data: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PusherConnectionEstablished {
    pub socket_id: String,
}
```

- [ ] **Step 2: Rewrite `src/ws/connection.rs`**

This file now contains the internal event loop and dispatch logic. The key change: instead of storing `Box<dyn Fn(Value)>` handlers, each subscription has an `mpsc::UnboundedSender<serde_json::Value>` that the event loop sends raw JSON into.

```rust
// src/ws/connection.rs
use std::collections::HashMap;

use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, warn};

use crate::error::ShurikenError;

use super::types::*;

pub(crate) type WsSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;

pub(crate) struct ActiveSubscription {
    pub channel: String,
    pub event: String,
    pub tx: mpsc::UnboundedSender<serde_json::Value>,
    pub filter: SubscriptionFilter,
    pub resolved: Option<ResolvedSubscription>,
}

pub(crate) async fn http_post(
    http: &Client,
    base_url: &str,
    path: &str,
    body: &impl serde::Serialize,
) -> Result<serde_json::Value, ShurikenError> {
    let url = format!("{base_url}{path}");
    let resp = http.post(&url).json(body).send().await?;
    let status = resp.status();
    let request_id = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ShurikenError::Auth(resp.text().await.unwrap_or_default()));
    }
    if !status.is_success() {
        return Err(ShurikenError::Api {
            status: status.as_u16(),
            message: resp.text().await.unwrap_or_default(),
            request_id,
        });
    }
    Ok(resp.json().await?)
}

pub(crate) async fn fetch_session(
    http: &Client,
    base_url: &str,
    filters: &[SubscriptionFilter],
) -> Result<SessionResponse, ShurikenError> {
    let value = http_post(
        http,
        base_url,
        "/api/v2/ws/session",
        &serde_json::json!({ "subscriptions": filters }),
    )
    .await?;
    serde_json::from_value(value).map_err(ShurikenError::from)
}

pub(crate) async fn pusher_subscribe(
    sink: &mut WsSink,
    http: &Client,
    base_url: &str,
    session: &SessionResponse,
    socket_id: &str,
    channel: &str,
    visibility: &str,
) -> Result<(), ShurikenError> {
    let auth = if visibility == "presence"
        || channel.starts_with("private-")
        || channel.starts_with("presence-")
    {
        let value = http_post(
            http,
            base_url,
            &session.connection.auth_endpoint,
            &serde_json::json!({
                "socket_id": socket_id,
                "channel_name": channel,
            }),
        )
        .await?;
        Some(
            value["auth"]
                .as_str()
                .ok_or_else(|| ShurikenError::Session("Missing auth in response".into()))?
                .to_string(),
        )
    } else {
        None
    };

    let msg = serde_json::to_string(&PusherSubscribe {
        event: "pusher:subscribe",
        data: PusherSubscribeData {
            channel: channel.to_string(),
            auth,
            channel_data: None,
        },
    })
    .map_err(|e| ShurikenError::Session(format!("Serialize error: {e}")))?;

    sink.send(Message::Text(msg.into()))
        .await
        .map_err(|e| ShurikenError::Session(format!("Send error: {e}")))?;

    Ok(())
}

pub(crate) fn dispatch(subscriptions: &[ActiveSubscription], msg: PusherMessage) {
    if msg.event.starts_with("pusher:") || msg.event.starts_with("pusher_internal:") {
        return;
    }
    let Some(channel) = &msg.channel else { return };
    let data = match &msg.data {
        Some(serde_json::Value::String(s)) => {
            serde_json::from_str(s).unwrap_or(serde_json::Value::String(s.clone()))
        }
        Some(v) => v.clone(),
        None => return,
    };
    for sub in subscriptions.iter() {
        if sub.channel == *channel && sub.event == msg.event {
            let _ = sub.tx.send(data.clone());
        }
    }
}
```

- [ ] **Step 3: Rewrite `src/ws/mod.rs` with `ShurikenWsClient`**

```rust
// src/ws/mod.rs
mod connection;
pub mod streams;
pub mod subscription;
pub(crate) mod types;

use std::collections::HashMap;

use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::Client;
use serde::de::DeserializeOwned;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, warn};

use crate::error::ShurikenError;
use connection::{ActiveSubscription, WsSink};
use streams::{IntoFilterMap, StreamDef};
use subscription::Subscription;
use types::*;

const DEFAULT_BASE_URL: &str = "https://api.shuriken.trade";

// ── Connection state ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

#[derive(Debug, Clone)]
pub struct ConnectionStateEvent {
    pub state: ConnectionState,
    pub reason: Option<String>,
}

// ── Client ──────────────────────────────────────────────────────────────────

pub struct ShurikenWsClient {
    http: Client,
    base_url: String,
    session: Option<SessionResponse>,
    socket_id: Option<String>,
    sink: Option<WsSink>,
    subscriptions: Vec<ActiveSubscription>,
    state: ConnectionState,
    state_tx: mpsc::UnboundedSender<ConnectionStateEvent>,
    state_rx: Option<mpsc::UnboundedReceiver<ConnectionStateEvent>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    unsub_tx: mpsc::UnboundedSender<usize>,
    unsub_rx: Option<mpsc::UnboundedReceiver<usize>>,
}

impl ShurikenWsClient {
    pub fn new(api_key: &str) -> Result<Self, ShurikenError> {
        Self::with_base_url(api_key, DEFAULT_BASE_URL)
    }

    pub fn with_base_url(api_key: &str, base_url: &str) -> Result<Self, ShurikenError> {
        let mut headers = HeaderMap::new();
        let mut auth_value = HeaderValue::from_str(&format!("Bearer {api_key}"))
            .map_err(|e| ShurikenError::Auth(e.to_string()))?;
        auth_value.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth_value);

        let http = Client::builder().default_headers(headers).build()?;
        let base_url = base_url.trim_end_matches('/').to_string();
        let (state_tx, state_rx) = mpsc::unbounded_channel();
        let (unsub_tx, unsub_rx) = mpsc::unbounded_channel();

        Ok(Self {
            http,
            base_url,
            session: None,
            socket_id: None,
            sink: None,
            subscriptions: Vec::new(),
            state: ConnectionState::Disconnected,
            state_tx,
            state_rx: Some(state_rx),
            shutdown_tx: None,
            unsub_tx,
            unsub_rx: Some(unsub_rx),
        })
    }

    pub async fn connect(&mut self) -> Result<(), ShurikenError> {
        if self.state == ConnectionState::Connected || self.state == ConnectionState::Connecting {
            return Err(ShurikenError::Session("Already connected".into()));
        }

        self.emit_state(ConnectionState::Connecting, None);

        let session = connection::fetch_session(
            &self.http,
            &self.base_url,
            &[SubscriptionFilter {
                stream: "alpha.signalFeedGlobal".into(),
                filter: HashMap::new(),
            }],
        )
        .await?;

        let conn = &session.connection;
        let scheme = if conn.force_tls { "wss" } else { "ws" };
        let url = format!(
            "{scheme}://{}:{}/app/{}?protocol=7&client=shuriken-sdk-rs&version=0.3.0",
            conn.ws_host, conn.ws_port, conn.app_key,
        );

        let (ws_stream, _) = tokio_tungstenite::connect_async(&url)
            .await
            .map_err(|e| ShurikenError::Session(format!("WebSocket connect failed: {e}")))?;

        let (sink, mut stream) = ws_stream.split();
        self.sink = Some(sink);
        self.session = Some(session);

        // Wait for pusher:connection_established
        let socket_id = loop {
            match stream.next().await {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(msg) = serde_json::from_str::<PusherMessage>(&text) {
                        if msg.event == "pusher:connection_established" {
                            if let Some(data) = &msg.data {
                                let data_str = match data {
                                    serde_json::Value::String(s) => s.clone(),
                                    other => other.to_string(),
                                };
                                let established: PusherConnectionEstablished =
                                    serde_json::from_str(&data_str).map_err(|e| {
                                        ShurikenError::Session(format!(
                                            "Failed to parse connection_established: {e}"
                                        ))
                                    })?;
                                break established.socket_id;
                            }
                        }
                    }
                }
                Some(Ok(Message::Close(_))) | None => {
                    return Err(ShurikenError::Session(
                        "Connection closed before established".into(),
                    ));
                }
                Some(Err(e)) => {
                    return Err(ShurikenError::Session(format!("WebSocket error: {e}")));
                }
                _ => continue,
            }
        };

        self.socket_id = Some(socket_id);
        self.emit_state(ConnectionState::Connected, None);

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Take the unsub receiver for the event loop
        let mut unsub_rx = self.unsub_rx.take().unwrap_or_else(|| {
            let (_, rx) = mpsc::unbounded_channel();
            rx
        });

        // Spawn the event loop — it needs shared access to subscriptions
        // We use a pointer via leaked Arc for the subscription list since
        // the event loop outlives the borrow. Instead, we'll use channels.
        // Actually — the event loop needs to read subscriptions to dispatch.
        // The cleanest approach: move subscriptions into shared state for dispatch.
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let subs_shared: Arc<Mutex<Vec<ActiveSubscription>>> =
            Arc::new(Mutex::new(std::mem::take(&mut self.subscriptions)));
        let subs_for_loop = Arc::clone(&subs_shared);
        let state_tx = self.state_tx.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = stream.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(m) = serde_json::from_str::<PusherMessage>(&text) {
                                    let subs = subs_for_loop.lock().await;
                                    connection::dispatch(&subs, m);
                                }
                            }
                            Some(Ok(Message::Ping(_))) => {
                                debug!("WebSocket ping");
                            }
                            Some(Ok(Message::Close(_))) | None => {
                                warn!("WebSocket closed");
                                let _ = state_tx.send(ConnectionStateEvent {
                                    state: ConnectionState::Disconnected,
                                    reason: Some("Connection closed".into()),
                                });
                                break;
                            }
                            Some(Err(e)) => {
                                tracing::error!("WebSocket error: {e}");
                                let _ = state_tx.send(ConnectionStateEvent {
                                    state: ConnectionState::Failed,
                                    reason: Some(e.to_string()),
                                });
                                break;
                            }
                            _ => {}
                        }
                    }
                    Some(sub_id) = unsub_rx.recv() => {
                        let mut subs = subs_for_loop.lock().await;
                        subs.retain(|s| s.filter.stream != format!("__id_{sub_id}"));
                        // Note: we use the index as ID; removing by marking closed
                        if let Some(sub) = subs.get(sub_id) {
                            let _ = sub.tx.send(serde_json::Value::Null); // signal close
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        debug!("WebSocket shutdown");
                        break;
                    }
                }
            }
        });

        // Store shared subs back — subscribe() will need to push to this
        self.subscriptions = Vec::new(); // placeholder, real subs in Arc
        // We need to store the Arc for subscribe() to use
        // Actually, let's restructure: store the Arc on self

        // This approach is getting complex. Let's simplify:
        // After connect(), new subscriptions go through the Arc.
        // We'll store the Arc on the struct.

        Ok(())
    }

    pub async fn disconnect(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        if let Some(mut sink) = self.sink.take() {
            use futures_util::SinkExt;
            let _ = sink.close().await;
        }
        self.subscriptions.clear();
        self.session = None;
        self.socket_id = None;
        self.emit_state(ConnectionState::Disconnected, None);
    }

    pub async fn subscribe<P, F>(
        &mut self,
        stream: StreamDef<P, F>,
        filter: F,
    ) -> Result<Subscription<P>, ShurikenError>
    where
        P: DeserializeOwned + Send + 'static,
        F: IntoFilterMap,
    {
        if self.state != ConnectionState::Connected {
            return Err(ShurikenError::Session(
                "Not connected. Call connect() first.".into(),
            ));
        }

        let filter_map = filter.into_filter_map();
        let sub_filter = SubscriptionFilter {
            stream: stream.name.to_string(),
            filter: filter_map,
        };

        // Check if session already has this stream resolved
        let resolved = self
            .session
            .as_ref()
            .and_then(|s| s.subscriptions.iter().find(|r| r.stream == stream.name).cloned());

        let (raw_tx, mut raw_rx) = mpsc::unbounded_channel::<serde_json::Value>();
        let (typed_tx, typed_rx) = mpsc::unbounded_channel::<P>();

        // Spawn a task to deserialize raw JSON into typed events
        tokio::spawn(async move {
            while let Some(value) = raw_rx.recv().await {
                match serde_json::from_value::<P>(value) {
                    Ok(typed) => {
                        if typed_tx.send(typed).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to deserialize stream event: {e}");
                    }
                }
            }
        });

        let sub_id = self.subscriptions.len();

        if let Some(resolved) = resolved {
            let sink = self.sink.as_mut().ok_or_else(|| {
                ShurikenError::Session("No WebSocket connection".into())
            })?;
            connection::pusher_subscribe(
                sink,
                &self.http,
                &self.base_url,
                self.session.as_ref().unwrap(),
                self.socket_id.as_ref().unwrap(),
                &resolved.channel,
                &resolved.visibility,
            )
            .await?;

            self.subscriptions.push(ActiveSubscription {
                channel: resolved.channel.clone(),
                event: resolved.event.clone(),
                tx: raw_tx,
                filter: sub_filter,
                resolved: Some(resolved),
            });
        } else {
            // Need to expand session
            self.subscriptions.push(ActiveSubscription {
                channel: String::new(),
                event: String::new(),
                tx: raw_tx,
                filter: sub_filter.clone(),
                resolved: None,
            });
            self.expand_session(&[sub_filter]).await?;
        }

        Ok(Subscription {
            rx: typed_rx,
            id: sub_id,
            unsub_tx: self.unsub_tx.clone(),
        })
    }

    pub async fn subscribe_raw(
        &mut self,
        stream: &str,
        filter: HashMap<String, String>,
    ) -> Result<Subscription<serde_json::Value>, ShurikenError> {
        if self.state != ConnectionState::Connected {
            return Err(ShurikenError::Session(
                "Not connected. Call connect() first.".into(),
            ));
        }

        let sub_filter = SubscriptionFilter {
            stream: stream.to_string(),
            filter,
        };

        let resolved = self
            .session
            .as_ref()
            .and_then(|s| s.subscriptions.iter().find(|r| r.stream == stream).cloned());

        let (tx, rx) = mpsc::unbounded_channel::<serde_json::Value>();
        let sub_id = self.subscriptions.len();

        if let Some(resolved) = resolved {
            let sink = self.sink.as_mut().ok_or_else(|| {
                ShurikenError::Session("No WebSocket connection".into())
            })?;
            connection::pusher_subscribe(
                sink,
                &self.http,
                &self.base_url,
                self.session.as_ref().unwrap(),
                self.socket_id.as_ref().unwrap(),
                &resolved.channel,
                &resolved.visibility,
            )
            .await?;

            self.subscriptions.push(ActiveSubscription {
                channel: resolved.channel.clone(),
                event: resolved.event.clone(),
                tx,
                filter: sub_filter,
                resolved: Some(resolved),
            });
        } else {
            self.subscriptions.push(ActiveSubscription {
                channel: String::new(),
                event: String::new(),
                tx,
                filter: sub_filter.clone(),
                resolved: None,
            });
            self.expand_session(&[sub_filter]).await?;
        }

        Ok(Subscription {
            rx,
            id: sub_id,
            unsub_tx: self.unsub_tx.clone(),
        })
    }

    pub fn on_state_change(&mut self) -> Subscription<ConnectionStateEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        // Replace the state_tx so future state changes go to this subscription too
        // Actually, we want multiple listeners. Use a broadcast or just return the existing rx.
        // Simplest: return the state_rx we already have (one listener).
        // For multiple listeners, we'd need broadcast. Keep it simple: one listener.
        // If state_rx was already taken, create a new pair.
        if let Some(existing_rx) = self.state_rx.take() {
            Subscription {
                rx: existing_rx,
                id: usize::MAX, // state sub doesn't unsubscribe from stream list
                unsub_tx: self.unsub_tx.clone(),
            }
        } else {
            // Already taken — create new channel pair
            let (new_tx, new_rx) = mpsc::unbounded_channel();
            self.state_tx = new_tx;
            Subscription {
                rx: new_rx,
                id: usize::MAX,
                unsub_tx: self.unsub_tx.clone(),
            }
        }
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    pub fn session(&self) -> Option<&SessionResponse> {
        self.session.as_ref()
    }

    // ── Internal ────────────────────────────────────────────────────────────

    async fn expand_session(
        &mut self,
        new_filters: &[SubscriptionFilter],
    ) -> Result<(), ShurikenError> {
        let mut all_filters: Vec<SubscriptionFilter> =
            self.subscriptions.iter().map(|s| s.filter.clone()).collect();
        for f in new_filters {
            let key = (&f.stream, &f.filter);
            if !all_filters.iter().any(|e| (&e.stream, &e.filter) == key) {
                all_filters.push(f.clone());
            }
        }

        let new_session = connection::fetch_session(&self.http, &self.base_url, &all_filters).await?;
        self.session = Some(new_session.clone());

        let sink = self.sink.as_mut().ok_or_else(|| {
            ShurikenError::Session("No WebSocket connection".into())
        })?;

        for sub in self.subscriptions.iter_mut() {
            if sub.resolved.is_some() {
                continue;
            }
            if let Some(resolved) = new_session
                .subscriptions
                .iter()
                .find(|r| r.stream == sub.filter.stream)
            {
                sub.channel = resolved.channel.clone();
                sub.event = resolved.event.clone();
                sub.resolved = Some(resolved.clone());
                if let Err(e) = connection::pusher_subscribe(
                    sink,
                    &self.http,
                    &self.base_url,
                    &new_session,
                    self.socket_id.as_ref().unwrap(),
                    &resolved.channel,
                    &resolved.visibility,
                )
                .await
                {
                    warn!("Failed to subscribe to channel {}: {e}", resolved.channel);
                }
            }
        }
        Ok(())
    }

    fn emit_state(&mut self, new_state: ConnectionState, reason: Option<String>) {
        self.state = new_state;
        let _ = self.state_tx.send(ConnectionStateEvent {
            state: new_state,
            reason,
        });
    }
}
```

**Important note for the implementing agent:** The event loop dispatch and shared subscription state is the trickiest part. The approach above has a known issue: `subscribe()` pushes to `self.subscriptions` but the event loop spawned in `connect()` doesn't see those. The implementing agent must resolve this by using `Arc<Mutex<Vec<ActiveSubscription>>>` shared between the event loop and the client. The client stores the Arc and pushes new subscriptions to it; the event loop reads from it for dispatch. Refactor accordingly — the types and method signatures above are correct, only the internal shared state mechanism needs adjustment.

- [ ] **Step 4: Verify it compiles**

```bash
cargo check --features ws
```

- [ ] **Step 5: Run all tests**

```bash
cargo test && cargo test --features ws
```

- [ ] **Step 6: Run clippy**

```bash
cargo clippy --features ws -- -D warnings
```

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: rewrite WS client as ShurikenWsClient with typed Stream subscriptions"
```

---

### Task 12: Update lib.rs exports and bump version

**Files:**
- Modify: `src/lib.rs`
- Modify: `Cargo.toml`

- [ ] **Step 1: Update `src/lib.rs` to export WS types**

```rust
// src/lib.rs
pub mod http;
mod error;

#[cfg(feature = "ws")]
pub mod ws;

pub use error::ShurikenError;
pub use http::ShurikenHttpClient;

#[cfg(feature = "ws")]
pub use ws::ShurikenWsClient;
#[cfg(feature = "ws")]
pub use ws::streams;
#[cfg(feature = "ws")]
pub use ws::subscription::Subscription;
#[cfg(feature = "ws")]
pub use ws::{ConnectionState, ConnectionStateEvent};

pub use shuriken_api_types as types;

pub use http::account;
pub use http::perps;
pub use http::portfolio;
pub use http::swap;
pub use http::tokens;
pub use http::trigger;
```

- [ ] **Step 2: Bump version to 0.3.0 in `Cargo.toml`**

Change `version = "0.2.0"` to `version = "0.3.0"`.

- [ ] **Step 3: Run full CI checks**

```bash
cargo check && cargo check --features ws && cargo test && cargo test --features ws && cargo clippy -- -D warnings && cargo clippy --features ws -- -D warnings && cargo fmt --check
```

Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: bump to v0.3.0, export ShurikenWsClient and typed stream types"
```

---

### Task 13: Update README.md for v0.3.0

**Files:**
- Modify: `README.md` in `shuriken-sdk-rs`

- [ ] **Step 1: Update README.md**

Update the README to reflect the new API:
- Change install to `shuriken-sdk = "0.3"`
- Update quick start to use `ShurikenHttpClient::new()` and namespace accessors
- Update WS examples to use `ShurikenWsClient::new()` and typed subscriptions with `StreamDef` constants
- Update all code examples to use new method names (`client.swap().get_quote()` instead of `client.get_swap_quote()`)

The implementing agent should read the current README.md and update each section to match the new API patterns. Key changes:
- `ShurikenClient::new` → `ShurikenHttpClient::new`
- `client.get_swap_quote(params)` → `client.swap().get_quote(params)`
- `client.ws.subscribe("svm.token.swaps", filter, |event| {...})` → `ws.subscribe(streams::SVM_TOKEN_SWAPS, SvmTokenFilter { ... }).await?` + `while let Some(event) = sub.next().await`

- [ ] **Step 2: Update CLAUDE.md**

Update version references and architecture notes to reflect `src/http/` and `src/ws/` structure.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "docs: update README and CLAUDE.md for v0.3.0 API redesign"
```

---

## Part 2: Quickstart Bootstrap

All tasks in Part 2 are executed in `/Users/nik/Projects/shuriken/shuriken-quickstart-rs/`.

### Task 14: Initialize quickstart project scaffold

**Files:**
- Create: `Cargo.toml`, `.gitignore`, `.env.example`, `CLAUDE.md`, `.github/workflows/ci.yml`

- [ ] **Step 1: Create `Cargo.toml`**

```toml
[package]
name = "shuriken-quickstart-rs"
version = "0.1.0"
edition = "2021"
description = "Quickstart examples for the Shuriken Rust SDK"
license = "MIT"

[dependencies]
shuriken-sdk = { version = "0.3", features = ["ws"] }
tokio = { version = "1", features = ["full"] }
dotenvy = "0.15"
serde_json = "1"
futures-util = "0.3"
```

Note: during development, you may need to use a path dependency instead of version:
```toml
shuriken-sdk = { path = "../shuriken-sdk-rs", features = ["ws"] }
```

- [ ] **Step 2: Create `.gitignore`**

```
/target
.env
```

- [ ] **Step 3: Create `.env.example`**

```
SHURIKEN_API_KEY=your-api-key-here
# SHURIKEN_API_URL=https://custom-api-url.example.com
```

- [ ] **Step 4: Create `CLAUDE.md`**

```markdown
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

```bash
cp .env.example .env       # add your API key
cargo run --example 01_account_info
```

## Adding new examples

Add a new file `examples/NN_name.rs` with a `#[tokio::main]` async main.
Use helpers from `shuriken_quickstart_rs` (the lib crate).

## Build & lint

```bash
cargo check
cargo clippy -- -D warnings
cargo fmt --check
```
```

- [ ] **Step 5: Create `.github/workflows/ci.yml`**

```yaml
name: CI

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

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: initialize quickstart project scaffold"
```

---

### Task 15: Create shared helpers

**Files:**
- Create: `src/lib.rs`

- [ ] **Step 1: Create `src/lib.rs`**

```rust
use shuriken_sdk::{ShurikenError, ShurikenHttpClient};
#[cfg(feature = "ws")]
use shuriken_sdk::ShurikenWsClient;

const LABS_URL: &str = "https://app.shuriken.trade/agents";

pub fn create_http_client() -> ShurikenHttpClient {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("SHURIKEN_API_KEY").unwrap_or_else(|_| {
        eprintln!("Missing SHURIKEN_API_KEY — copy .env.example to .env and add your key");
        eprintln!("Create one at: {LABS_URL}");
        std::process::exit(1);
    });

    let client = match std::env::var("SHURIKEN_API_URL") {
        Ok(url) => ShurikenHttpClient::with_base_url(&api_key, &url),
        Err(_) => ShurikenHttpClient::new(&api_key),
    };

    client.unwrap_or_else(|e| {
        eprintln!("Failed to create client: {e}");
        std::process::exit(1);
    })
}

pub fn create_ws_client() -> ShurikenWsClient {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("SHURIKEN_API_KEY").unwrap_or_else(|_| {
        eprintln!("Missing SHURIKEN_API_KEY — copy .env.example to .env and add your key");
        eprintln!("Create one at: {LABS_URL}");
        std::process::exit(1);
    });

    let client = match std::env::var("SHURIKEN_API_URL") {
        Ok(url) => ShurikenWsClient::with_base_url(&api_key, &url),
        Err(_) => ShurikenWsClient::new(&api_key),
    };

    client.unwrap_or_else(|e| {
        eprintln!("Failed to create WS client: {e}");
        std::process::exit(1);
    })
}

pub fn format_usd(value: f64) -> String {
    format!("${value:.2}")
}

pub fn format_token(value: f64, symbol: &str) -> String {
    if symbol.is_empty() {
        format!("{value:.6}")
    } else {
        format!("{value:.6} {symbol}")
    }
}

pub fn format_pct(value: f64) -> String {
    let sign = if value >= 0.0 { "+" } else { "" };
    format!("{sign}{value:.2}%")
}

pub fn log_section(title: &str) {
    println!("\n{}", "=".repeat(60));
    println!("  {title}");
    println!("{}", "=".repeat(60));
}

pub fn log_json(label: &str, data: &impl serde::Serialize) {
    println!("\n--- {label} ---");
    println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
}

pub fn handle_error(err: ShurikenError) {
    match &err {
        ShurikenError::Auth(_) => {
            eprintln!("\nAuthentication failed — your API key is missing or invalid.");
            eprintln!("Create or rotate your key at: {LABS_URL}");
        }
        _ => eprintln!("{err}"),
    }
    std::process::exit(1);
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add shared helpers (client constructors, formatters, error handling)"
```

---

### Task 16: Create basic read-only examples (01-06)

**Files:**
- Create: `examples/01_account_info.rs` through `examples/06_browse_perp_markets.rs`

Each example follows the same pattern: `#[tokio::main] async fn main()`, uses helpers from the lib crate, calls SDK methods, prints results. The implementing agent should reference the TS quickstart examples at `/Users/nik/Projects/shuriken/shuriken-quickstart-ts/examples/` for the exact logic and output format of each example, adapting to Rust idioms.

- [ ] **Step 1: Create `examples/01_account_info.rs`**

```rust
use shuriken_quickstart_rs::*;

#[tokio::main]
async fn main() {
    let client = create_http_client();

    log_section("Account Info");
    match client.account().get_me().await {
        Ok(info) => {
            println!("User ID: {}", info.user_id);
            println!("Display Name: {}", info.display_name.as_deref().unwrap_or("N/A"));
        }
        Err(e) => handle_error(e),
    }

    log_section("Wallets");
    match client.account().get_wallets().await {
        Ok(wallets) => {
            for w in &wallets {
                println!(
                    "  {} — {} ({})",
                    w.wallet_id,
                    w.address,
                    w.chain.as_deref().unwrap_or("unknown")
                );
            }
            println!("\n  Total: {} wallet(s)", wallets.len());
        }
        Err(e) => handle_error(e),
    }

    log_section("Trade Settings");
    match client.account().get_settings().await {
        Ok(settings) => log_json("Settings", &settings),
        Err(e) => handle_error(e),
    }

    log_section("API Key Usage & Limits");
    match client.account().get_usage().await {
        Ok(usage) => {
            println!("Key ID: {}", usage.key_id);
            println!("Scopes: {}", usage.scopes.join(", "));
            println!("Buys enabled: {}", usage.constraints.buys_enabled);
            println!("Sells enabled: {}", usage.constraints.sells_enabled);
            println!("Max executions/hour: {}", usage.constraints.max_executions_per_hour);
            println!("Max executions/day: {}", usage.constraints.max_executions_per_day);
        }
        Err(e) => handle_error(e),
    }
}
```

- [ ] **Step 2: Create examples 02-06**

The implementing agent should create each example by reading the corresponding TS example file and translating to Rust. Key references:
- `02_search_tokens.rs` — read `/Users/nik/Projects/shuriken/shuriken-quickstart-ts/examples/02-search-tokens.ts`
- `03_token_analytics.rs` — read `03-token-analytics.ts`
- `04_swap_quote.rs` — read `04-swap-quote.ts`
- `05_portfolio_overview.rs` — read `05-portfolio-overview.ts`
- `06_browse_perp_markets.rs` — read `06-browse-perp-markets.ts`

All examples use `create_http_client()`, the namespace API (`client.tokens().search(...)` etc.), and helper formatters.

- [ ] **Step 3: Verify all compile**

```bash
cargo check
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add basic read-only examples (01-06)"
```

---

### Task 17: Create trading examples (07-09)

**Files:**
- Create: `examples/07_execute_swap.rs`, `examples/08_trigger_orders.rs`, `examples/09_perp_trading.rs`

- [ ] **Step 1: Create examples 07-09**

The implementing agent should create each example by reading the corresponding TS example and translating to Rust. These examples use `const DRY_RUN: bool = true` for safety. Key differences from TS:
- No interactive prompts (Rust stdin is more verbose) — use defaults or CLI args via `std::env::args()`
- Use `tokio::time::sleep` instead of JS `setTimeout`
- Polling loops use `loop` with `tokio::time::sleep(Duration::from_secs(2))`

References:
- `07_execute_swap.rs` — read `07-execute-swap.ts`
- `08_trigger_orders.rs` — read `08-trigger-orders.ts`
- `09_perp_trading.rs` — read `09-perp-trading.ts`

- [ ] **Step 2: Verify all compile**

```bash
cargo check
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add trading examples (07-09) with dry-run safety"
```

---

### Task 18: Create streaming examples (10-13)

**Files:**
- Create: `examples/10_stream_token_swaps.rs` through `examples/13_stream_graduated_tokens.rs`

- [ ] **Step 1: Create examples 10-13**

These examples use `create_ws_client()`, `ws.connect().await?`, typed subscriptions with `StreamDef` constants, and `futures_util::StreamExt` for `.next().await`. Each runs for a fixed duration using `tokio::time::timeout` or `tokio::select!` with `tokio::time::sleep`.

References:
- `10_stream_token_swaps.rs` — read `10-stream-token-swaps.ts`
- `11_stream_wallet.rs` — read `11-stream-wallet.ts`
- `12_stream_new_tokens.rs` — read `12-stream-new-tokens.ts`
- `13_stream_graduated_tokens.rs` — read `13-stream-graduated-tokens.ts`

Pattern for all streaming examples:

```rust
use futures_util::StreamExt;
use shuriken_sdk::streams;
use shuriken_quickstart_rs::*;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let mut ws = create_ws_client();
    ws.connect().await.unwrap_or_else(|e| handle_error(e));

    let mut sub = ws
        .subscribe(streams::SVM_TOKEN_SWAPS, shuriken_sdk::streams::SvmTokenFilter {
            token_address: "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN".into(),
        })
        .await
        .unwrap_or_else(|e| handle_error(e));

    log_section("Streaming Token Swaps (30s)");

    let timeout = tokio::time::sleep(Duration::from_secs(30));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(event) = sub.next() => {
                let side = if event.is_buy { "BUY" } else { "SELL" };
                println!("{side} {:.4} SOL (${:.2}) — {}", event.size_sol, event.size_usd, event.signature);
            }
            _ = &mut timeout => {
                println!("\nTimeout reached, disconnecting...");
                break;
            }
        }
    }

    ws.disconnect().await;
}
```

- [ ] **Step 2: Verify all compile**

```bash
cargo check
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add streaming examples (10-13) with typed subscriptions"
```

---

### Task 19: Create advanced composite examples (14-20)

**Files:**
- Create: `examples/14_token_sniper.rs` through `examples/20_watchlist_dashboard.rs`

- [ ] **Step 1: Create examples 14-20**

These are the most complex examples, combining HTTP and WS. The implementing agent should read each TS counterpart and translate to Rust. Key patterns:
- Examples that trade (14, 15, 18, 19) use `const DRY_RUN: bool = true`
- Use `tokio::select!` for multiplexing WS streams with timeouts
- Use `client.clone()` when the HTTP client is needed inside a loop that also uses WS

References:
- `14_token_sniper.rs` — read `14-token-sniper.ts`
- `15_whale_copy_trader.rs` — read `15-whale-copy-trader.ts`
- `16_portfolio_rebalancer.rs` — read `16-portfolio-rebalancer.ts`
- `17_new_token_screener.rs` — read `17-new-token-screener.ts`
- `18_perps_hedger.rs` — read `18-perps-hedger.ts`
- `19_trailing_stop.rs` — read `19-trailing-stop.ts`
- `20_watchlist_dashboard.rs` — read `20-watchlist-dashboard.ts`

- [ ] **Step 2: Verify all compile**

```bash
cargo check
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add advanced composite examples (14-20)"
```

---

### Task 20: Create README.md and run final CI checks

**Files:**
- Create: `README.md`

- [ ] **Step 1: Create `README.md`**

The implementing agent should create a README following the same structure as the TS quickstart README at `/Users/nik/Projects/shuriken/shuriken-quickstart-ts/README.md`, adapted for Rust:
- Setup instructions (clone, copy .env, cargo run)
- Complete examples table (20 examples in 4 categories)
- Links to SDK docs and GitHub repos

- [ ] **Step 2: Run full CI checks**

```bash
cargo check && cargo clippy -- -D warnings && cargo fmt --check
```

Expected: all pass.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "docs: add README with setup guide and examples table"
```
