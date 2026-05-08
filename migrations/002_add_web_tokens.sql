-- Kiro Web 会话 token 字段 (用于浏览器级用户切换)
-- 这些字段来自自动注册时 Python 脚本的输出
ALTER TABLE credentials ADD COLUMN password TEXT;
ALTER TABLE credentials ADD COLUMN web_access_token TEXT;
ALTER TABLE credentials ADD COLUMN web_session_token TEXT;
ALTER TABLE credentials ADD COLUMN web_user_id TEXT;
