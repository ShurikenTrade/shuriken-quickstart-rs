/// 08 — Trigger Orders
///
/// Create a conditional trigger order (buy-the-dip), then list all
/// trigger orders on the account.
///
/// WARNING: When DRY_RUN is set to false this creates a REAL trigger
/// order that stays active until triggered, cancelled, or expired.
use shuriken_quickstart_rs::*;
use shuriken_sdk::trigger::{CreateTriggerOrderParams, ListTriggerOrdersParams};

const DRY_RUN: bool = true;

const SOL: &str = "So11111111111111111111111111111111111111112";
const JUP: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

#[tokio::main]
async fn main() {
    let client = create_http_client();

    // ── Pick a Solana wallet ──────────────────────────────────────────
    log_section("Selecting Wallet");
    let wallets = match client.account().get_wallets().await {
        Ok(w) => w,
        Err(e) => handle_error(e),
    };

    let wallet = wallets
        .iter()
        .find(|w| w.chain.as_deref() == Some("solana") || w.chain.is_none())
        .unwrap_or_else(|| {
            eprintln!("No Solana wallet found on your account");
            std::process::exit(1);
        });

    let label = wallet.label.as_deref().unwrap_or(&wallet.wallet_id);
    println!("  Wallet : {label} ({})", wallet.address);

    // ── Get current JUP price for reference ───────────────────────────
    log_section("Current JUP Price");
    let price = match client.tokens().get_price(&format!("solana:{JUP}")).await {
        Ok(p) => p,
        Err(e) => handle_error(e),
    };

    let price_usd: f64 = price.price_usd.unwrap_or(0.0);
    println!("  JUP Price : {}", format_usd(price_usd));

    // Set trigger 10% below current price (buy the dip)
    let trigger_price = price_usd * 0.90;
    let trigger_value = format!("{trigger_price:.6}");

    // ── Create trigger order ──────────────────────────────────────────
    log_section("Creating Trigger Order");

    if DRY_RUN {
        println!("  [DRY RUN] Would create trigger order:");
        println!("    Pair        : SOL -> JUP");
        println!("    Amount      : 1000000 lamports (0.001 SOL)");
        println!("    Direction   : below (buy the dip)");
        println!("    Trigger at  : {}", format_usd(trigger_price));
        println!("    Current     : {}", format_usd(price_usd));
        println!();
        println!("  Set DRY_RUN = false to create for real");
    } else {
        let order = match client
            .trigger()
            .create(&CreateTriggerOrderParams {
                chain: "solana".into(),
                input_token: SOL.into(),
                output_token: JUP.into(),
                amount: "1000000".into(), // 0.001 SOL (9 decimals)
                wallet_id: wallet.wallet_id.clone(),
                trigger_metric: "price_usd".into(),
                trigger_direction: "below".into(),
                trigger_value: Some(trigger_value),
                ..Default::default()
            })
            .await
        {
            Ok(o) => o,
            Err(e) => handle_error(e),
        };

        println!("  Order ID  : {}", order.order_id);
        println!("  Status    : {}", order.status);
        println!(
            "  Trigger   : {} {} {}",
            order.trigger.metric,
            order.trigger.direction,
            order.trigger.value.as_deref().unwrap_or("N/A"),
        );
    }

    // ── List all trigger orders (read-only, always runs) ──────────────
    log_section("Your Trigger Orders");
    let list = match client
        .trigger()
        .list(&ListTriggerOrdersParams {
            limit: Some(10),
            ..Default::default()
        })
        .await
    {
        Ok(l) => l,
        Err(e) => handle_error(e),
    };

    if list.orders.is_empty() {
        println!("  No trigger orders found");
    } else {
        for o in &list.orders {
            let trigger_str = match &o.trigger {
                Some(t) => format!(
                    "{} {} {}",
                    t.metric,
                    t.direction,
                    t.value.as_deref().unwrap_or("trailing"),
                ),
                None => "N/A".into(),
            };
            let chain = o.chain.as_deref().unwrap_or("any");
            println!(
                "  {}  {:10}  [{}]  {} -> {}  {}",
                o.order_id,
                o.status,
                chain,
                &o.input_token[..8.min(o.input_token.len())],
                &o.output_token[..8.min(o.output_token.len())],
                trigger_str,
            );
        }
    }
}
