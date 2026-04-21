/// 18 — Perps Hedger
///
/// Reads your spot portfolio positions and opens opposing perpetual
/// positions to delta-hedge. Shows the net exposure before and after.
///
/// WARNING: Set DRY_RUN=false to actually open perp positions.
use shuriken_quickstart_rs::*;
use shuriken_sdk::perps::{GetPerpPositionsParams, PlaceOrderParams};
use shuriken_sdk::portfolio::GetPositionsParams;

const DRY_RUN: bool = true;

// Map of spot token addresses to perp coin symbols
const SPOT_TO_PERP: &[(&str, &str)] = &[
    ("So11111111111111111111111111111111111111112", "SOL"),
    // Add more mappings as needed
];

fn perp_coin_for(token_address: &str) -> Option<&'static str> {
    SPOT_TO_PERP
        .iter()
        .find(|(addr, _)| *addr == token_address)
        .map(|(_, coin)| *coin)
}

#[tokio::main]
async fn main() {
    let client = create_http_client();

    let wallets = match client.account().get_wallets().await {
        Ok(w) => w,
        Err(e) => handle_error(e),
    };
    let wallet = wallets.first().unwrap_or_else(|| {
        eprintln!("No wallet found on your account");
        std::process::exit(1);
    });

    // ── Spot positions ───────────────────────────────────────────────
    log_section("Spot Portfolio");
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

    struct Hedgeable {
        coin: &'static str,
        spot_value_usd: f64,
    }

    let mut hedgeable = Vec::new();

    for pos in &positions.positions {
        let balance: f64 =
            pos.latest_balance_raw.parse().unwrap_or(0.0) / 10f64.powi(pos.token_decimal as i32);
        let value = pos.latest_token_usd_price * balance;
        let perp_coin = perp_coin_for(&pos.token_address);

        println!(
            "  {}...  Value: {}  Perp: {}",
            &pos.token_address[..pos.token_address.len().min(12)],
            format_usd(value),
            perp_coin.unwrap_or("N/A"),
        );

        if let Some(coin) = perp_coin {
            if value > 1.0 {
                hedgeable.push(Hedgeable {
                    coin,
                    spot_value_usd: value,
                });
            }
        }
    }

    if hedgeable.is_empty() {
        println!("\n  No hedgeable positions found (need spot tokens with matching perp markets)");
        return;
    }

    // ── Current perp positions ───────────────────────────────────────
    log_section("Current Perp Positions");
    let perp_positions = match client
        .perps()
        .get_positions(&GetPerpPositionsParams::default())
        .await
    {
        Ok(p) => p,
        Err(e) => handle_error(e),
    };

    let mut existing_perps = std::collections::HashMap::new();
    for p in &perp_positions.positions {
        let szi: f64 = p.szi.parse().unwrap_or(0.0);
        let entry: f64 = p.entry_px.parse().unwrap_or(0.0);
        let notional = szi * entry;
        existing_perps.insert(p.coin.as_str(), notional);
        println!(
            "  {}  size={}  notional={}",
            p.coin,
            p.szi,
            format_usd(notional.abs()),
        );
    }
    if perp_positions.positions.is_empty() {
        println!("  No open perp positions");
    }

    // ── Calculate hedge orders ───────────────────────────────────────
    log_section("Hedge Plan");
    println!(
        "  {:<8}{:<16}{:<16}{:<16}Action",
        "Coin", "Spot Long", "Perp Short", "Net Exposure",
    );
    println!("  {}", "-".repeat(64));

    for h in &hedgeable {
        let existing_short = existing_perps.get(h.coin).copied().unwrap_or(0.0);
        let net_exposure = h.spot_value_usd + existing_short;

        let action = if net_exposure.abs() < 1.0 {
            "HEDGED".to_string()
        } else {
            format!("SHORT {}", format_usd(net_exposure))
        };

        println!(
            "  {:<8}{:<16}{:<16}{:<16}{}",
            h.coin,
            format_usd(h.spot_value_usd),
            format_usd(existing_short.abs()),
            format_usd(net_exposure),
            action,
        );

        if net_exposure.abs() >= 1.0 && !DRY_RUN {
            match client
                .perps()
                .place_order(&PlaceOrderParams {
                    wallet_id: wallet.wallet_id.clone(),
                    coin: h.coin.into(),
                    is_buy: false, // short
                    size_usd: Some(format!("{:.0}", net_exposure)),
                    order_type: Some("market".into()),
                    ..Default::default()
                })
                .await
            {
                Ok(resp) => {
                    for r in &resp.results {
                        println!(
                            "    -> Order placed: {} OID={}",
                            r.status,
                            r.oid.map(|o| o.to_string()).unwrap_or_else(|| "N/A".into()),
                        );
                    }
                }
                Err(e) => println!("    -> Error placing order: {e}"),
            }
        }
    }

    if DRY_RUN {
        println!("\n  [DRY RUN] No orders placed. Set DRY_RUN=false to execute.");
    }
}
