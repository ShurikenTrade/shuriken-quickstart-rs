/// 08 — Trigger Orders
///
/// Interactively create a conditional trigger order. You pick the
/// token pair, amount, direction, and trigger price.
///
/// WARNING: This creates a REAL trigger order that stays active until
/// triggered, cancelled, or expired.
use shuriken_quickstart_rs::*;
use shuriken_sdk::portfolio::GetBalancesParams;
use shuriken_sdk::tokens::SearchTokensParams;
use shuriken_sdk::trigger::CreateTriggerOrderParams;
use shuriken_sdk::ShurikenError;

const SOL: &str = "So11111111111111111111111111111111111111112";

#[tokio::main]
async fn main() {
    let client = create_http_client();

    // ── Pick a wallet ─────────────────────────────────────────────────
    let wallets = match client.account().get_wallets().await {
        Ok(w) => w,
        Err(e) => handle_error(e),
    };

    let sol_wallets: Vec<_> = wallets
        .iter()
        .filter(|w| w.chain.as_deref() == Some("solana") || w.chain.is_none())
        .collect();

    if sol_wallets.is_empty() {
        eprintln!("No Solana wallet found on your account");
        std::process::exit(1);
    }

    let balances = client
        .portfolio()
        .get_balances(&GetBalancesParams {
            chain: Some("solana".into()),
        })
        .await
        .unwrap_or_default();

    let wallet = if sol_wallets.len() > 1 {
        println!("\nAvailable Solana wallets:");
        for (i, w) in sol_wallets.iter().enumerate() {
            let label = w.label.as_deref().unwrap_or(&w.wallet_id);
            let bal = balances
                .iter()
                .find(|b| b.wallet_address == w.address)
                .map(|b| format!("{} SOL", b.native_balance))
                .unwrap_or_else(|| "unknown balance".into());
            println!("  [{}] {label} ({}) — {bal}", i + 1, w.address);
        }
        let idx = choose(
            &format!("\nSelect wallet (1-{}):  ", sol_wallets.len()),
            sol_wallets.len(),
        );
        sol_wallets[idx]
    } else {
        sol_wallets[0]
    };

    let label = wallet.label.as_deref().unwrap_or(&wallet.wallet_id);
    println!("\nUsing wallet: {label} ({})", wallet.address);

    // ── Pick a token ──────────────────────────────────────────────────
    let query = prompt_non_empty("\nSearch for a token (name, symbol, or address):  ");
    let results = match client
        .tokens()
        .search(&SearchTokensParams {
            q: query,
            chain: Some("solana".into()),
            limit: Some(5),
            page: None,
        })
        .await
    {
        Ok(r) => r,
        Err(e) => handle_error(e),
    };

    if results.is_empty() {
        println!("No tokens found. Aborted.");
        return;
    }

    println!("\nResults:");
    for (i, t) in results.iter().enumerate() {
        let addr_short = &t.address[..8.min(t.address.len())];
        println!("  [{}] {} ({}) — {addr_short}...", i + 1, t.name, t.symbol);
    }

    let token_idx = choose(
        &format!("\nSelect token (1-{}):  ", results.len()),
        results.len(),
    );
    let token = &results[token_idx];

    // ── Show current price ────────────────────────────────────────────
    let price = match client
        .tokens()
        .get_price(&format!("solana:{}", token.address))
        .await
    {
        Ok(p) => p,
        Err(e) => handle_error(e),
    };

    let price_usd = price.price_usd.unwrap_or(0.0);
    println!("\n  Current price of {}: ${price_usd:.8}", token.symbol);

    // ── Order parameters ──────────────────────────────────────────────
    println!("\n  Trigger direction:");
    println!(
        "    'below' = buy the dip — swap SOL → {} when price drops to your target",
        token.symbol
    );
    println!(
        "    'above' = take profit — swap SOL → {} when price rises to your target",
        token.symbol
    );

    let direction = loop {
        let input = prompt("\nTrigger direction — 'above' or 'below':  ");
        if input == "above" || input == "below" {
            break input;
        }
        println!("  Please enter 'above' or 'below'.");
    };

    let trigger_price = loop {
        let input = prompt("Trigger price (USD):  ");
        if let Ok(v) = input.parse::<f64>() {
            if v > 0.0 {
                break input;
            }
        }
        println!("  Please enter a valid price greater than 0.");
    };

    let (amount_f64, amount_display) = loop {
        let input = prompt("Amount of SOL to swap:  ");
        if let Ok(v) = input.parse::<f64>() {
            if v > 0.0 {
                break (v, input);
            }
        }
        println!("  Please enter a valid amount greater than 0.");
    };
    let amount_lamports = format!("{}", (amount_f64 * 1e9) as u64);

    // ── Confirm ───────────────────────────────────────────────────────
    println!("\n  Order summary:");
    println!("    Swap    : {amount_display} SOL → {}", token.symbol);
    println!(
        "    When    : {} price goes {direction} ${trigger_price}",
        token.symbol
    );
    println!("    Current : ${price_usd:.8}");

    if !confirm("\n⚠️  This will create a REAL trigger order. Type 'yes' to continue:  ") {
        println!("Aborted.");
        return;
    }

    // ── Create the order ──────────────────────────────────────────────
    log_section("Creating Trigger Order");
    let order_params = CreateTriggerOrderParams {
        chain: "solana".into(),
        input_token: SOL.into(),
        output_token: token.address.clone(),
        amount: amount_lamports,
        wallet_id: wallet.wallet_id.clone(),
        trigger_metric: "price_usd".into(),
        trigger_direction: direction,
        trigger_value: Some(trigger_price),
        ..Default::default()
    };

    let order = match client.trigger().create(&order_params).await {
        Ok(o) => o,
        Err(ShurikenError::Api { ref response, .. })
            if response.error.code == "NONCE_NOT_INITIALIZED" =>
        {
            println!("\n  This wallet does not have durable nonce initialized.");
            println!("  Trigger orders on Solana require this feature.");
            println!("  Enabling it is an on-chain action and incurs a small SOL fee.");

            if !confirm("\n  Enable multisend for this wallet? Type 'yes' to continue:  ") {
                println!("Aborted.");
                return;
            }

            println!("  Enabling multisend...");
            let task_id = match client.account().enable_multisend(&wallet.wallet_id).await {
                Ok(resp) => resp.task_id,
                Err(e) => {
                    eprintln!("  Failed to enable multisend: {e}");
                    return;
                }
            };

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                match client.tasks().get_status(&task_id).await {
                    Ok(s) => {
                        println!("  Multisend status: {}", s.status);
                        match s.status.as_str() {
                            "pending" => continue,
                            "success" => {
                                println!("  Multisend enabled successfully.");
                                break;
                            }
                            _ => {
                                eprintln!(
                                    "  Multisend task failed: {}",
                                    s.error_message.as_deref().unwrap_or(&s.status)
                                );
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("  Task poll failed: {e}");
                        return;
                    }
                }
            }

            // Retry creating the order
            match client.trigger().create(&order_params).await {
                Ok(o) => o,
                Err(e) => handle_error(e),
            }
        }
        Err(e) => handle_error(e),
    };

    println!("  Order ID  : {}", order.order_id);
    println!("  Status    : {}", order.status);
    println!(
        "  Trigger   : {} {} {}",
        token.symbol,
        order.trigger.direction,
        order.trigger.value.as_deref().unwrap_or("N/A")
    );

    // ── List orders ───────────────────────────────────────────────────
    log_section("Your Trigger Orders");
    let list = match client
        .trigger()
        .list(&shuriken_sdk::trigger::ListTriggerOrdersParams {
            limit: Some(10),
            cursor: None,
        })
        .await
    {
        Ok(l) => l,
        Err(e) => handle_error(e),
    };

    // Batch-fetch token names
    let addresses: Vec<String> = list
        .orders
        .iter()
        .flat_map(|o| vec![o.input_token.clone(), o.output_token.clone()])
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .map(|a| format!("solana:{a}"))
        .collect();

    let token_lookup: std::collections::HashMap<String, String> = if !addresses.is_empty() {
        client
            .tokens()
            .batch(&addresses)
            .await
            .map(|b| {
                b.tokens
                    .into_iter()
                    .map(|t| (t.address, t.symbol))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Default::default()
    };

    for o in &list.orders {
        let input = token_lookup
            .get(&o.input_token)
            .cloned()
            .unwrap_or_else(|| o.input_token[..8.min(o.input_token.len())].to_string());
        let output = token_lookup
            .get(&o.output_token)
            .cloned()
            .unwrap_or_else(|| o.output_token[..8.min(o.output_token.len())].to_string());
        let trigger = o
            .trigger
            .as_ref()
            .map(|t| {
                format!(
                    "{} {} {}",
                    t.metric,
                    t.direction,
                    t.value.as_deref().unwrap_or("trailing")
                )
            })
            .unwrap_or_else(|| "N/A".into());
        println!(
            "  {}  {:10}  {input} → {output}  {trigger}",
            o.order_id, o.status
        );
    }
}
