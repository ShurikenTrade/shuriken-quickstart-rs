/// 15 — Whale Copy Trader
///
/// Monitors a whale wallet's token balance changes via WebSocket. When
/// a new token appears in their wallet (balance goes from 0 to > 0),
/// we fetch a quote and optionally execute the same swap.
///
/// WARNING: Set DRY_RUN=false to execute real trades. Use with caution.
use std::collections::HashSet;
use std::time::Duration;

use futures_util::StreamExt;
use shuriken_quickstart_rs::*;
use shuriken_sdk::streams::{self, SvmWalletFilter};
use shuriken_sdk::swap::{ExecuteSwapParams, GetSwapQuoteParams};

const DRY_RUN: bool = true;

const COPY_AMOUNT_LAMPORTS: &str = "1000000"; // 0.001 SOL per copy trade
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

#[tokio::main]
async fn main() {
    let whale_address = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: cargo run --example 15_whale_copy_trader -- <whale-wallet-address>");
        std::process::exit(1);
    });

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

    log_section("Whale Copy Trader");
    println!("  Watching  : {whale_address}");
    println!("  My Wallet : {}", wallet.address);
    println!("  Copy Size : {COPY_AMOUNT_LAMPORTS} lamports (0.001 SOL)");
    println!("  Dry Run   : {DRY_RUN}");
    println!("\n  Connecting to WebSocket...");

    let mut ws = create_ws_client();
    if let Err(e) = ws.connect().await {
        handle_error(e);
    }
    println!("  Connected! Monitoring whale activity (10 minutes)...\n");

    let mut sub = ws
        .subscribe(
            streams::SVM_WALLET_TOKEN_BALANCES,
            SvmWalletFilter {
                wallet_address: whale_address,
            },
        )
        .await
        .unwrap_or_else(|e| handle_error(e));

    let wallet_id = wallet.wallet_id.clone();
    let mut known_tokens = HashSet::new();
    let mut copy_count: u64 = 0;

    let timeout = tokio::time::sleep(Duration::from_secs(600));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(event) = sub.next() => {
                let is_new_position = event.pre_balance == 0 && event.post_balance > 0;
                let is_sell = event.pre_balance > 0 && event.post_balance == 0;

                if is_new_position && !known_tokens.contains(&event.mint) {
                    known_tokens.insert(event.mint.clone());
                    let balance = event.post_balance as f64 / 10f64.powi(event.decimals as i32);

                    println!("  NEW POSITION detected!");
                    println!("    Token    : {}", event.mint);
                    println!("    Balance  : {}", format_token(balance, ""));
                    println!("    Slot     : {}", event.slot);

                    // Get a quote to see what we'd get
                    match client
                        .swap()
                        .get_quote(&GetSwapQuoteParams {
                            chain: "solana".into(),
                            input_mint: SOL_MINT.into(),
                            output_mint: event.mint.clone(),
                            amount: COPY_AMOUNT_LAMPORTS.into(),
                            slippage_bps: Some(300),
                        })
                        .await
                    {
                        Ok(quote) => {
                            println!("    Quote    : {} tokens out", quote.out_amount);
                            println!(
                                "    Impact   : {}",
                                quote.price_impact_pct.as_deref().unwrap_or("N/A"),
                            );

                            if DRY_RUN {
                                println!("    [DRY RUN] Would execute swap\n");
                                continue;
                            }

                            match client
                                .swap()
                                .execute(&ExecuteSwapParams {
                                    chain: "solana".into(),
                                    input_mint: SOL_MINT.into(),
                                    output_mint: event.mint.clone(),
                                    amount: COPY_AMOUNT_LAMPORTS.into(),
                                    wallet_id: wallet_id.clone(),
                                    slippage_bps: Some(300),
                                })
                                .await
                            {
                                Ok(result) => {
                                    copy_count += 1;
                                    println!("    COPIED! Task: {}", result.task_id);

                                    let task_id = result.task_id;
                                    loop {
                                        tokio::time::sleep(Duration::from_secs(2)).await;
                                        match client.tasks().get_status(&task_id).await {
                                            Ok(task) => {
                                                if task.status != "pending" {
                                                    println!(
                                                        "    Final: {} Tx: {}\n",
                                                        task.status,
                                                        task.tx_hash.as_deref().unwrap_or("N/A"),
                                                    );
                                                    break;
                                                }
                                            }
                                            Err(e) => {
                                                println!("    Poll error: {e}");
                                                break;
                                            }
                                        }
                                    }
                                }
                                Err(e) => println!("    Error executing swap: {e}\n"),
                            }
                        }
                        Err(e) => println!("    Error getting quote: {e}\n"),
                    }
                } else if is_sell {
                    println!("  WHALE SOLD: {} (full exit)\n", event.mint);
                }
            }
            _ = &mut timeout => {
                println!("\n  Copied {copy_count} trades. Disconnecting...");
                break;
            }
        }
    }

    ws.disconnect().await;
}
