/// 23 — Stream Personal Signal Feed
///
/// Subscribe to your personal signal feed — token signals filtered to
/// your configured feeds and watchlists. Prints the origin of each
/// signal (author + channel/chat/tweet/tx) so back-to-back events for
/// the same token are visibly distinguishable.
///
/// Runs for 60 seconds, then disconnects.
use futures_util::StreamExt;
use shuriken_quickstart_rs::*;
use shuriken_sdk::streams;
use shuriken_sdk::types::signal::SignalSource;
use std::time::Duration;

#[tokio::main]
async fn main() {
    log_section("Streaming Personal Signal Feed");
    println!("  Connecting to WebSocket...");

    let mut ws = create_ws_client();
    if let Err(e) = ws.connect().await {
        handle_error(e);
    }
    println!("  Connected! Listening for 60 seconds...\n");

    let mut sub = ws
        .subscribe(streams::ALPHA_SIGNAL_FEED_PERSONAL, streams::NoFilter)
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
                if let Some(feed) = &event.feed_id {
                    println!("    Feed    : {feed}");
                }
                if let Some(s) = &event.latest_signal {
                    println!("    Signal  : {} at ts={}", source_label(&s.source), s.timestamp_ms);
                    println!("    Origin  : {}", format_origin(&s.source));
                    println!("    Price   : {}", format_usd(s.price_usd));
                    println!("    MCap    : {}", format_usd(s.marketcap_usd));
                    println!("    Liq     : {}", format_usd(s.liquidity_usd));
                }
                println!();
            }
            _ = &mut timeout => {
                println!("  Received {count} personal signal feed events. Disconnecting...");
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

fn format_origin(source: &SignalSource) -> String {
    match source {
        SignalSource::Discord(s) => {
            let author = s
                .author_display_name
                .clone()
                .or_else(|| s.author_username.clone())
                .unwrap_or_else(|| s.author_id.clone());
            format!(
                "discord:{author} guild={} channel={} msg={}",
                s.guild_id, s.channel_id, s.message_id
            )
        }
        SignalSource::Telegram(s) => {
            let sender = s
                .sender_display_name
                .clone()
                .or_else(|| s.sender_username.clone())
                .unwrap_or_else(|| s.sender_id.clone());
            let topic = s
                .topic_title
                .as_ref()
                .map(|t| format!(" topic={t}"))
                .unwrap_or_default();
            format!(
                "telegram:{sender} chat={}{topic} msg={}",
                s.chat_id, s.message_id
            )
        }
        SignalSource::X(s) => {
            let author = s
                .author_display_name
                .clone()
                .or_else(|| s.author_username.clone())
                .unwrap_or_else(|| s.author_id.clone());
            format!("x:{author} tweet={}", s.tweet_id)
        }
        SignalSource::Trade(s) => {
            let side = if s.is_buy { "buy" } else { "sell" };
            format!(
                "trade:{side} ${} wallet={} tx={}",
                s.amount_usd, s.wallet_address, s.tx_signature
            )
        }
    }
}
