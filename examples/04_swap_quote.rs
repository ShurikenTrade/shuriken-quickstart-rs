/// 04 — Swap Quote
///
/// Get a swap quote without executing it. This is completely read-only
/// and safe to run -- no funds are moved.
///
/// Demonstrates get_quote() with route and fee breakdown.
use shuriken_quickstart_rs::*;
use shuriken_sdk::swap::GetSwapQuoteParams;

const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const JUP: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

#[tokio::main]
async fn main() {
    let client = create_http_client();

    // ── Get quote: 1 USDC -> JUP on Solana ─────────────────────────────
    log_section("Swap Quote: 1 USDC -> JUP");
    let quote = match client
        .swap()
        .get_quote(&GetSwapQuoteParams {
            chain: "solana".into(),
            input_mint: USDC.into(),
            output_mint: JUP.into(),
            amount: "1000000".into(), // 1 USDC (6 decimals)
            slippage_bps: Some(50),
        })
        .await
    {
        Ok(q) => q,
        Err(e) => handle_error(e),
    };

    println!("  Quote ID     : {}", quote.quote_id);
    println!("  Chain        : {}", quote.chain);
    println!("  In           : {} (raw)", quote.in_amount);
    println!("  Out          : {} (raw)", quote.out_amount);
    println!("  Slippage     : {} bps", quote.slippage_bps);
    println!(
        "  Price Impact : {}",
        quote.price_impact_pct.as_deref().unwrap_or("N/A")
    );
    println!("  Expires At   : {}", quote.expires_at);

    // ── Fee breakdown ──────────────────────────────────────────────────
    log_section("Fees");
    println!(
        "  Platform Fee : {} ({} bps)",
        quote.fees.platform_fee_amount.as_deref().unwrap_or("0"),
        quote.fees.platform_fee_bps.unwrap_or(0),
    );
    println!(
        "  DEX Fee      : {} (native)",
        quote.fees.dex_fee_in_native.as_deref().unwrap_or("0"),
    );

    // ── Routes ─────────────────────────────────────────────────────────
    log_section("Routes");
    for (i, route) in quote.routes.iter().enumerate() {
        println!("  Route {}: {}", i + 1, route.source);
        println!("    In  : {}", route.in_amount.as_deref().unwrap_or("N/A"));
        println!("    Out : {}", route.out_amount.as_deref().unwrap_or("N/A"));
    }
}
