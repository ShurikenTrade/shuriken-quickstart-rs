/// 05 — Portfolio Overview
///
/// Fetch your cross-chain wallet balances, PnL summary, open positions,
/// and recent trade history -- a complete portfolio snapshot.
use shuriken_quickstart_rs::*;
use shuriken_sdk::portfolio::{
    GetBalancesParams, GetHistoryParams, GetPnlParams, GetPositionsParams,
};

#[tokio::main]
async fn main() {
    let client = create_http_client();

    // ── Balances ───────────────────────────────────────────────────────
    log_section("Wallet Balances");
    match client
        .portfolio()
        .get_balances(&GetBalancesParams::default())
        .await
    {
        Ok(balances) => {
            for b in &balances {
                println!("  [{}] {}", b.chain, b.wallet_address);
                println!(
                    "    {} ({})",
                    format_token(
                        b.native_balance.parse::<f64>().unwrap_or(0.0),
                        &b.native_symbol
                    ),
                    format_usd(b.native_balance_usd),
                );
            }
        }
        Err(e) => handle_error(e),
    }

    // ── PnL ────────────────────────────────────────────────────────────
    log_section("PnL Summary (30d)");
    match client
        .portfolio()
        .get_pnl(&GetPnlParams {
            timeframe: Some("30d".into()),
        })
        .await
    {
        Ok(pnl) => {
            println!("  Total Value       : {}", format_usd(pnl.total_value_usd));
            println!("  Total Bought      : {}", format_usd(pnl.total_bought_usd));
            println!("  Total Sold        : {}", format_usd(pnl.total_sold_usd));
            println!(
                "  Realized PnL      : {}",
                format_usd(pnl.total_realized_pnl_usd)
            );
            println!(
                "  Unrealized PnL    : {}",
                format_usd(pnl.total_unrealized_pnl_usd)
            );
            println!("  Total PnL         : {}", format_usd(pnl.total_pnl_usd));
            println!("  Open Positions    : {}", pnl.position_count);
        }
        Err(e) => handle_error(e),
    }

    // ── Positions ──────────────────────────────────────────────────────
    log_section("Open Positions");
    match client
        .portfolio()
        .get_positions(&GetPositionsParams::default())
        .await
    {
        Ok(positions) => {
            println!("  Count       : {}", positions.position_count);
            println!("  Total Value : {}", format_usd(positions.total_value_usd));

            let top: Vec<_> = positions.positions.iter().take(10).collect();
            let token_ids: Vec<String> = top
                .iter()
                .map(|p| format!("{}:{}", p.network, p.token_address))
                .collect();

            if !token_ids.is_empty() {
                match client.tokens().batch(&token_ids).await {
                    Ok(batch) => {
                        let lookup: std::collections::HashMap<&str, (&str, &str)> = batch
                            .tokens
                            .iter()
                            .map(|t| (t.address.as_str(), (t.name.as_str(), t.symbol.as_str())))
                            .collect();

                        for pos in &top {
                            let label = lookup
                                .get(pos.token_address.as_str())
                                .map(|(name, sym)| format!("{name} ({sym})"))
                                .unwrap_or_else(|| pos.token_address.clone());
                            println!("\n  {label}");
                            println!("    Address : {}", pos.token_address);
                            println!("    Wallet  : {}", pos.wallet_address);
                            println!("    Price   : ${:.8}", pos.latest_token_usd_price);
                        }
                    }
                    Err(_) => {
                        // Fall back to showing without names
                        for pos in &top {
                            println!("\n  {}", pos.token_address);
                            println!("    Wallet  : {}", pos.wallet_address);
                            println!("    Price   : ${:.8}", pos.latest_token_usd_price);
                        }
                    }
                }
            }
        }
        Err(e) => handle_error(e),
    }

    // ── Recent Trades ──────────────────────────────────────────────────
    log_section("Recent Trades (last 10)");
    match client
        .portfolio()
        .get_history(&GetHistoryParams {
            limit: Some(10),
            ..Default::default()
        })
        .await
    {
        Ok(trades) => {
            for t in &trades {
                let side = if t.is_buy { "BUY " } else { "SELL" };
                let time = {
                    // Simple timestamp formatting
                    let secs = t.timestamp;
                    let days = secs / 86400;
                    let rem = secs % 86400;
                    let hours = rem / 3600;
                    let mins = (rem % 3600) / 60;
                    let s = rem % 60;
                    // Approximate date from epoch
                    format!("{}d+{:02}:{:02}:{:02}", days, hours, mins, s)
                };
                println!(
                    "  {time}  {side}  {}  {}  ({})",
                    t.size_usd, t.token, t.chain
                );
            }
        }
        Err(e) => handle_error(e),
    }
}
