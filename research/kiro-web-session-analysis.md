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

| 脚本输出字段 | → | Web Cookie | 说明 |
|-------------|---|------------|------|
| `accessToken` | → | `AccessToken` | Kiro Web 访问令牌 |
| `sessionToken` | → | `SessionToken` | SSO Bearer Token |
| N/A | → | `Idp` | 固定值 "BuilderId" |
| N/A | → | `UserId` | 需从 whoAmI 或 token 解码获取 |
| `password` | → | N/A | 明文密码 (用于浏览器重新登录) |
| `clientId` | → | N/A | OIDC Client ID (API 网关用) |
| `clientSecret` | → | N/A | OIDC Client Secret (API 网关用) |
| `refreshToken` | → | N/A | OIDC Refresh Token (API 网关用) |

## 5. 实施建议

### 数据库新增字段

在 `credentials` 表中添加:
- `password TEXT` - 明文密码 (用于浏览器登录)
- `web_access_token TEXT` - Kiro Web AccessToken cookie 值
- `web_session_token TEXT` - Kiro Web SessionToken cookie 值
- `web_user_id TEXT` - Kiro Web UserId cookie 值

### 注册时保存

在 `auto_register.rs` 中，从 Python 脚本的 `###RESULT###` JSON 提取并保存:
- `password` → `credentials.password`
- `accessToken` → `credentials.web_access_token`
- `sessionToken` → `credentials.web_session_token`

### 用户切换 API (未来)

可提供 Admin API 端点:
```
GET /api/admin/credentials/:id/web-session
→ 返回 agent-browser 兼容的 state JSON
```

## 6. 安全注意事项

- 所有 web token 均有时效性 (~7天)
- 明文密码存储需评估安全风险
- 建议加密存储敏感字段
- signin.aws 的 `aws-usi-authn` cookie 是 AWS 端的 session，跨用户复用会导致问题
