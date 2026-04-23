/// 22 — Stream Global Signal Feed
///
/// Subscribe to the global signal feed — real-time token signals
/// aggregated across Discord, Telegram, X, and on-chain trades.
///
/// Runs for 60 seconds, then disconnects.
use futures_util::StreamExt;
use shuriken_quickstart_rs::*;
use shuriken_sdk::streams;
use std::time::Duration;

#[tokio::main]
async fn main() {
    log_section("Streaming Global Signal Feed");
    println!("  Connecting to WebSocket...");

    let mut ws = create_ws_client();
    if let Err(e) = ws.connect().await {
        handle_error(e);
    }
    println!("  Connected! Listening for 60 seconds...\n");

    let mut sub = ws
        .subscribe(streams::ALPHA_SIGNAL_FEED_GLOBAL, streams::NoFilter)
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
                if let Some(s) = &event.latest_signal {
                    println!("    Signal  : {} at ts={}", source_label(&s.source), s.timestamp_ms);
                    println!("    Price   : {}", format_usd(s.price_usd));
                    println!("    MCap    : {}", format_usd(s.marketcap_usd));
                    println!("    Liq     : {}", format_usd(s.liquidity_usd));
                    if let Some(dex) = &s.dex_name {
                        println!("    DEX     : {dex}");
                    }
                }
                println!();
            }
            _ = &mut timeout => {
                println!("  Received {count} global signal feed events. Disconnecting...");
                break;
            }
        }
    }

    ws.disconnect().await;
}

fn source_label(source: &shuriken_sdk::types::signal::SignalSource) -> &'static str {
    use shuriken_sdk::types::signal::SignalSource;
    match source {
        SignalSource::Discord(_) => "discord",
        SignalSource::Telegram(_) => "telegram",
        SignalSource::X(_) => "x",
        SignalSource::Trade(_) => "trade",
    }
}
