use std::time::Duration;

#[tokio::main]
async fn main() {
    // Test the same way GUI does
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("reqwest client");

    let url = "http://127.0.0.1:3000/api/health";
    println!("Testing GUI-style connection to: {}", url);
    println!("Using no_proxy client (like official code)");

    match client.get(url).send().await {
        Ok(response) => {
            println!("✅ Response status: {}", response.status());
            if response.status().is_success() {
                match response.text().await {
                    Ok(body) => println!("✅ Body: {}", body),
                    Err(e) => println!("❌ Failed to read body: {}", e),
                }
            } else {
                println!("❌ Non-success status");
            }
        }
        Err(e) => println!("❌ Request failed: {}", e),
    }
}
