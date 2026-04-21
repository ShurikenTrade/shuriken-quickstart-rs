/// 12 — Stream New Tokens
///
/// Subscribe to bonding curve creation events on Solana. Every time a
/// new token launches via a bonding curve (e.g. pump.fun), you'll see
/// it here in real-time.
///
/// Runs for 60 seconds, then disconnects.
use futures_util::StreamExt;
use shuriken_quickstart_rs::*;
use shuriken_sdk::streams::{self, NoFilter};
use std::time::Duration;

#[tokio::main]
async fn main() {
    log_section("Streaming New Bonding Curve Tokens");
    println!("  Connecting to WebSocket...");

    let mut ws = create_ws_client();
    if let Err(e) = ws.connect().await {
        handle_error(e);
    }
    println!("  Connected! Listening for 60 seconds...\n");

    let mut count: u64 = 0;

    let mut sub = ws
        .subscribe(streams::SVM_BONDING_CURVE_CREATIONS, NoFilter)
        .await
        .unwrap_or_else(|e| handle_error(e));

    let timeout = tokio::time::sleep(Duration::from_secs(60));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(event) = sub.next() => {
                count += 1;
                println!("  #{count} New Token");
                println!("    Token   : {}", event.token_address);
                println!("    Curve   : {}", event.curve_address);
                println!("    DEX     : {}", event.curve_dex_type);
                println!("    Creator : {}", event.creator);
                println!("    Sig     : {}", event.signature);
                println!("    Slot    : {}", event.slot);
                println!("    Block   : {}", event.block_height);
                println!();
            }
            _ = &mut timeout => {
                println!("  Received {count} new token events. Disconnecting...");
                break;
            }
        }
    }

    ws.disconnect().await;
}
