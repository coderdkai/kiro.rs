//! AWS OIDC Device Flow 代理
//!
//! 实现 OIDC 客户端注册、设备授权和 Token 轮询，
//! 用于 Admin UI 的"设备登录"功能。

use anyhow::{bail, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::http_client::{ProxyConfig, build_client};
use crate::model::config::{Config, TlsBackend};

use super::error::AdminServiceError;

// ============ OIDC JSON 类型（与 AWS API 对齐） ============

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OidcRegisterRequest {
    client_name: &'static str,
    client_type: &'static str,
    issuer_url: String,
    scopes: &'static [&'static str],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OidcRegisterResponse {
    client_id: String,
    client_secret: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OidcDeviceAuthRequest {
    client_id: String,
    client_secret: String,
    start_url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OidcDeviceAuthResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: Option<String>,
    interval: Option<u64>,
    expires_in: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OidcTokenRequest {
    client_id: String,
    client_secret: String,
    device_code: String,
    grant_type: &'static str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OidcTokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    error: Option<String>,
    error_description: Option<String>,
}

// ============ startUrl 映射 ============

const DEFAULT_START_URL: &str = "https://view.awsapps.com/start";
const DEFAULT_ENTERPRISE_URL: &str = "https://d-906600eb6f.awsapps.com/start";

fn resolve_start_url(login_type: &str, enterprise_url: Option<&str>) -> String {
    match login_type {
        "enterprise" => {
            let url = enterprise_url
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .unwrap_or(DEFAULT_ENTERPRISE_URL);
            if !url.starts_with("https://") {
                DEFAULT_ENTERPRISE_URL.to_string()
            } else {
                url.trim_end_matches('/').to_string()
            }
        }
        _ => DEFAULT_START_URL.to_string(),
    }
}

// ============ 公开方法 ============

impl super::service::AdminService {
    /// Device Flow 步骤 1：注册 OIDC 客户端
    pub async fn device_flow_register(
        &self,
        login_type: String,
        enterprise_start_url: Option<String>,
    ) -> Result<super::types::DeviceFlowRegisterResponse, AdminServiceError> {
        let config = self.token_manager.config();
        let proxy = self.build_proxy();
        let client = build_oidc_client(&proxy, config.tls_backend)
            .map_err(|e| AdminServiceError::InternalError(e.to_string()))?;

        let region = config.effective_auth_region();
        let url = format!("https://oidc.{}.amazonaws.com/client/register", region);
        let start_url = resolve_start_url(&login_type, enterprise_start_url.as_deref());

        let body = OidcRegisterRequest {
            client_name: "kiro-manual-auth",
            client_type: "public",
            issuer_url: start_url,
            scopes: &[
                "codewhisperer:completions",
                "codewhisperer:analysis",
                "codewhisperer:conversations",
            ],
        };

        let resp = send_oidc_request(&client, &url, &body, region, &config).await
            .map_err(|e| AdminServiceError::UpstreamError(e.to_string()))?;

        let data: OidcRegisterResponse = resp.json().await
            .map_err(|e| AdminServiceError::UpstreamError(format!("解析注册响应失败: {}", e)))?;

        Ok(super::types::DeviceFlowRegisterResponse {
            client_id: data.client_id,
            client_secret: data.client_secret,
        })
    }

    /// Device Flow 步骤 2：获取设备授权码
    pub async fn device_flow_authorize(
        &self,
        client_id: String,
        client_secret: String,
        login_type: String,
        enterprise_start_url: Option<String>,
    ) -> Result<super::types::DeviceFlowAuthorizeResponse, AdminServiceError> {
        let config = self.token_manager.config();
        let proxy = self.build_proxy();
        let http_client = build_oidc_client(&proxy, config.tls_backend)
            .map_err(|e| AdminServiceError::InternalError(e.to_string()))?;

        let region = config.effective_auth_region();
        let url = format!("https://oidc.{}.amazonaws.com/device_authorization", region);
        let start_url = resolve_start_url(&login_type, enterprise_start_url.as_deref());

        let body = OidcDeviceAuthRequest {
            client_id,
            client_secret,
            start_url,
        };

        let resp = send_oidc_request(&http_client, &url, &body, region, &config).await
            .map_err(|e| AdminServiceError::UpstreamError(e.to_string()))?;

        let data: OidcDeviceAuthResponse = resp.json().await
            .map_err(|e| AdminServiceError::UpstreamError(format!("解析授权响应失败: {}", e)))?;

        Ok(super::types::DeviceFlowAuthorizeResponse {
            device_code: data.device_code,
            user_code: data.user_code,
            verification_uri: data.verification_uri,
            verification_uri_complete: data.verification_uri_complete.unwrap_or_default(),
            interval: data.interval.unwrap_or(2),
            expires_in: data.expires_in.unwrap_or(600),
        })
    }

    /// Device Flow 步骤 3：轮询 Token
    ///
    /// 透传 AWS OIDC 响应，让前端根据 error 字段判断状态：
    /// - `authorization_pending` → 等待用户授权
    /// - `slow_down` → 增大轮询间隔
    /// - `expired_token` → 设备码过期
    /// - 无 error 且有 accessToken → 授权成功
    pub async fn device_flow_poll(
        &self,
        client_id: String,
        client_secret: String,
        device_code: String,
    ) -> Result<super::types::DeviceFlowPollResponse, AdminServiceError> {
        let config = self.token_manager.config();
        let proxy = self.build_proxy();
        let http_client = build_oidc_client(&proxy, config.tls_backend)
            .map_err(|e| AdminServiceError::InternalError(e.to_string()))?;

        let region = config.effective_auth_region();
        let url = format!("https://oidc.{}.amazonaws.com/token", region);

        let body = OidcTokenRequest {
            client_id,
            client_secret,
            device_code,
            grant_type: "urn:ietf:params:oauth:grant-type:device_code",
        };

        // poll 允许非 200 响应（如 400 authorization_pending）
        let resp = send_oidc_request_raw(&http_client, &url, &body, region, &config).await
            .map_err(|e| AdminServiceError::UpstreamError(e.to_string()))?;

        let status = resp.status();
        let text = resp.text().await
            .map_err(|e| AdminServiceError::UpstreamError(format!("读取响应失败: {}", e)))?;

        let data: OidcTokenResponse = serde_json::from_str(&text)
            .map_err(|e| AdminServiceError::UpstreamError(format!("解析响应失败: {} (body: {})", e, &text[..text.len().min(200)])))?;

        // 如果有 error 字段，直接透传（即使 HTTP 200）
        if data.error.is_some() {
            return Ok(super::types::DeviceFlowPollResponse {
                access_token: None,
                refresh_token: None,
                expires_in: None,
                error: data.error,
                error_description: data.error_description,
            });
        }

        // HTTP 非 200 但没有 error 字段 → 视为上游错误
        if !status.is_success() {
            return Err(AdminServiceError::UpstreamError(format!(
                "OIDC 轮询失败: {} {}", status, &text[..text.len().min(200)]
            )));
        }

        Ok(super::types::DeviceFlowPollResponse {
            access_token: data.access_token,
            refresh_token: data.refresh_token,
            expires_in: data.expires_in,
            error: None,
            error_description: None,
        })
    }

    /// 构建代理配置（从全局 config 获取）
    fn build_proxy(&self) -> Option<ProxyConfig> {
        let config = self.token_manager.config();
        config.proxy_url.as_ref().map(|url| {
            let mut proxy = ProxyConfig::new(url);
            if let (Some(username), Some(password)) = (&config.proxy_username, &config.proxy_password) {
                proxy = proxy.with_auth(username, password);
            }
            proxy
        })
    }
}

// ============ 辅助函数 ============

fn build_oidc_client(proxy: &Option<ProxyConfig>, tls_backend: TlsBackend) -> anyhow::Result<Client> {
    build_client(proxy.as_ref(), 30, tls_backend)
}

/// 发送 OIDC 请求（要求成功状态码）
async fn send_oidc_request<T: Serialize>(
    client: &Client,
    url: &str,
    body: &T,
    region: &str,
    config: &Config,
) -> anyhow::Result<reqwest::Response> {
    let resp = send_oidc_request_raw(client, url, body, region, config).await?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        bail!("OIDC 请求失败: {} {} (url: {})", status, &text[..text.len().min(300)], url);
    }
    Ok(resp)
}

/// 发送 OIDC 请求（不检查状态码，由调用方处理）
async fn send_oidc_request_raw<T: Serialize>(
    client: &Client,
    url: &str,
    body: &T,
    region: &str,
    config: &Config,
) -> anyhow::Result<reqwest::Response> {
    let x_amz_user_agent = "aws-sdk-js/3.980.0 KiroIDE";
    let user_agent = format!(
        "aws-sdk-js/3.980.0 ua/2.1 os/{} lang/js md/nodejs#{} api/sso-oidc#3.980.0 m/E KiroIDE",
        config.system_version, config.node_version
    );

    let host = format!("oidc.{}.amazonaws.com", region);

    let resp = client
        .post(url)
        .header("content-type", "application/json")
        .header("x-amz-user-agent", x_amz_user_agent)
        .header("user-agent", &user_agent)
        .header("host", &host)
        .header("amz-sdk-invocation-id", uuid::Uuid::new_v4().to_string())
        .header("amz-sdk-request", "attempt=1; max=4")
        .header("Connection", "close")
        .json(body)
        .send()
        .await
        .with_context(|| format!("OIDC 请求发送失败: {}", url))?;

    Ok(resp)
}
