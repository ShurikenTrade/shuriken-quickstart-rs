use shuriken_sdk::{ShurikenError, ShurikenHttpClient, ShurikenWsClient};

const LABS_URL: &str = "https://app.shuriken.trade/agents";

pub fn create_http_client() -> ShurikenHttpClient {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("SHURIKEN_API_KEY").unwrap_or_else(|_| {
        eprintln!("Missing SHURIKEN_API_KEY — copy .env.example to .env and add your key");
        eprintln!("Create one at: {LABS_URL}");
        std::process::exit(1);
    });

    let client = match std::env::var("SHURIKEN_API_URL") {
        Ok(url) => ShurikenHttpClient::with_base_url(&api_key, &url),
        Err(_) => ShurikenHttpClient::new(&api_key),
    };

    client.unwrap_or_else(|e| {
        eprintln!("Failed to create client: {e}");
        std::process::exit(1);
    })
}

pub fn create_ws_client() -> ShurikenWsClient {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("SHURIKEN_API_KEY").unwrap_or_else(|_| {
        eprintln!("Missing SHURIKEN_API_KEY — copy .env.example to .env and add your key");
        eprintln!("Create one at: {LABS_URL}");
        std::process::exit(1);
    });

    let client = match std::env::var("SHURIKEN_API_URL") {
        Ok(url) => ShurikenWsClient::with_base_url(&api_key, &url),
        Err(_) => ShurikenWsClient::new(&api_key),
    };

    client.unwrap_or_else(|e| {
        eprintln!("Failed to create WS client: {e}");
        std::process::exit(1);
    })
}

pub fn format_usd(value: f64) -> String {
    format!("${value:.2}")
}

pub fn format_token(value: f64, symbol: &str) -> String {
    if symbol.is_empty() {
        format!("{value:.6}")
    } else {
        format!("{value:.6} {symbol}")
    }
}

pub fn format_pct(value: f64) -> String {
    let sign = if value >= 0.0 { "+" } else { "" };
    format!("{sign}{value:.2}%")
}

pub fn log_section(title: &str) {
    println!("\n{}", "=".repeat(60));
    println!("  {title}");
    println!("{}", "=".repeat(60));
}

pub fn log_json(label: &str, data: &impl serde::Serialize) {
    println!("\n--- {label} ---");
    println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
}

pub fn handle_error(err: ShurikenError) -> ! {
    match &err {
        ShurikenError::Auth(_) => {
            eprintln!("\nAuthentication failed — your API key is missing or invalid.");
            eprintln!("Create or rotate your key at: {LABS_URL}");
        }
        _ => eprintln!("{err}"),
    }
    std::process::exit(1);
}
