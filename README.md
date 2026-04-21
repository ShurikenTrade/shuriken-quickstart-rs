# Shuriken Quickstart — Rust

Runnable examples for the [Shuriken Rust SDK](https://github.com/ShurikenTrade/shuriken-sdk-rs).

## Setup

```bash
git clone https://github.com/ShurikenTrade/shuriken-quickstart-rs.git
cd shuriken-quickstart-rs
cp .env.example .env
```

Add your API key to `.env`. You can get one at [app.shuriken.trade/agents](https://app.shuriken.trade/agents).

## Running Examples

```bash
cargo run --example 01_account_info
```

Replace `01_account_info` with the name of any example file (without the `.rs` extension).

## Examples

### Basic (read-only)

| # | Example | Description |
|---|---------|-------------|
| 01 | `account_info` | Fetch account profile, wallets, settings, and API key usage limits |
| 02 | `search_tokens` | Search tokens by name/symbol, fetch token info and price |
| 03 | `token_analytics` | Price, OHLCV chart, trading stats, and liquidity pools for a token |
| 04 | `swap_quote` | Get a swap quote without executing (SOL → USDC) |
| 05 | `portfolio_overview` | Cross-chain balances, PnL, open positions, and trade history |
| 06 | `browse_perp_markets` | List perp markets, inspect order book and funding rates |

### Trading (executes real transactions)

| # | Example | Description |
|---|---------|-------------|
| 07 | `execute_swap` | Execute a managed swap and poll for confirmation |
| 08 | `trigger_orders` | Create a conditional trigger order, list, then cancel it |
| 09 | `perp_trading` | Place a limit order with TP/SL on Hyperliquid, list, then cancel |

> Trading examples default to a `DRY_RUN` mode — set `DRY_RUN=false` in `.env` to send real transactions.

### Streaming (WebSocket, long-running)

| # | Example | Description |
|---|---------|-------------|
| 10 | `stream_token_swaps` | Real-time Solana swap events for a token (30 s) |
| 11 | `stream_wallet` | Native SOL balance change notifications (30 s) |
| 12 | `stream_new_tokens` | New bonding curve token creations (60 s) |
| 13 | `stream_graduated_tokens` | Bonding curve graduation events (60 s) |

### Advanced (composite use cases)

| # | Example | Description |
|---|---------|-------------|
| 14 | `token_sniper` | Stream new tokens → analyze → auto-buy if criteria met |
| 15 | `whale_copy_trader` | Monitor a whale wallet → copy their new positions |
| 16 | `portfolio_rebalancer` | Compare portfolio to target allocation → generate rebalance quotes |
| 17 | `new_token_screener` | Live leaderboard of new tokens ranked by liquidity and volume |
| 18 | `perps_hedger` | Delta-hedge spot positions with opposing perp shorts |
| 19 | `trailing_stop` | Stream price → dynamically update trigger orders as a trailing stop |
| 20 | `watchlist_dashboard` | Auto-refreshing multi-token dashboard with prices, stats, and liquidity |

## SDK Documentation

See the [SDK README](https://github.com/ShurikenTrade/shuriken-sdk-rs) and the [Shuriken API docs](https://docs.shuriken.trade) for the full API reference.

## License

MIT
