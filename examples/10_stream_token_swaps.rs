/// 10 — Stream Token Swaps
///
/// Subscribe to real-time swap events for a Solana token via WebSocket.
/// Prints each swap as it happens for 30 seconds, then disconnects.
///
/// Pass a token address as the first CLI argument, or it defaults to JUP.
use futures_util::StreamExt;
use shuriken_quickstart_rs::*;
use shuriken_sdk::streams::{self, SvmTokenFilter};
use std::time::Duration;

const DEFAULT_TOKEN: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"; // JUP

#[tokio::main]
async fn main() {
    let token_address = std::env::args().nth(1).unwrap_or(DEFAULT_TOKEN.into());

    log_section(&format!("Streaming Swaps — {token_address}"));
    println!("  Connecting to WebSocket...");

    let mut ws = create_ws_client();
    if let Err(e) = ws.connect().await {
        handle_error(e);
    }
    println!("  Connected! Listening for 30 seconds...\n");

    let mut count: u64 = 0;

    let mut sub = ws
        .subscribe(
            streams::SVM_TOKEN_SWAPS,
            SvmTokenFilter {
                token_address: token_address.clone(),
            },
        )
        .await
        .unwrap_or_else(|e| handle_error(e));

    let timeout = tokio::time::sleep(Duration::from_secs(30));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(event) = sub.next() => {
                count += 1;
                let side = if event.is_buy { "BUY " } else { "SELL" };
                let sol: f64 = event.size_sol.parse().unwrap_or(0.0);
                let usd: f64 = event.size_usd.parse().unwrap_or(0.0);
                let maker = event.maker.as_deref().map_or("?".to_string(), |m| {
                    format!("{}...", &m[..m.len().min(8)])
                });
                println!(
                    "  #{count}  {side}  {}  {}  maker={maker}  sig={}...",
                    format_token(sol, "SOL"),
                    format_usd(usd),
                    &event.signature[..event.signature.len().min(16)],
                );
            }
            _ = &mut timeout => {
                println!("\n  Received {count} swap events. Disconnecting...");
                break;
            }
        }
    }

    ws.disconnect().await;
}
