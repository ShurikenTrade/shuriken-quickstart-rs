/// 09 — Perp Trading
///
/// Place a limit order on Hyperliquid with take-profit and stop-loss,
/// inspect it, then cancel it.
///
/// WARNING: When DRY_RUN is set to false this places a REAL order
/// (then immediately cancels it). The limit price is set far from
/// market so it should not fill.
use shuriken_quickstart_rs::*;
use shuriken_sdk::perps::{
    CancelOrderParams, GetPerpAccountParams, GetPerpFeesParams, GetPerpOrdersParams,
    GetPerpPositionsParams, PlaceOrderParams, TpSlParams,
};

const DRY_RUN: bool = true;

#[tokio::main]
async fn main() {
    let client = create_http_client();
    let coin = "BTC";

    // ── We need a wallet ID for perps operations ──────────────────────
    let wallets = match client.account().get_wallets().await {
        Ok(w) => w,
        Err(e) => handle_error(e),
    };
    let wallet = wallets.first().unwrap_or_else(|| {
        eprintln!("No wallet found on your account");
        std::process::exit(1);
    });

    // ── Account state ─────────────────────────────────────────────────
    log_section("Perps Account");
    let account = match client
        .perps()
        .get_account(&GetPerpAccountParams::default())
        .await
    {
        Ok(a) => a,
        Err(e) => handle_error(e),
    };

    let account_value: f64 = account.account_value.parse().unwrap_or(0.0);
    let withdrawable: f64 = account.withdrawable.parse().unwrap_or(0.0);
    println!("  Account Value : {}", format_usd(account_value));
    println!("  Withdrawable  : {}", format_usd(withdrawable));

    // ── Fees ──────────────────────────────────────────────────────────
    let fees = match client.perps().get_fees(&GetPerpFeesParams::default()).await {
        Ok(f) => f,
        Err(e) => handle_error(e),
    };
    println!("  Maker Rate    : {}", fees.maker_rate);
    println!("  Taker Rate    : {}", fees.taker_rate);

    // ── Current market price ──────────────────────────────────────────
    let market = match client.perps().get_market(coin).await {
        Ok(m) => m,
        Err(e) => handle_error(e),
    };
    let mark_price: f64 = market.ctx.mark_px.parse().unwrap_or(0.0);
    println!("\n  {coin} Mark Price : {}", format_usd(mark_price));

    // ── Place a limit buy far below market (won't fill) ───────────────
    let limit_px = format!("{:.0}", mark_price * 0.5); // 50% below market
    let tp_px = format!("{:.0}", mark_price * 1.1); // TP at +10%
    let sl_px = format!("{:.0}", mark_price * 0.4); // SL at -20% from limit

    log_section(&format!(
        "Limit Buy: {coin} @ {}",
        format_usd(limit_px.parse::<f64>().unwrap_or(0.0))
    ));
    println!("  Size       : 0.001 {coin}");
    println!("  Limit      : {limit_px}");
    println!("  Take Profit: {tp_px}");
    println!("  Stop Loss  : {sl_px}");

    if DRY_RUN {
        println!();
        println!("  [DRY RUN] Would place limit buy order");
        println!("  Set DRY_RUN = false to execute for real");
    } else {
        let order_resp = match client
            .perps()
            .place_order(&PlaceOrderParams {
                wallet_id: wallet.wallet_id.clone(),
                coin: coin.into(),
                is_buy: true,
                sz: Some("0.001".into()),
                limit_px: Some(limit_px),
                order_type: Some("limit".into()),
                reduce_only: Some(false),
                tp: Some(TpSlParams {
                    trigger_px: tp_px,
                    is_market: None,
                    limit_px: None,
                }),
                sl: Some(TpSlParams {
                    trigger_px: sl_px,
                    is_market: None,
                    limit_px: None,
                }),
                ..Default::default()
            })
            .await
        {
            Ok(r) => r,
            Err(e) => handle_error(e),
        };

        println!("  Results:");
        for r in &order_resp.results {
            println!(
                "    Status : {}  OID : {}  {}",
                r.status,
                r.oid.map(|o| o.to_string()).unwrap_or("N/A".into()),
                r.error.as_deref().unwrap_or(""),
            );
        }

        // ── List open orders ──────────────────────────────────────────
        log_section("Open Orders");
        let orders = match client
            .perps()
            .get_orders(&GetPerpOrdersParams::default())
            .await
        {
            Ok(o) => o,
            Err(e) => handle_error(e),
        };

        if orders.is_empty() {
            println!("  No open orders");
        } else {
            for o in &orders {
                println!(
                    "  {}  {}  {} @ {}  OID={}  {}",
                    o.coin, o.side, o.sz, o.limit_px, o.oid, o.order_type,
                );
            }
        }

        // ── Cancel the order we just placed ───────────────────────────
        let placed = order_resp.results.iter().find(|r| r.oid.is_some());
        if let Some(p) = placed {
            let oid = p.oid.unwrap();
            log_section("Cancelling Order");
            let cancel_resp = match client
                .perps()
                .cancel_order(&CancelOrderParams {
                    wallet_id: wallet.wallet_id.clone(),
                    coin: coin.into(),
                    oid: Some(oid),
                    cloid: None,
                    cancel_all: None,
                })
                .await
            {
                Ok(r) => r,
                Err(e) => handle_error(e),
            };
            log_json("Cancel Result", &cancel_resp);
        }
    }

    // ── Positions (read-only, always runs) ────────────────────────────
    log_section("Current Positions");
    let positions = match client
        .perps()
        .get_positions(&GetPerpPositionsParams::default())
        .await
    {
        Ok(p) => p,
        Err(e) => handle_error(e),
    };

    if positions.positions.is_empty() {
        println!("  No open positions");
    } else {
        for p in &positions.positions {
            println!(
                "  {}  size={}  entry={}  pnl={}",
                p.coin, p.szi, p.entry_px, p.unrealized_pnl,
            );
        }
    }
}
