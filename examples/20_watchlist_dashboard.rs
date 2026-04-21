/// 20 — Watchlist Dashboard
///
/// Batch-fetches a configurable watchlist of tokens and displays
/// prices, 24h stats, and pool liquidity in a formatted table.
/// Refreshes every 30 seconds for 10 rounds (5 minutes total).
use std::time::Duration;

use shuriken_quickstart_rs::*;

const WATCHLIST: &[&str] = &[
    "solana:So11111111111111111111111111111111111111112", // SOL
    "solana:EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
    "solana:JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN", // JUP
];

const REFRESH_INTERVAL_SECS: u64 = 30;
const MAX_REFRESHES: u32 = 10;

#[tokio::main]
async fn main() {
    let client = create_http_client();

    // Batch-fetch token metadata once
    let token_ids: Vec<String> = WATCHLIST.iter().map(|s| (*s).to_string()).collect();
    let token_meta = match client.tokens().batch(&token_ids).await {
        Ok(m) => m,
        Err(e) => handle_error(e),
    };

    let token_symbols: std::collections::HashMap<&str, &str> = token_meta
        .tokens
        .iter()
        .map(|t| (t.token_id.as_str(), t.symbol.as_str()))
        .collect();

    for round in 1..=MAX_REFRESHES {
        println!("{}", "=".repeat(110));
        println!("  WATCHLIST DASHBOARD  (refresh {round}/{MAX_REFRESHES})");
        println!("{}", "=".repeat(110));

        // Fetch all data for each token
        let mut prices = Vec::new();
        let mut all_stats = Vec::new();
        let mut all_pools = Vec::new();

        for token_id in WATCHLIST {
            prices.push(client.tokens().get_price(token_id).await.ok());
            all_stats.push(client.tokens().get_stats(token_id).await.ok());
            all_pools.push(client.tokens().get_pools(token_id).await.ok());
        }

        // Header
        println!(
            "\n  {:<10}{:<16}{:<10}{:<10}{:<10}{:<16}{:<16}Buyers/Sellers",
            "Token", "Price", "5m", "1h", "24h", "Vol 24h", "Liquidity",
        );
        println!("  {}", "-".repeat(106));

        for (i, token_id) in WATCHLIST.iter().enumerate() {
            let symbol = token_symbols
                .get(token_id)
                .copied()
                .unwrap_or(&token_id[..token_id.len().min(8)]);

            let price_str = prices[i]
                .as_ref()
                .map(|p| format_usd(p.price_usd.unwrap_or(0.0)))
                .unwrap_or_else(|| "N/A".into());

            let stats = &all_stats[i];

            let chg_5m = stats
                .as_ref()
                .and_then(|s| s.price_change.m5)
                .map(|v| format_pct(v))
                .unwrap_or_else(|| "N/A".into());

            let chg_1h = stats
                .as_ref()
                .and_then(|s| s.price_change.h1)
                .map(|v| format_pct(v))
                .unwrap_or_else(|| "N/A".into());

            let chg_24h = stats
                .as_ref()
                .and_then(|s| s.price_change.h24)
                .map(|v| format_pct(v))
                .unwrap_or_else(|| "N/A".into());

            let vol_24h = stats
                .as_ref()
                .map(|s| {
                    format_usd(s.volume.buy24h.unwrap_or(0.0) + s.volume.sell24h.unwrap_or(0.0))
                })
                .unwrap_or_else(|| "N/A".into());

            let liquidity = all_pools[i]
                .as_ref()
                .and_then(|p| p.pools.first())
                .and_then(|p| p.liquidity_usd.as_deref())
                .and_then(|l| l.parse::<f64>().ok())
                .map(|v| format_usd(v))
                .unwrap_or_else(|| "N/A".into());

            let buyers = stats
                .as_ref()
                .and_then(|s| s.unique_traders.buyers24h)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "?".into());

            let sellers = stats
                .as_ref()
                .and_then(|s| s.unique_traders.sellers24h)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "?".into());

            println!(
                "  {:<10}{:<16}{:<10}{:<10}{:<10}{:<16}{:<16}{}/{}",
                symbol, price_str, chg_5m, chg_1h, chg_24h, vol_24h, liquidity, buyers, sellers,
            );
        }

        if !token_meta.not_found.is_empty() {
            println!("\n  Not found: {}", token_meta.not_found.join(", "));
        }

        if round < MAX_REFRESHES {
            println!("\n  Next refresh in {REFRESH_INTERVAL_SECS}s... (Ctrl+C to stop)");
            tokio::time::sleep(Duration::from_secs(REFRESH_INTERVAL_SECS)).await;
        }
    }

    println!("\n  Dashboard complete.");
}
