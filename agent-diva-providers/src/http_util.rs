//! Shared `reqwest::Client` options for provider HTTP calls.

use reqwest::Client;
use std::time::Duration;

/// Local gateways and plain `http://` bases are often happier with HTTP/1.1 only.
/// Remote `https://` APIs (e.g. DeepSeek) should use normal ALPN so the peer does not RST during TLS.
pub(crate) fn should_force_http1_only_for_api_base(api_base: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(api_base.trim()) else {
        return true;
    };
    match url.scheme() {
        "http" => true,
        "https" => url
            .host_str()
            .map(|host| {
                let h = host.to_ascii_lowercase();
                h == "localhost" || h == "127.0.0.1" || h == "::1" || h.ends_with(".local")
            })
            .unwrap_or(true),
        _ => true,
    }
}

pub(crate) fn build_api_http_client(
    api_base: &str,
    request_timeout: Duration,
) -> reqwest::Result<Client> {
    let mut builder = Client::builder()
        .connect_timeout(Duration::from_secs(45))
        .timeout(request_timeout);
    if should_force_http1_only_for_api_base(api_base) {
        builder = builder.http1_only();
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deepseek_https_public_no_force_http1() {
        assert!(!should_force_http1_only_for_api_base(
            "https://api.deepseek.com/v1"
        ));
    }

    #[test]
    fn local_litellm_https_forces_http1() {
        assert!(should_force_http1_only_for_api_base(
            "https://127.0.0.1:4000/v1"
        ));
    }

    #[test]
    fn plain_http_forces_http1() {
        assert!(should_force_http1_only_for_api_base(
            "http://localhost:4000"
        ));
    }
}
