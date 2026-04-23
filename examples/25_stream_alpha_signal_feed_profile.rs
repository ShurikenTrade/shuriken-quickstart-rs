/// 25 — Stream Profile Signal Feed
///
/// Subscribe to a profile-scoped signal feed by profile ID. Profile
/// feeds aggregate signals from all feeds linked to a specific profile.
///
/// Usage:
///   cargo run --example 25_stream_alpha_signal_feed_profile -- <profileId>
///
/// Runs for 60 seconds, then disconnects.
use futures_util::StreamExt;
use shuriken_quickstart_rs::*;
use shuriken_sdk::streams::{self, AlphaProfileFilter};
use shuriken_sdk::types::signal::SignalSource;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let Some(profile_id) = std::env::args().nth(1) else {
        eprintln!("Usage: cargo run --example 25_stream_alpha_signal_feed_profile -- <profileId>");
        std::process::exit(1);
    };

    log_section(&format!("Streaming Profile Signal Feed — {profile_id}"));
    println!("  Connecting to WebSocket...");

    let mut ws = create_ws_client();
    if let Err(e) = ws.connect().await {
        handle_error(e);
    }
    println!("  Connected! Listening for 60 seconds...\n");

    let mut sub = ws
        .subscribe(
            streams::ALPHA_SIGNAL_FEED_PROFILE,
            AlphaProfileFilter { profile_id },
        )
        .await
        .unwrap_or_else(|e| handle_error(e));

    let mut count: u64 = 0;
    let timeout = tokio::time::sleep(Duration::from_secs(60));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(Ok(event)) = sub.next() => {
                count += 1;
                let (symbol, name) = event
                    .token_meta
                    .as_ref()
                    .map(|m| (m.symbol.clone(), m.name.clone()))
                    .unwrap_or_else(|| ("???".into(), "Unknown".into()));

                println!("  #{count} {symbol} ({name}) on {:?}", event.network);
                println!("    Token   : {}", event.token_address);
                if let Some(pid) = &event.profile_id {
                    println!("    Profile : {pid}");
                }
                if let Some(s) = &event.latest_signal {
                    println!("    Signal  : {} at ts={}", source_label(&s.source), s.timestamp_ms);
                    println!("    Price   : {}", format_usd(s.price_usd));
                    println!("    MCap    : {}", format_usd(s.marketcap_usd));
                    println!("    Liq     : {}", format_usd(s.liquidity_usd));
                }
                println!();
            }
            _ = &mut timeout => {
                println!("  Received {count} profile feed events. Disconnecting...");
                break;
            }
        }
    }

    ws.disconnect().await;
}

fn source_label(source: &SignalSource) -> &'static str {
    match source {
        SignalSource::Discord(_) => "discord",
        SignalSource::Telegram(_) => "telegram",
        SignalSource::X(_) => "x",
        SignalSource::Trade(_) => "trade",
    }
}
