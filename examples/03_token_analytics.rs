/// 03 — Token Analytics
///
/// Deep-dive into a single token: current price, OHLCV chart data,
/// trading stats (volume, txns, unique traders), and liquidity pools.
///
/// Pass a token ID as the first CLI argument, or it defaults to JUP.
use shuriken_quickstart_rs::*;
use shuriken_sdk::tokens::GetTokenChartParams;

const DEFAULT_TOKEN: &str = "solana:JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

#[tokio::main]
async fn main() {
    let client = create_http_client();
    let token_id = std::env::args().nth(1).unwrap_or(DEFAULT_TOKEN.into());

    // ── Price ──────────────────────────────────────────────────────────
    log_section(&format!("Price — {token_id}"));
    match client.tokens().get_price(&token_id).await {
        Ok(price) => {
            println!("  Price : {}", format_usd(price.price_usd.unwrap_or(0.0)));
        }
        Err(e) => handle_error(e),
    }

    // ── Chart (1h candles, last 24) ────────────────────────────────────
    log_section("OHLCV Chart (1h x 24)");
    match client
        .tokens()
        .get_chart(&GetTokenChartParams {
            token_id: token_id.clone(),
            resolution: Some("1h".into()),
            count: Some(24),
        })
        .await
    {
        Ok(chart) => {
            println!("  Resolution : {}", chart.resolution);
            println!("  Candles    : {}", chart.candles.len());
            if let Some(latest) = chart.candles.last() {
                println!(
                    "  Latest     : O={} H={} L={} C={}",
                    format_usd(latest.open),
                    format_usd(latest.high),
                    format_usd(latest.low),
                    format_usd(latest.close),
                );
            }
        }
        Err(e) => handle_error(e),
    }

    // ── Stats ──────────────────────────────────────────────────────────
    log_section("Trading Stats");
    match client.tokens().get_stats(&token_id).await {
        Ok(stats) => {
            println!("\n  Volume (USD):");
            println!(
                "    5m : buy {}  sell {}",
                format_usd(stats.volume.buy5m.unwrap_or(0.0)),
                format_usd(stats.volume.sell5m.unwrap_or(0.0)),
            );
            println!(
                "    1h : buy {}  sell {}",
                format_usd(stats.volume.buy1h.unwrap_or(0.0)),
                format_usd(stats.volume.sell1h.unwrap_or(0.0)),
            );
            println!(
                "   24h : buy {}  sell {}",
                format_usd(stats.volume.buy24h.unwrap_or(0.0)),
                format_usd(stats.volume.sell24h.unwrap_or(0.0)),
            );

            println!("\n  Price Change:");
            println!(
                "    5m : {}",
                format_pct(stats.price_change.m5.unwrap_or(0.0))
            );
            println!(
                "    1h : {}",
                format_pct(stats.price_change.h1.unwrap_or(0.0))
            );
            println!(
                "   24h : {}",
                format_pct(stats.price_change.h24.unwrap_or(0.0))
            );

            println!("\n  Unique Traders (24h):");
            println!(
                "    Buyers  : {}",
                stats.unique_traders.buyers24h.unwrap_or(0)
            );
            println!(
                "    Sellers : {}",
                stats.unique_traders.sellers24h.unwrap_or(0)
            );
        }
        Err(e) => handle_error(e),
    }

    // ── Pools ──────────────────────────────────────────────────────────
    log_section("Liquidity Pools");
    match client.tokens().get_pools(&token_id).await {
        Ok(pools) => {
            for pool in &pools.pools {
                let addr = pool.address.as_deref().unwrap_or("unknown");
                let liq = pool.liquidity_usd.as_deref().unwrap_or("N/A");
                let mcap = pool.market_cap_usd.as_deref().unwrap_or("N/A");
                println!("  {addr}");
                println!("    Liquidity  : {liq}");
                println!("    Market Cap : {mcap}");
            }
        }
        Err(e) => handle_error(e),
    }
}
