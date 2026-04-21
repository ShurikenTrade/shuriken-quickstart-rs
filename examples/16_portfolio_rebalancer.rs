/// 16 — Portfolio Rebalancer
///
/// Fetches your current portfolio positions and compares them to target
/// allocation percentages. For each token that's overweight or underweight,
/// it generates the swap quotes needed to rebalance.
///
/// This is read-only -- it only generates quotes, never executes.
use shuriken_quickstart_rs::*;
use shuriken_sdk::portfolio::GetPositionsParams;
use shuriken_sdk::swap::GetSwapQuoteParams;

const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const JUP_MINT: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

struct TargetAllocation {
    address: &'static str,
    symbol: &'static str,
    target_pct: f64,
}

const TARGETS: &[TargetAllocation] = &[
    TargetAllocation {
        address: SOL_MINT,
        symbol: "SOL",
        target_pct: 50.0,
    },
    TargetAllocation {
        address: USDC_MINT,
        symbol: "USDC",
        target_pct: 30.0,
    },
    TargetAllocation {
        address: JUP_MINT,
        symbol: "JUP",
        target_pct: 20.0,
    },
];

#[tokio::main]
async fn main() {
    let client = create_http_client();

    // ── Fetch current positions ──────────────────────────────────────
    log_section("Current Portfolio");
    let positions = match client
        .portfolio()
        .get_positions(&GetPositionsParams {
            chain: Some("solana".into()),
            ..Default::default()
        })
        .await
    {
        Ok(p) => p,
        Err(e) => handle_error(e),
    };

    let total_value = positions.total_value_usd;
    println!("  Total Value: {}", format_usd(total_value));

    // Build current allocation map: token_address -> (value_usd, current_pct)
    let mut current_alloc = std::collections::HashMap::new();
    for pos in &positions.positions {
        let balance: f64 =
            pos.latest_balance_raw.parse().unwrap_or(0.0) / 10f64.powi(pos.token_decimal as i32);
        let value = pos.latest_token_usd_price * balance;
        let current_pct = if total_value > 0.0 {
            (value / total_value) * 100.0
        } else {
            0.0
        };
        current_alloc.insert(pos.token_address.as_str(), (value, current_pct));
    }

    // ── Compare to targets ───────────────────────────────────────────
    log_section("Allocation Comparison");
    println!(
        "  {:<10}{:<14}{:<14}{:<14}Action",
        "Token", "Current %", "Target %", "Diff",
    );
    println!("  {}", "-".repeat(60));

    struct RebalanceAction {
        from: String,
        symbol: String,
        amount_usd: f64,
    }

    let mut rebalance_actions = Vec::new();

    for target in TARGETS {
        let (_, current_pct) = current_alloc
            .get(target.address)
            .copied()
            .unwrap_or((0.0, 0.0));
        let diff_pct = current_pct - target.target_pct;
        let diff_usd = (diff_pct / 100.0) * total_value;

        let action = if diff_pct.abs() > 2.0 {
            if diff_pct > 0.0 {
                format!("SELL {}", format_usd(diff_usd.abs()))
            } else {
                format!("BUY {}", format_usd(diff_usd.abs()))
            }
        } else {
            "OK".into()
        };

        println!(
            "  {:<10}{:>6.1}%{:<7}{:>6.1}%{:<7}{:<14}{}",
            target.symbol,
            current_pct,
            "",
            target.target_pct,
            "",
            format_pct(diff_pct),
            action,
        );

        if diff_pct > 2.0 {
            rebalance_actions.push(RebalanceAction {
                from: target.address.to_string(),
                symbol: target.symbol.to_string(),
                amount_usd: diff_usd.abs(),
            });
        }
    }

    // ── Generate rebalance quotes ────────────────────────────────────
    if rebalance_actions.is_empty() {
        log_section("Portfolio is balanced (within 2% tolerance)");
        return;
    }

    log_section("Rebalance Quotes");
    for action in &rebalance_actions {
        let token_id = format!("solana:{}", action.from);

        let price = match client.tokens().get_price(&token_id).await {
            Ok(p) => p,
            Err(e) => {
                println!("\n  Error getting price for {}: {e}", action.symbol);
                continue;
            }
        };

        let price_usd = match price.price_usd {
            Some(p) if p > 0.0 => p,
            _ => {
                println!("\n  No price available for {}", action.symbol);
                continue;
            }
        };

        let info = match client.tokens().get(&token_id).await {
            Ok(i) => i,
            Err(e) => {
                println!("\n  Error getting token info for {}: {e}", action.symbol);
                continue;
            }
        };

        let token_amount = action.amount_usd / price_usd;
        let raw_amount = (token_amount * 10f64.powi(info.decimals as i32)) as u64;

        match client
            .swap()
            .get_quote(&GetSwapQuoteParams {
                chain: "solana".into(),
                input_mint: action.from.clone(),
                output_mint: USDC_MINT.into(),
                amount: raw_amount.to_string(),
                slippage_bps: Some(100),
            })
            .await
        {
            Ok(quote) => {
                println!("\n  Sell {} -> USDC", action.symbol);
                println!("    In       : {} (raw)", quote.in_amount);
                println!("    Out      : {} (raw USDC)", quote.out_amount);
                println!(
                    "    Impact   : {}",
                    quote.price_impact_pct.as_deref().unwrap_or("N/A"),
                );
                println!("    Quote ID : {}", quote.quote_id);
            }
            Err(e) => {
                println!("\n  Error quoting {}: {e}", action.symbol);
            }
        }
    }

    println!(
        "\n  To execute these rebalances, use client.swap().execute() with the appropriate parameters."
    );
}
