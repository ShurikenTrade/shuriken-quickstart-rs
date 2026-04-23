/// 13 — Stream Graduated Tokens
///
/// Subscribe to bonding curve graduation events on Solana. When a token
/// migrates from a bonding curve (e.g. pump.fun) to a full DEX pool,
/// a graduation event fires.
///
/// Runs for 60 seconds, then disconnects.
use futures_util::StreamExt;
use shuriken_quickstart_rs::*;
use shuriken_sdk::streams::{self, NoFilter};
use std::time::Duration;

#[tokio::main]
async fn main() {
    log_section("Streaming Bonding Curve Graduations");
    println!("  Connecting to WebSocket...");

    let mut ws = create_ws_client();
    if let Err(e) = ws.connect().await {
        handle_error(e);
    }
    println!("  Connected! Listening for 60 seconds...\n");

    let mut count: u64 = 0;

    let mut sub = ws
        .subscribe(streams::SVM_BONDING_CURVE_GRADUATIONS, NoFilter)
        .await
        .unwrap_or_else(|e| handle_error(e));

    let timeout = tokio::time::sleep(Duration::from_secs(60));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(Ok(event)) = sub.next() => {
                count += 1;
                println!("  #{count} Graduation");
                println!("    Token     : {}", event.token_address);
                println!("    Curve     : {}", event.curve_address);
                println!("    Curve DEX : {}", event.curve_dex_type);
                println!("    Dest Pool : {}", event.dest_pool_address);
                println!("    Dest DEX  : {}", event.dest_pool_dex_type);
                println!("    Sig       : {}", event.signature);
                println!("    Slot      : {}", event.slot);
                println!("    Block     : {}", event.block_height);
                println!();
            }
            _ = &mut timeout => {
                println!("  Received {count} graduation events. Disconnecting...");
                break;
            }
        }
    }

    ws.disconnect().await;
}
