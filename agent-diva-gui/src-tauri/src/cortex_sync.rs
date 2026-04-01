//! 皮层与 **本机 gateway** 对齐（Story 2.2 / 6.4 / NFR-R1）。
//!
//! # 契约语义
//!
//! - **网关可达**（`/api/health` 成功）：写入前 **必须** `POST /api/swarm/cortex` 成功后才可更新桌面壳内
//!   [`agent_diva_swarm::CortexRuntime`]，与 gateway 内 `ProcessEventPipeline` 门控一致。
//! - **网关不可达**：视为无同伴进程（如调试外置后端或未启动），跳过远端握手，仅更新本地 `Arc`（与既有离线开发路径兼容）。
//! - **故障模拟：** 在 **`cfg(test)` 或 `debug_assertions`** 下若 `AGENT_DIVA_TEST_CORTEX_SYNC_FAIL=1`，则 **在 HTTP 之前**
//!   返回 [`ERR_CORTEX_SYNC_REJECTED`]。Release 正式构建忽略该变量。
//!
//! # 查询
//!
//! [`try_pull_cortex_from_gateway`] 在网关健康时 `GET /api/swarm/cortex` 并解析快照，供 `get_cortex_state` 将本地 `Arc` 与权威源对齐。

use agent_diva_swarm::CortexState;
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, error};

/// 稳定机器可读错误码：`invoke` 失败时 `Err` 字符串等于此值，前端可映射 i18n（勿向用户直接暴露内部栈）。
pub const ERR_CORTEX_SYNC_REJECTED: &str = "cortex_sync_rejected";

const TEST_FAIL_ENV: &str = "AGENT_DIVA_TEST_CORTEX_SYNC_FAIL";

const HTTP_TIMEOUT: Duration = Duration::from_secs(3);

fn health_url(api_base: &str) -> String {
    format!("{}/health", api_base.trim_end_matches('/'))
}

fn cortex_url(api_base: &str) -> String {
    format!("{}/swarm/cortex", api_base.trim_end_matches('/'))
}

/// 本机 companion gateway 是否响应健康检查。
pub async fn gateway_health_ok(client: &Client, api_base: &str) -> bool {
    let url = health_url(api_base);
    match client.get(&url).timeout(HTTP_TIMEOUT).send().await {
        Ok(r) => r.status().is_success(),
        Err(_) => false,
    }
}

/// 网关健康时拉取皮层快照；不可达或非 2xx 时返回 `None`（调用方保留本地状态）。
pub async fn try_pull_cortex_from_gateway(client: &Client, api_base: &str) -> Option<CortexState> {
    if !gateway_health_ok(client, api_base).await {
        return None;
    }
    let url = cortex_url(api_base);
    let resp = client.get(&url).timeout(HTTP_TIMEOUT).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.json::<CortexState>().await.ok()
}

/// 在提交皮层目标状态 **之前** 调用；失败时 **不得** 修改桌面壳 `CortexRuntime`。
pub async fn sync_cortex_before_gateway_commit(
    client: &Client,
    api_base: &str,
    target_enabled: bool,
    op_label: &str,
) -> Result<(), String> {
    #[cfg(any(test, debug_assertions))]
    if std::env::var(TEST_FAIL_ENV).ok().as_deref() == Some("1") {
        error!(
            target_enabled,
            op = op_label,
            kind = "cortex_gateway_sync",
            "cortex gateway sync failed (injected)"
        );
        return Err(ERR_CORTEX_SYNC_REJECTED.to_string());
    }

    if !gateway_health_ok(client, api_base).await {
        debug!(
            target: "agent_diva_gui::cortex",
            op = op_label,
            "gateway unreachable; cortex commit local-only"
        );
        return Ok(());
    }

    let url = cortex_url(api_base);
    let resp = client
        .post(&url)
        .timeout(HTTP_TIMEOUT)
        .json(&serde_json::json!({ "enabled": target_enabled }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        return Ok(());
    }
    error!(
        status = %resp.status(),
        op = op_label,
        kind = "cortex_gateway_sync",
        "gateway rejected cortex set"
    );
    Err(ERR_CORTEX_SYNC_REJECTED.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn injected_fail_returns_stable_code() {
        let client = Client::new();
        std::env::set_var(TEST_FAIL_ENV, "1");
        let r = sync_cortex_before_gateway_commit(&client, "http://127.0.0.1:9/api", true, "unit_test").await;
        std::env::remove_var(TEST_FAIL_ENV);
        assert_eq!(r.unwrap_err(), ERR_CORTEX_SYNC_REJECTED);
    }

    #[tokio::test]
    #[serial]
    async fn without_injection_unreachable_gateway_ok() {
        std::env::remove_var(TEST_FAIL_ENV);
        let client = Client::new();
        let r = sync_cortex_before_gateway_commit(&client, "http://127.0.0.1:9/api", false, "unit_test").await;
        assert!(r.is_ok());
    }
}
