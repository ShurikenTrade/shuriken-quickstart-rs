/// 11 — Stream Wallet Balance
///
/// Subscribe to native SOL balance changes for a wallet address.
/// Prints each balance update for 30 seconds, then disconnects.
///
/// Pass a wallet address as the first CLI argument, or it uses your
/// first registered Solana wallet.
use futures_util::StreamExt;
use shuriken_quickstart_rs::*;
use shuriken_sdk::streams::{self, SvmWalletFilter};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let wallet_address = match std::env::args().nth(1) {
        Some(addr) => addr,
        None => {
            let http = create_http_client();
            let wallets = http
                .account()
                .get_wallets()
                .await
                .unwrap_or_else(|e| handle_error(e));

            let sol_wallet = wallets
                .iter()
                .find(|w| w.chain.as_deref() == Some("solana") || w.chain.is_none());

            match sol_wallet {
                Some(w) => w.address.clone(),
                None => {
                    eprintln!("No Solana wallet found — pass a wallet address as argument");
                    std::process::exit(1);
                }
            }
        }
    };

    log_section(&format!("Streaming SOL Balance — {wallet_address}"));
    println!("  Connecting to WebSocket...");

    let mut ws = create_ws_client();
    if let Err(e) = ws.connect().await {
        handle_error(e);
    }
    println!("  Connected! Listening for 30 seconds...\n");

    let mut count: u64 = 0;

    let mut sub = ws
        .subscribe(
            streams::SVM_WALLET_NATIVE_BALANCE,
            SvmWalletFilter {
                wallet_address: wallet_address.clone(),
            },
        )
        .await
        .unwrap_or_else(|e| handle_error(e));

    let timeout = tokio::time::sleep(Duration::from_secs(30));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(Ok(event)) = sub.next() => {
                count += 1;
                let pre = event.pre_balance as f64 / 1e9;
                let post = event.post_balance as f64 / 1e9;
                let delta = post - pre;
                let sign = if delta >= 0.0 { "+" } else { "" };
                println!(
                    "  #{count}  {} -> {}  ({sign}{delta:.9} SOL)  slot={}",
                    format_token(pre, "SOL"),
                    format_token(post, "SOL"),
                    event.slot,
                );
            }
            _ = &mut timeout => {
                println!("\n  Received {count} balance events. Disconnecting...");
                break;
            }
        }
    }

    ws.disconnect().await;
}
