# Kiro Web 用户会话切换分析

> 日期: 2026-05-08
> 方法: agent-browser 有头模式实际登录 + cookie 注入验证

## 1. 认证流程

```
Kiro /signin
  → InitiateLogin (CBOR, 生成 PKCE code_challenge)
  → AWS Builder ID (signin.aws) 重定向链
  → 输入 Email → 输入 Password → 输入 OTP (邮箱验证码)
  → OAuth callback (code + state)
  → ExchangeToken (CBOR: code + codeVerifier + state)
  → 设置 cookies (AccessToken, SessionToken, Idp, UserId)
```

## 2. 关键 Cookie 分析

### app.kiro.dev 域 (认证核心)

| Cookie | 类型 | 属性 | 有效期 | 用途 |
|--------|------|------|--------|------|
| `AccessToken` | Kiro API 访问令牌 | httpOnly, Secure, SameSite=Lax | ~7天 | Kiro API 调用授权 |
| `SessionToken` | AWS SSO bearer token (JWE) | httpOnly, Secure, SameSite=Lax | ~7天 | SSO 会话标识 |
| `Idp` | 身份提供商 | httpOnly, Secure, SameSite=Lax | ~7天 | 固定值 "BuilderId" |
| `UserId` | 用户标识 | httpOnly, Secure, SameSite=Lax | ~7天 | 格式: `d-9067642ac7.<uuid>` |
| `kiro-visitor-id` | 访客追踪 | Secure (非 httpOnly) | 会话 | 前端生成 |
| `awsccc` | AWS cookie consent | Secure (非 httpOnly) | 会话 | 合规用途 |

### signin.aws 域 (登录流程)

| Cookie | 用途 | 切换用户时是否需要 |
|--------|------|-------------------|
| `directory-csrf-token` | 登录 CSRF 保护 | 否 |
| `aws-usi-authn` | AWS 认证 session | 否 (但会影响后续登录) |
| `workflow-csrf-token` | 登录 workflow CSRF | 否 |
| `platform-ubid` | 平台追踪 | 否 |
| `login-interview-token` | 登录面试 token | 否 |
| `workflow-step-id` | 登录步骤标识 | 否 |

## 3. 最小切换实验

### 实验 1: 完整 state 恢复 (17 cookies)
- **方法**: `agent-browser state save` → `state load`
- **结果**: ✅ 成功，直接以目标用户身份访问

### 实验 2: 最小 cookie 注入 (4 cookies)
- **方法**: 只注入 `AccessToken`, `SessionToken`, `Idp`, `UserId`
- **结果**: ✅ 成功，直接以目标用户身份访问
- **结论**: 这 4 个 cookie 是切换用户的充分条件

### 实验 3: JavaScript 注入
- **方法**: 通过 `document.cookie` 设置
- **结果**: ❌ 失败，因为 4 个核心 cookie 均为 httpOnly
- **结论**: 必须通过 CDP 或 state file 方式注入

### 实验 4: 跨用户 signin.aws session 复用
- **方法**: 用户 A 登录后，尝试用户 B 走 Builder ID 登录
- **结果**: ❌ PKCE 状态不匹配导致 OAuth 回调失败
- **结论**: 切换用户时需要清除 signin.aws 域的 cookies，或使用独立 session

## 4. Python 脚本输出的 Token 映射

`kiro_register.py` `###RESULT###` JSON 中的字段与 Kiro Web cookie 的对应关系:

| 脚本输出字段 | → | Web Cookie | 来源 | 说明 |
|-------------|---|------------|------|------|
| `accessToken` | → | `AccessToken` | **Set-Cookie 优先**, CBOR body 备选 | Kiro Web 访问令牌 |
| `sessionToken` | → | `SessionToken` | **Set-Cookie 优先**, bearer_token 备选 | SSO Bearer Token (JWE) |
| `userId` | → | `UserId` | **Set-Cookie** | 用户标识 `d-9067642ac7.<uuid>` |
| N/A | → | `Idp` | 固定值 | 固定值 "BuilderId" |
| `password` | → | N/A | 注册脚本 | 明文密码 (用于浏览器重新登录) |
| `clientId` | → | N/A | device auth 流程 | OIDC Client ID (反代用) |
| `clientSecret` | → | N/A | device auth 流程 | OIDC Client Secret (反代用) |
| `refreshToken` | → | N/A | device auth 流程 | OIDC Refresh Token (反代用) |

> **重要**: ExchangeToken 端点的 CBOR body 中的 `accessToken` 值可能与 Set-Cookie 头中的 `AccessToken` 不同。
> Python 脚本优先从 `r.cookies` (Set-Cookie) 提取，确保存储的是浏览器实际使用的 cookie 值。

## 5. 已实施的功能

### 数据库字段 (已完成)

`credentials` 表新增字段:
- `password TEXT` - 明文密码 (用于浏览器登录)
- `web_access_token TEXT` - Kiro Web AccessToken cookie 值
- `web_session_token TEXT` - Kiro Web SessionToken cookie 值
- `web_user_id TEXT` - Kiro Web UserId cookie 值

### 自动注册入库 (已完成)

`auto_register.rs` 从 Python 脚本的 `###RESULT###` JSON 提取并保存:
- `password` → `credentials.password`
- `accessToken` → `credentials.web_access_token` (Set-Cookie 优先)
- `sessionToken` → `credentials.web_session_token` (Set-Cookie 优先)
- `userId` → `credentials.web_user_id` (Set-Cookie)

## 6. Cookie 注入恢复用户状态

### State 文件格式

agent-browser 兼容的 state JSON，cookie 必须包含 `expires` 字段：

```json
{
  "cookies": [
    {"name": "AccessToken", "value": "...", "domain": ".app.kiro.dev", "path": "/", "httpOnly": true, "secure": true, "sameSite": "Lax", "expires": 1778838400},
    {"name": "SessionToken", "value": "...", "domain": ".app.kiro.dev", "path": "/", "httpOnly": true, "secure": true, "sameSite": "Lax", "expires": 1778838400},
    {"name": "Idp", "value": "BuilderId", "domain": ".app.kiro.dev", "path": "/", "httpOnly": true, "secure": true, "sameSite": "Lax", "expires": 1778838400},
    {"name": "UserId", "value": "d-9067642ac7.xxx", "domain": ".app.kiro.dev", "path": "/", "httpOnly": true, "secure": true, "sameSite": "Lax", "expires": 1778838400}
  ],
  "origins": []
}
```

> **关键**: `expires` 字段必须存在，否则 cookie 被视为 session cookie，导致注入无效。
> **关键**: `domain` 使用 `.app.kiro.dev` (带点前缀)。

### 使用 Roxy Browser 恢复 (推荐)

```bash
# 1. 导出 state 文件
./scripts/kiro_export_web_state.sh do

# 2. 恢复用户状态 (交互式选择)
./scripts/kiro_restore_roxy.sh

# 或指定用户 ID
./scripts/kiro_restore_roxy.sh 18

# 或指定 state 文件
./scripts/kiro_restore_roxy.sh /tmp/kiro-web-states/kiro_user_18.json
```

### 使用 agent-browser 直接恢复

```bash
agent-browser --headed batch \
  "state load /tmp/kiro-web-states/kiro_user_18.json" \
  "open https://app.kiro.dev" \
  "wait 5000"
```

## 7. 安全注意事项

- 所有 web token 均有时效性 (~7天)
- 明文密码存储需评估安全风险
- 建议加密存储敏感字段
- signin.aws 的 `aws-usi-authn` cookie 是 AWS 端的 session，跨用户复用会导致问题
- State 文件包含明文 token，应添加到 `.gitignore`
