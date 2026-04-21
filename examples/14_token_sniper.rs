/// 14 — Token Sniper
///
/// Streams new bonding curve token creations on Solana, fetches analytics
/// for each new token (stats + pools), and if criteria are met (e.g.
/// minimum liquidity, volume), executes a small swap to buy in early.
///
/// WARNING: This example will execute REAL swaps if DRY_RUN is set to false.
/// Adjust SNIPE_AMOUNT and the criteria to your risk tolerance.
use std::time::Duration;

use futures_util::StreamExt;
use shuriken_quickstart_rs::*;
use shuriken_sdk::streams::{self, NoFilter};
use shuriken_sdk::swap::ExecuteSwapParams;

const DRY_RUN: bool = true;

const SNIPE_AMOUNT_LAMPORTS: &str = "1000000"; // 0.001 SOL
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const MIN_LIQUIDITY_USD: f64 = 1000.0;

#[tokio::main]
async fn main() {
    let client = create_http_client();

    // ── Pick a Solana wallet ──────────────────────────────────────────
    log_section("Selecting Wallet");
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

    log_section("Token Sniper");
    println!("  Wallet    : {}", wallet.address);
    println!("  Amount    : {SNIPE_AMOUNT_LAMPORTS} lamports (0.001 SOL)");
    println!("  Min Liq   : {}", format_usd(MIN_LIQUIDITY_USD));
    println!("  Dry Run   : {DRY_RUN}");
    println!("\n  Connecting to WebSocket...");

    let mut ws = create_ws_client();
    if let Err(e) = ws.connect().await {
        handle_error(e);
    }
    println!("  Connected! Listening for new tokens (5 minutes)...\n");

    let mut sub = ws
        .subscribe(streams::SVM_BONDING_CURVE_CREATIONS, NoFilter)
        .await
        .unwrap_or_else(|e| handle_error(e));

    let wallet_id = wallet.wallet_id.clone();
    let mut seen: u64 = 0;
    let mut sniped: u64 = 0;

    let timeout = tokio::time::sleep(Duration::from_secs(300));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(event) = sub.next() => {
                seen += 1;
                let token_address = &event.token_address;
                println!("  [{seen}] New token: {token_address} ({})", event.curve_dex_type);

                let token_id = format!("solana:{token_address}");

                // Fetch token analytics
                let stats = client.tokens().get_stats(&token_id).await.ok();
                let pools = client.tokens().get_pools(&token_id).await.ok();

                // Check criteria
                let liquidity = pools
                    .as_ref()
                    .and_then(|p| p.pools.first())
                    .and_then(|p| p.liquidity_usd.as_deref())
                    .and_then(|l| l.parse::<f64>().ok())
                    .unwrap_or(0.0);

                let volume_24h = stats
                    .as_ref()
                    .map(|s| {
                        s.volume.buy24h.unwrap_or(0.0) + s.volume.sell24h.unwrap_or(0.0)
                    })
                    .unwrap_or(0.0);

                println!(
                    "    Liquidity: {} | 24h Vol: {}",
                    format_usd(liquidity),
                    format_usd(volume_24h),
                );

                if liquidity < MIN_LIQUIDITY_USD {
                    println!("    SKIP -- below min liquidity\n");
                    continue;
                }

                println!("    MATCH -- criteria met!");

                if DRY_RUN {
                    println!(
                        "    [DRY RUN] Would buy {SNIPE_AMOUNT_LAMPORTS} lamports of SOL -> {token_address}\n"
                    );
                    continue;
                }

                match client
                    .swap()
                    .execute(&ExecuteSwapParams {
                        chain: "solana".into(),
                        input_mint: SOL_MINT.into(),
                        output_mint: token_address.clone(),
                        amount: SNIPE_AMOUNT_LAMPORTS.into(),
                        wallet_id: wallet_id.clone(),
                        slippage_bps: Some(500),
                    })
                    .await
                {
                    Ok(result) => {
                        sniped += 1;
                        println!("    SNIPED! Task: {} Status: {}", result.task_id, result.status);

                        // Poll for final status
                        let mut status = result;
                        while status.status == "submitted" || status.status == "pending" {
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            match client.swap().get_status(&status.task_id).await {
                                Ok(s) => status = s,
                                Err(e) => {
                                    println!("    Poll error: {e}");
                                    break;
                                }
                            }
                        }
                        println!(
                            "    Final: {} Tx: {}\n",
                            status.status,
                            status.tx_hash.as_deref().unwrap_or("N/A"),
                        );
                    }
                    Err(e) => println!("    Error executing swap: {e}\n"),
                }
            }
            _ = &mut timeout => {
                println!("\n  Seen: {seen} tokens | Sniped: {sniped}");
                break;
            }
        }
    }

    ws.disconnect().await;
}
