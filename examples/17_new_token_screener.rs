/// 17 — New Token Screener
///
/// Streams new bonding curve creations, enriches each token with
/// on-chain analytics (stats + pools), then ranks them in a live
/// leaderboard by liquidity and volume.
///
/// This is purely read-only -- no trades are executed.
use std::time::Duration;

use futures_util::StreamExt;
use shuriken_quickstart_rs::*;
use shuriken_sdk::streams::{self, NoFilter};

struct TokenScore {
    address: String,
    dex_type: String,
    liquidity: f64,
    volume_24h: f64,
    buyers_24h: u64,
    sellers_24h: u64,
    price_change_5m: f64,
}

#[tokio::main]
async fn main() {
    let client = create_http_client();

    log_section("New Token Screener");
    println!("  Connecting to WebSocket...");

    let mut ws = create_ws_client();
    if let Err(e) = ws.connect().await {
        handle_error(e);
    }
    println!("  Connected! Screening new tokens for 5 minutes...\n");

    let mut sub = ws
        .subscribe(streams::SVM_BONDING_CURVE_CREATIONS, NoFilter)
        .await
        .unwrap_or_else(|e| handle_error(e));

    let mut leaderboard: Vec<TokenScore> = Vec::new();

    let timeout = tokio::time::sleep(Duration::from_secs(300));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(Ok(event)) = sub.next() => {
                let token_id = format!("solana:{}", event.token_address);

                let stats = client.tokens().get_stats(&token_id).await.ok();
                let pools = client.tokens().get_pools(&token_id).await.ok();

                let liquidity = pools
                    .as_ref()
                    .and_then(|p| p.pools.first())
                    .and_then(|p| p.liquidity_usd.as_deref())
                    .and_then(|l| l.parse::<f64>().ok())
                    .unwrap_or(0.0);

                let volume_24h = stats
                    .as_ref()
                    .map(|s| s.volume.buy24h.unwrap_or(0.0) + s.volume.sell24h.unwrap_or(0.0))
                    .unwrap_or(0.0);

                let buyers_24h = stats
                    .as_ref()
                    .and_then(|s| s.unique_traders.buyers24h)
                    .unwrap_or(0);

                let sellers_24h = stats
                    .as_ref()
                    .and_then(|s| s.unique_traders.sellers24h)
                    .unwrap_or(0);

                let price_change_5m = stats
                    .as_ref()
                    .and_then(|s| s.price_change.m5)
                    .unwrap_or(0.0);

                leaderboard.push(TokenScore {
                    address: event.token_address.clone(),
                    dex_type: event.curve_dex_type.clone(),
                    liquidity,
                    volume_24h,
                    buyers_24h,
                    sellers_24h,
                    price_change_5m,
                });

                // Print updated leaderboard (top 10 by liquidity)
                leaderboard.sort_by(|a, b| b.liquidity.partial_cmp(&a.liquidity).unwrap_or(std::cmp::Ordering::Equal));
                let top: Vec<&TokenScore> = leaderboard.iter().take(10).collect();

                println!("{}", "=".repeat(100));
                println!("  NEW TOKEN SCREENER -- Live Leaderboard (sorted by liquidity)");
                println!("{}", "=".repeat(100));
                println!(
                    "  {:<4}{:<46}{:<14}{:<14}{:<10}{:<10}",
                    "#", "Token", "Liq", "Vol 24h", "Buyers", "5m Chg",
                );
                println!("  {}", "-".repeat(96));

                for (i, t) in top.iter().enumerate() {
                    let sign = if t.price_change_5m >= 0.0 { "+" } else { "" };
                    println!(
                        "  {:<4}{:<46}{:<14}{:<14}{:<10}{}",
                        i + 1,
                        t.address,
                        format_usd(t.liquidity),
                        format_usd(t.volume_24h),
                        t.buyers_24h,
                        format!("{sign}{:.1}%", t.price_change_5m),
                    );
                }

                println!("\n  Total tokens discovered: {}\n", leaderboard.len());
            }
            _ = &mut timeout => {
                println!("  Screener complete. Disconnecting...");
                break;
            }
        }
    }

    ws.disconnect().await;
}
