/// 02 — Search Tokens
///
/// Search for tokens by name or symbol across all supported chains.
/// Demonstrates the tokens.search() endpoint with optional chain filtering.
use shuriken_quickstart_rs::*;
use shuriken_sdk::tokens::SearchTokensParams;

#[tokio::main]
async fn main() {
    let client = create_http_client();

    // ── Search across all chains ───────────────────────────────────────
    log_section("Search: 'bonk' (all chains)");
    let results = match client
        .tokens()
        .search(&SearchTokensParams {
            q: "bonk".into(),
            chain: None,
            page: None,
            limit: Some(5),
        })
        .await
    {
        Ok(r) => r,
        Err(e) => handle_error(e),
    };

    for t in &results {
        println!(
            "  {:10} {:30} {:10} {}",
            t.symbol, t.name, t.chain, t.address
        );
    }

    // ── Search on Solana only ──────────────────────────────────────────
    log_section("Search: 'usdc' (solana only)");
    match client
        .tokens()
        .search(&SearchTokensParams {
            q: "usdc".into(),
            chain: Some("solana".into()),
            page: None,
            limit: Some(5),
        })
        .await
    {
        Ok(sol_results) => {
            for t in &sol_results {
                println!("  {:10} {:30} {}", t.symbol, t.name, t.token_id);
            }
        }
        Err(e) => handle_error(e),
    }

    // ── Get full token info ────────────────────────────────────────────
    if let Some(first) = results.first() {
        log_section(&format!("Token Details: {}", first.symbol));
        match client.tokens().get(&first.token_id).await {
            Ok(info) => {
                println!("  Token ID : {}", info.token_id);
                println!("  Name     : {}", info.name);
                println!("  Symbol   : {}", info.symbol);
                println!("  Chain    : {}", info.chain);
                println!("  Decimals : {}", info.decimals);
            }
            Err(e) => handle_error(e),
        }

        match client.tokens().get_price(&first.token_id).await {
            Ok(price) => {
                let usd = price.price_usd.unwrap_or(0.0);
                println!("  Price    : {}", format_usd(usd));
            }
            Err(e) => handle_error(e),
        }
    }
}
