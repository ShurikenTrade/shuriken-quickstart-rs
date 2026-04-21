/// 07 — Execute Swap
///
/// Execute a managed swap using a Shuriken-hosted wallet. The platform
/// signs the transaction for you -- no private key needed.
///
/// WARNING: When DRY_RUN is set to false this example moves REAL funds.
/// It swaps a very small amount (0.001 SOL -> JUP) but review the
/// parameters before running.
use std::time::Duration;

use shuriken_quickstart_rs::*;
use shuriken_sdk::swap::{ExecuteSwapParams, GetSwapQuoteParams};

const DRY_RUN: bool = true;

const SOL: &str = "So11111111111111111111111111111111111111112";
const JUP: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

#[tokio::main]
async fn main() {
    let client = create_http_client();

    // ── Pick a Solana wallet ──────────────────────────────────────────
    log_section("Selecting Wallet");
    let wallets = match client.account().get_wallets().await {
        Ok(w) => w,
        Err(e) => handle_error(e),
    };

    let sol_wallet = wallets
        .iter()
        .find(|w| w.chain.as_deref() == Some("solana") || w.chain.is_none())
        .unwrap_or_else(|| {
            eprintln!("No Solana wallet found on your account");
            std::process::exit(1);
        });

    let label = sol_wallet.label.as_deref().unwrap_or(&sol_wallet.wallet_id);
    println!("  Wallet : {label} ({})", sol_wallet.address);

    // ── Get a quote first (read-only) ─────────────────────────────────
    log_section("Swap Quote: 0.001 SOL -> JUP");
    let quote = match client
        .swap()
        .get_quote(&GetSwapQuoteParams {
            chain: "solana".into(),
            input_mint: SOL.into(),
            output_mint: JUP.into(),
            amount: "1000000".into(), // 0.001 SOL (9 decimals)
            slippage_bps: Some(100),
        })
        .await
    {
        Ok(q) => q,
        Err(e) => handle_error(e),
    };

    println!("  In           : {} lamports", quote.in_amount);
    println!("  Out          : {} (raw)", quote.out_amount);
    println!("  Slippage     : {} bps", quote.slippage_bps);
    println!(
        "  Price Impact : {}",
        quote.price_impact_pct.as_deref().unwrap_or("N/A")
    );

    // ── Execute the swap ──────────────────────────────────────────────
    log_section("Executing Swap");

    if DRY_RUN {
        println!("  [DRY RUN] Would execute swap: 0.001 SOL -> JUP");
        println!("    Wallet   : {}", sol_wallet.wallet_id);
        println!("    Amount   : 1000000 lamports (0.001 SOL)");
        println!("    Slippage : 100 bps");
        println!();
        println!("  Set DRY_RUN = false to execute for real");
    } else {
        let result = match client
            .swap()
            .execute(&ExecuteSwapParams {
                chain: "solana".into(),
                input_mint: SOL.into(),
                output_mint: JUP.into(),
                amount: "1000000".into(),
                wallet_id: sol_wallet.wallet_id.clone(),
                slippage_bps: Some(100),
            })
            .await
        {
            Ok(r) => r,
            Err(e) => handle_error(e),
        };

        println!("  Task ID : {}", result.task_id);
        println!("  Status  : {}", result.status);

        // ── Poll until finished ───────────────────────────────────────
        log_section("Polling Status");
        let mut status = result;
        while status.status == "submitted" || status.status == "pending" {
            tokio::time::sleep(Duration::from_secs(2)).await;
            status = match client.swap().get_status(&status.task_id).await {
                Ok(s) => s,
                Err(e) => handle_error(e),
            };
            println!("  Status : {}", status.status);
        }

        log_section("Final Result");
        println!("  Status  : {}", status.status);
        println!("  Tx Hash : {}", status.tx_hash.as_deref().unwrap_or("N/A"));
        if let Some(err) = &status.error_message {
            println!("  Error   : {err}");
        }
    }
}
