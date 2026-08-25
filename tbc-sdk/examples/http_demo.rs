//! CLI demo: login + move via tbc-sdk HTTP client.

use tbc_sdk::TbcHttpClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = std::env::var("TBC_URL").unwrap_or_else(|_| "http://127.0.0.1:6014".into());
    let client = TbcHttpClient::new(base);
    let status = client.status().await?;
    println!("Server tick={} ruleset={}", status.tick, status.ruleset);

    let login = client.login().await?;
    println!(
        "Login iuoc={} fwau={} frame={}",
        login.iuoc, login.fwau, login.frame
    );

    let snap = client.move_player(login.fwau, 1.0, 0.0, Some(1)).await?;
    println!(
        "Move snapshot tick={} entities={}",
        snap.get("tick").and_then(|t| t.as_u64()).unwrap_or(0),
        snap.get("entities")
            .and_then(|e| e.as_array())
            .map(|a| a.len())
            .unwrap_or(0)
    );

    Ok(())
}
