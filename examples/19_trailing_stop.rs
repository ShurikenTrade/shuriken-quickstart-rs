/// 19 — Trailing Stop
///
/// Streams real-time swap events for a token to track price, then
/// dynamically creates/updates trigger orders to implement a trailing
/// stop-loss that follows the price upward.
///
/// WARNING: This creates real trigger orders when DRY_RUN is set to false.
/// Review the configuration before running.
use std::time::Duration;

use futures_util::StreamExt;
use shuriken_quickstart_rs::*;
use shuriken_sdk::streams::{self, SvmTokenFilter};
use shuriken_sdk::trigger::CreateTriggerOrderParams;

const DRY_RUN: bool = true;

const TRAIL_PCT: f64 = 5.0; // Trailing stop distance (5% below peak)
const SELL_AMOUNT: &str = "1000000"; // Amount to sell in base units
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const DEFAULT_TOKEN: &str = "So11111111111111111111111111111111111111112";

#[tokio::main]
async fn main() {
    let token_address = std::env::args().nth(1).unwrap_or(DEFAULT_TOKEN.into());

    let client = create_http_client();

    // ── Pick a Solana wallet ──────────────────────────────────────────
    let wallets = match client.account().get_wallets().await {
        Ok(w) => w,
        Err(e) => handle_error(e),
    };

    let wallet = wallets
        .iter()
        .find(|w| w.chain.as_deref() == Some("solana") || w.chain.is_none())
        .unwrap_or_else(|| {
            eprintln!("No Solana wallet found on your account");
            std::process::exit(1);
        });

    // Get initial price
    let token_id = format!("solana:{token_address}");
    let initial_price = match client.tokens().get_price(&token_id).await {
        Ok(p) => p,
        Err(e) => handle_error(e),
    };

    let mut peak_price = initial_price.price_usd.unwrap_or(0.0);
    let mut stop_price = peak_price * (1.0 - TRAIL_PCT / 100.0);
    let mut active_order_id: Option<String> = None;

    log_section("Trailing Stop");
    println!("  Token     : {token_address}");
    println!("  Trail %   : {TRAIL_PCT}%");
    println!("  Initial   : {}", format_usd(peak_price));
    println!("  Stop      : {}", format_usd(stop_price));
    println!("  Dry Run   : {DRY_RUN}");

    let mut ws = create_ws_client();
    if let Err(e) = ws.connect().await {
        handle_error(e);
    }
    println!("\n  Streaming prices (5 minutes)...\n");

    let mut sub = ws
        .subscribe(
            streams::SVM_TOKEN_SWAPS,
            SvmTokenFilter {
                token_address: token_address.clone(),
            },
        )
        .await
        .unwrap_or_else(|e| handle_error(e));

    let wallet_id = wallet.wallet_id.clone();
    let mut event_count: u64 = 0;

    let timeout = tokio::time::sleep(Duration::from_secs(300));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(event) = sub.next() => {
                event_count += 1;
                let price: f64 = event.price_usd.parse().unwrap_or(0.0);
                if price <= 0.0 {
                    continue;
                }

                if price > peak_price {
                    peak_price = price;
                    let new_stop = peak_price * (1.0 - TRAIL_PCT / 100.0);

                    if new_stop > stop_price {
                        let old_stop = stop_price;
                        stop_price = new_stop;

                        println!(
                            "  [{event_count}] NEW PEAK {} -- stop raised {} -> {}",
                            format_usd(peak_price),
                            format_usd(old_stop),
                            format_usd(stop_price),
                        );

                        if !DRY_RUN {
                            // Cancel old trigger and create new one
                            if let Some(ref oid) = active_order_id {
                                let _ = client.trigger().cancel(oid).await;
                            }

                            match client
                                .trigger()
                                .create(&CreateTriggerOrderParams {
                                    chain: "solana".into(),
                                    input_token: token_address.clone(),
                                    output_token: USDC_MINT.into(),
                                    amount: SELL_AMOUNT.into(),
                                    wallet_id: wallet_id.clone(),
                                    trigger_metric: "price_usd".into(),
                                    trigger_direction: "below".into(),
                                    trigger_value: Some(format!("{stop_price:.6}")),
                                    ..Default::default()
                                })
                                .await
                            {
                                Ok(order) => {
                                    active_order_id = Some(order.order_id.clone());
                                    println!("    -> Trigger updated: {}", order.order_id);
                                }
                                Err(e) => println!("    -> Error creating trigger: {e}"),
                            }
                        }
                    }
                }

                // Periodic status line
                if event_count % 20 == 0 {
                    let gap_pct = if peak_price > 0.0 {
                        ((peak_price - price) / peak_price) * 100.0
                    } else {
                        0.0
                    };
                    println!(
                        "  [{event_count}] Price: {}  Peak: {}  Stop: {}  Gap: {:.2}%",
                        format_usd(price),
                        format_usd(peak_price),
                        format_usd(stop_price),
                        gap_pct,
                    );
                }
            }
            _ = &mut timeout => {
                println!("\n  Processed {event_count} price events.");
                println!("  Final peak: {}  Stop: {}", format_usd(peak_price), format_usd(stop_price));
                if let Some(ref oid) = active_order_id {
                    println!("  Active trigger order: {oid}");
                }
                break;
            }
        }
    }

    ws.disconnect().await;
}
