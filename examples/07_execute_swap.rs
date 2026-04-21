/// 07 — Execute Swap
///
/// Execute a managed swap using a Shuriken-hosted wallet. The platform
/// signs the transaction for you — no private key needed.
///
/// WARNING: This example moves REAL funds. It swaps a very small amount
/// (0.001 SOL → JUP) and asks for confirmation before executing.
use std::time::Duration;

use shuriken_quickstart_rs::*;
use shuriken_sdk::portfolio::GetBalancesParams;
use shuriken_sdk::swap::ExecuteSwapParams;

const SOL: &str = "So11111111111111111111111111111111111111112";
const JUP: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

#[tokio::main]
async fn main() {
    let client = create_http_client();

    // ── Pick a Solana wallet ──────────────────────────────────────────
    let wallets = match client.account().get_wallets().await {
        Ok(w) => w,
        Err(e) => handle_error(e),
    };

    let sol_wallets: Vec<_> = wallets
        .iter()
        .filter(|w| w.chain.as_deref() == Some("solana") || w.chain.is_none())
        .collect();

    if sol_wallets.is_empty() {
        eprintln!("No Solana wallet found on your account");
        std::process::exit(1);
    }

    let balances = client
        .portfolio()
        .get_balances(&GetBalancesParams {
            chain: Some("solana".into()),
        })
        .await
        .unwrap_or_default();

    let sol_wallet = if sol_wallets.len() > 1 {
        println!("\nAvailable Solana wallets:");
        for (i, w) in sol_wallets.iter().enumerate() {
            let label = w.label.as_deref().unwrap_or(&w.wallet_id);
            let bal = balances
                .iter()
                .find(|b| b.wallet_address == w.address)
                .map(|b| format!("{} SOL", b.native_balance))
                .unwrap_or_else(|| "unknown balance".into());
            println!("  [{}] {label} ({}) — {bal}", i + 1, w.address);
        }
        let idx = choose(
            &format!("\nSelect wallet (1-{}):  ", sol_wallets.len()),
            sol_wallets.len(),
        );
        sol_wallets[idx]
    } else {
        sol_wallets[0]
    };

    let label = sol_wallet.label.as_deref().unwrap_or(&sol_wallet.wallet_id);
    println!("\nUsing wallet: {label} ({})", sol_wallet.address);

    // ── Confirm before executing ──────────────────────────────────────
    if !confirm("\n⚠️  This will execute a REAL trade: 0.001 SOL → JUP. Type 'yes' to continue:  ")
    {
        println!("Aborted.");
        return;
    }

    // ── Execute: 0.001 SOL → JUP ─────────────────────────────────────
    log_section("Executing Swap: 0.001 SOL → JUP");
    let result = match client
        .swap()
        .execute(&ExecuteSwapParams {
            chain: "solana".into(),
            input_mint: SOL.into(),
            output_mint: JUP.into(),
            amount: "1000000".into(), // 0.001 SOL (9 decimals)
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

    // ── Poll until finished ───────────────────────────────────────────
    log_section("Polling Status");
    let task_id = result.task_id;
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        match client.tasks().get_status(&task_id).await {
            Ok(task) => {
                println!("  Status : {}", task.status);
                if task.status != "pending" {
                    log_section("Final Result");
                    println!("  Status  : {}", task.status);
                    println!("  Tx Hash : {}", task.tx_hash.as_deref().unwrap_or("N/A"));
                    if let Some(err) = &task.error_message {
                        println!("  Error   : {err}");
                    }
                    break;
                }
            }
            Err(e) => {
                println!("  Status poll error: {e}");
                break;
            }
        }
    }
}
