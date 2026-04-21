/// 01 — Account Info
///
/// Fetch your account profile, registered wallets, trade settings, and
/// agent-key usage limits. This is the simplest possible example and a
/// good first script to run to verify your API key works.
use shuriken_quickstart_rs::*;

#[tokio::main]
async fn main() {
    let client = create_http_client();

    // ── Profile ────────────────────────────────────────────────────────
    log_section("Account Profile");
    match client.account().get_me().await {
        Ok(me) => {
            println!("  User ID : {}", me.user_id);
            println!(
                "  Display : {}",
                me.display_name.as_deref().unwrap_or("(not set)")
            );
        }
        Err(e) => handle_error(e),
    }

    // ── Wallets ────────────────────────────────────────────────────────
    log_section("Wallets");
    match client.account().get_wallets().await {
        Ok(wallets) => {
            for w in &wallets {
                let chain = w.chain.as_deref().unwrap_or("any");
                let label = w.label.as_deref().unwrap_or(&w.wallet_id);
                println!("  [{chain}] {label} — {}", w.address);
            }
        }
        Err(e) => handle_error(e),
    }

    // ── Trade Settings ─────────────────────────────────────────────────
    log_section("Trade Settings");
    match client.account().get_settings().await {
        Ok(settings) => log_json("Trade Settings", &settings),
        Err(e) => handle_error(e),
    }

    // ── Agent Key Usage ────────────────────────────────────────────────
    log_section("Agent Key Usage & Limits");
    match client.account().get_usage().await {
        Ok(usage) => {
            println!("  Key ID : {}", usage.key_id);
            println!("  Scopes : {}", usage.scopes.join(", "));
            let c = &usage.constraints;
            println!("  Buys enabled          : {}", c.buys_enabled);
            println!("  Sells enabled         : {}", c.sells_enabled);
            println!("  Max executions/hour   : {}", c.max_executions_per_hour);
            println!("  Max executions/day    : {}", c.max_executions_per_day);
            println!("  Max concurrent        : {}", c.max_concurrent_executions);
            println!("  Max limit orders/day  : {}", c.max_limit_orders_per_day);
        }
        Err(e) => handle_error(e),
    }
}
