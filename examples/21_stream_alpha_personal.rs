/// 21 — Stream Personal Alpha
///
/// Subscribe to your personal alpha channel via WebSocket. Delivers
/// raw chat messages (Discord, Telegram, X) routed to your account.
///
/// Runs for 60 seconds, then disconnects.
use futures_util::StreamExt;
use shuriken_quickstart_rs::*;
use shuriken_sdk::streams;
use std::time::Duration;

#[tokio::main]
async fn main() {
    log_section("Streaming Personal Alpha");
    println!("  Connecting to WebSocket...");

    let mut ws = create_ws_client();
    if let Err(e) = ws.connect().await {
        handle_error(e);
    }
    println!("  Connected! Listening for 60 seconds...\n");

    let mut sub = ws
        .subscribe(streams::ALPHA_PERSONAL, streams::NoFilter)
        .await
        .unwrap_or_else(|e| handle_error(e));

    let mut count: u64 = 0;
    let timeout = tokio::time::sleep(Duration::from_secs(60));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            msg = sub.next() => {
                match msg {
                    Some(Ok(event)) => {
                        count += 1;
                        let author = event
                            .author
                            .as_ref()
                            .and_then(|a| a.display_name.clone().or_else(|| a.username.clone()))
                            .unwrap_or_else(|| "unknown".into());
                        let tokens: Vec<String> = event
                            .tokens
                            .iter()
                            .map(|t| t.address.chars().take(8).collect::<String>())
                            .collect();
                        let preview: String = event.content.chars().take(120).collect();
                        let suffix = if event.content.chars().count() > 120 { "..." } else { "" };

                        println!("  #{count} [{:?}] ts={}", event.platform, event.timestamp_ms);
                        println!("    Author  : {author}");
                        println!("    Content : {preview}{suffix}");
                        if !tokens.is_empty() {
                            println!("    Tokens  : {}", tokens.join(", "));
                        }
                        println!();
                    }
                    Some(Err(e)) => eprintln!("  Subscription error: {e}"),
                    None => {
                        println!("  Subscription closed.");
                        break;
                    }
                }
            }
            _ = &mut timeout => {
                println!("  Received {count} alpha events. Disconnecting...");
                break;
            }
        }
    }

    ws.disconnect().await;
}
