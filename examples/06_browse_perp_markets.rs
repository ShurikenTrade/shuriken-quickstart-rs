/// 06 — Browse Perp Markets
///
/// List all available perpetual markets on Hyperliquid and inspect
/// a single market's order book, funding rate, and metadata.
///
/// Pass a coin symbol as the first CLI argument, or it defaults to BTC.
use shuriken_quickstart_rs::*;

#[tokio::main]
async fn main() {
    let client = create_http_client();
    let coin = std::env::args().nth(1).unwrap_or("BTC".into());

    // ── All Markets ────────────────────────────────────────────────────
    log_section("Perpetual Markets");
    let markets = match client.perps().get_markets().await {
        Ok(m) => m,
        Err(e) => handle_error(e),
    };
    println!("  Total markets: {}\n", markets.len());

    // Show top 10 by volume
    let mut sorted = markets.clone();
    sorted.sort_by(|a, b| {
        let va: f64 = b.ctx.day_ntl_vlm.parse().unwrap_or(0.0);
        let vb: f64 = a.ctx.day_ntl_vlm.parse().unwrap_or(0.0);
        va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
    });

    println!("  Top 10 by 24h Volume:");
    println!(
        "  {:10}{:16}{:18}{:14}Max Lev",
        "Coin", "Price", "24h Volume", "Funding"
    );
    println!("  {}", "-".repeat(70));

    for m in sorted.iter().take(10) {
        let price = format_usd(m.ctx.mark_px.parse::<f64>().unwrap_or(0.0));
        let vol = format_usd(m.ctx.day_ntl_vlm.parse::<f64>().unwrap_or(0.0));
        let funding = format!(
            "{:.4}%",
            m.ctx.funding.parse::<f64>().unwrap_or(0.0) * 100.0
        );
        println!(
            "  {:10}{:16}{:18}{:14}{}x",
            m.meta.name, price, vol, funding, m.meta.max_leverage,
        );
    }

    // ── Single Market Deep-Dive ────────────────────────────────────────
    log_section(&format!("Market Detail: {coin}"));
    let market = match client.perps().get_market(&coin).await {
        Ok(m) => m,
        Err(e) => handle_error(e),
    };

    println!("  Name          : {}", market.meta.name);
    println!("  Max Leverage  : {}x", market.meta.max_leverage);
    println!("  Size Decimals : {}", market.meta.sz_decimals);
    println!("  Only Isolated : {}", market.meta.only_isolated);
    println!(
        "  Mark Price    : {}",
        format_usd(market.ctx.mark_px.parse::<f64>().unwrap_or(0.0))
    );
    println!(
        "  Oracle Price  : {}",
        format_usd(market.ctx.oracle_px.parse::<f64>().unwrap_or(0.0))
    );
    println!(
        "  24h Volume    : {}",
        format_usd(market.ctx.day_ntl_vlm.parse::<f64>().unwrap_or(0.0))
    );
    println!("  Open Interest : {}", market.ctx.open_interest);
    println!(
        "  Funding Rate  : {:.4}%",
        market.ctx.funding.parse::<f64>().unwrap_or(0.0) * 100.0
    );

    // ── Order Book Snapshot ────────────────────────────────────────────
    log_section("Order Book (top 5)");
    println!("  {:40}BIDS", "ASKS");
    println!(
        "  {:16}{:14}{:10}{:16}{:14}Orders",
        "Price", "Size", "Orders", "Price", "Size"
    );
    println!("  {}", "-".repeat(70));

    for i in 0..5 {
        let ask_str = market
            .asks
            .get(i)
            .map(|a| format!("{:16}{:14}{:10}", a.price, a.size, a.num_orders.to_string()))
            .unwrap_or_else(|| " ".repeat(40));
        let bid_str = market
            .bids
            .get(i)
            .map(|b| format!("{:16}{:14}{}", b.price, b.size, b.num_orders))
            .unwrap_or_default();
        println!("  {ask_str}{bid_str}");
    }
}
