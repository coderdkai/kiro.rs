//! 自动注册功能：调用 Python 注册脚本，通过 SSE 流式输出进度

use std::convert::Infallible;
use std::process::Stdio;
use std::sync::Arc;

use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::Stream;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_stream::wrappers::LinesStream;
use tokio_stream::StreamExt;

use crate::model::config::RegisterConfig;

use super::service::AdminService;
use super::types::AddCredentialRequest;

const RESULT_PREFIX: &str = "###RESULT###";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterResultEvent {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn create_register_stream(
    service: Arc<AdminService>,
    reg_config: RegisterConfig,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        yield Ok(Event::default().event("log").data("🚀 开始自动注册..."));

        let script_path = reg_config
            .script_path
            .clone()
            .unwrap_or_else(|| "/app/scripts/kiro_register.py".into());

        if !tokio::fs::try_exists(&script_path).await.unwrap_or(false) {
            yield Ok(emit_error(format!("注册脚本不存在: {}", script_path)));
            yield Ok(emit_result(RegisterResultEvent {
                success: false, email: None, credential_id: None,
                error: Some(format!("注册脚本不存在: {}", script_path)),
            }));
            return;
        }

        yield Ok(Event::default().event("log").data(
            format!("📜 脚本路径: {}", script_path)
        ));

        let env_vars = build_env_vars(&reg_config);

        let child_result = Command::new("python3")
            .arg(&script_path)
            .envs(env_vars.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .env("PYTHONUNBUFFERED", "1")
            .env("KIRO_JSON_OUTPUT", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let mut child = match child_result {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("启动 Python 进程失败: {}", e);
                yield Ok(emit_error(&msg));
                yield Ok(emit_result(RegisterResultEvent {
                    success: false, email: None, credential_id: None,
                    error: Some(msg),
                }));
                return;
            }
        };

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let mut stdout_lines = LinesStream::new(BufReader::new(stdout).lines());
        let mut stderr_lines = LinesStream::new(BufReader::new(stderr).lines());

        let mut result_json: Option<String> = None;

        loop {
            tokio::select! {
                line = stdout_lines.next() => {
                    match line {
                        Some(Ok(ref text)) => {
                            if let Some(json_str) = text.strip_prefix(RESULT_PREFIX) {
                                result_json = Some(json_str.to_string());
                            } else {
                                yield Ok(Event::default().event("log").data(text.as_str()));
                            }
                        }
                        Some(Err(e)) => {
                            yield Ok(Event::default().event("log").data(
                                format!("⚠️ stdout 读取错误: {}", e)
                            ));
                        }
                        None => {
                            while let Some(Ok(ref text)) = stderr_lines.next().await {
                                yield Ok(Event::default().event("log").data(
                                    format!("[stderr] {}", text)
                                ));
                            }
                            break;
                        }
                    }
                }
                line = stderr_lines.next() => {
                    if let Some(Ok(line)) = line {
                        yield Ok(Event::default().event("log").data(
                            format!("[stderr] {}", line)
                        ));
                    }
                }
            }
        }

        let exit_status = match child.wait().await {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("等待进程退出失败: {}", e);
                yield Ok(emit_error(&msg));
                yield Ok(emit_result(RegisterResultEvent {
                    success: false, email: None, credential_id: None,
                    error: Some(msg),
                }));
                return;
            }
        };

        if !exit_status.success() {
            let msg = format!("注册脚本退出码: {}", exit_status);
            yield Ok(emit_error(&msg));
            yield Ok(emit_result(RegisterResultEvent {
                success: false, email: None, credential_id: None,
                error: Some(msg),
            }));
            return;
        }

        let result_json = match result_json {
            Some(j) => j,
            None => {
                let msg = "注册脚本未输出结果 (缺少 ###RESULT### 行)".to_string();
                yield Ok(emit_error(&msg));
                yield Ok(emit_result(RegisterResultEvent {
                    success: false, email: None, credential_id: None,
                    error: Some(msg),
                }));
                return;
            }
        };

        yield Ok(Event::default().event("log").data("📋 解析注册结果..."));

        let parsed: serde_json::Value = match serde_json::from_str(&result_json) {
            Ok(v) => v,
            Err(e) => {
                let msg = format!("解析结果 JSON 失败: {}", e);
                yield Ok(emit_error(&msg));
                yield Ok(emit_result(RegisterResultEvent {
                    success: false, email: None, credential_id: None,
                    error: Some(msg),
                }));
                return;
            }
        };

        let email = parsed.get("email").and_then(|v| v.as_str()).map(String::from);
        let refresh_token = parsed.get("refreshToken").and_then(|v| v.as_str()).map(String::from);
        let client_id = parsed.get("clientId").and_then(|v| v.as_str()).map(String::from);
        let client_secret = parsed.get("clientSecret").and_then(|v| v.as_str()).map(String::from);
        let password = parsed.get("password").and_then(|v| v.as_str()).map(String::from);
        let web_access_token = parsed.get("accessToken").and_then(|v| v.as_str()).map(String::from);
        let web_session_token = parsed.get("sessionToken").and_then(|v| v.as_str()).map(String::from);
        let web_user_id = parsed.get("userId").and_then(|v| v.as_str())
            .filter(|s| !s.is_empty()).map(String::from);

        if refresh_token.is_none() {
            let msg = "注册结果缺少 refreshToken，无法添加到凭据池";
            yield Ok(Event::default().event("log").data(format!("⚠️ {}", msg)));
            yield Ok(emit_result(RegisterResultEvent {
                success: false, email: email.clone(), credential_id: None,
                error: Some(msg.into()),
            }));
            return;
        }

        yield Ok(Event::default().event("log").data(
            format!("✅ 注册成功: {}", email.as_deref().unwrap_or("unknown"))
        ));
        yield Ok(Event::default().event("log").data("📥 正在添加凭据到池子..."));

        let add_req = AddCredentialRequest {
            refresh_token,
            auth_method: "idc".into(),
            client_id,
            client_secret,
            priority: 0,
            region: None,
            auth_region: None,
            api_region: None,
            machine_id: None,
            email: email.clone(),
            proxy_url: None,
            proxy_username: None,
            proxy_password: None,
            kiro_api_key: None,
            endpoint: None,
            password,
            web_access_token,
            web_session_token,
            web_user_id,
        };

        match service.add_credential(add_req).await {
            Ok(resp) => {
                yield Ok(Event::default().event("log").data(
                    format!("🎉 凭据添加成功! ID: {}", resp.credential_id)
                ));
                yield Ok(emit_result(RegisterResultEvent {
                    success: true,
                    email,
                    credential_id: Some(resp.credential_id),
                    error: None,
                }));
            }
            Err(e) => {
                let msg = format!("添加凭据失败: {}", e);
                yield Ok(emit_error(&msg));
                yield Ok(emit_result(RegisterResultEvent {
                    success: false, email, credential_id: None,
                    error: Some(msg),
                }));
            }
        }
    };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("ping"),
    )
}

fn emit_error(msg: impl Into<String>) -> Event {
    Event::default().event("error").data(msg.into())
}

fn emit_result(result: RegisterResultEvent) -> Event {
    Event::default()
        .event("result")
        .data(serde_json::to_string(&result).unwrap_or_default())
}

fn build_env_vars(reg: &RegisterConfig) -> Vec<(String, String)> {
    let mut vars = vec![("EMAIL_PROVIDER".into(), "icloud".into())];

    macro_rules! push_if {
        ($field:expr, $key:expr) => {
            if let Some(v) = &$field {
                vars.push(($key.into(), v.clone()));
            }
        };
    }

    push_if!(reg.imap_host, "IMAP_HOST");
    if let Some(v) = reg.imap_port {
        vars.push(("IMAP_PORT".into(), v.to_string()));
    }
    push_if!(reg.imap_email, "IMAP_EMAIL");
    push_if!(reg.imap_password, "IMAP_PASSWORD");
    push_if!(reg.icloud_dsid, "ICLOUD_DSID");
    push_if!(reg.icloud_partition, "ICLOUD_PARTITION");
    push_if!(reg.icloud_cookies, "ICLOUD_COOKIES");
    push_if!(reg.hme_label, "HME_LABEL");
    push_if!(reg.proxy, "PROXY");
    push_if!(reg.password, "PASSWORD");

    vars
}
