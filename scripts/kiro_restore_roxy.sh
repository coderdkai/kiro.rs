#!/usr/bin/env bash
# 通过 Roxy Browser + agent-browser 恢复 Kiro 用户的浏览器状态
# 用法: ./kiro_restore_roxy.sh [state_file | user_id]
# 示例:
#   ./kiro_restore_roxy.sh /tmp/kiro-web-states/kiro_user_18.json
#   ./kiro_restore_roxy.sh 18           # 自动查找 /tmp/kiro-web-states/kiro_user_18.json
#   ./kiro_restore_roxy.sh              # 交互式选择用户

set -euo pipefail

STATE_DIR="${KIRO_STATE_DIR:-/tmp/kiro-web-states}"
ROXY_HOST="${ROXY_HOST:-http://127.0.0.1:50000}"
ROXY_API_KEY="${ROXY_API_KEY:?请设置 ROXY_API_KEY 环境变量}"

roxy_api() {
    local method="$1" endpoint="$2"
    shift 2
    curl -s -X "$method" "${ROXY_HOST}${endpoint}" \
        -H "Authorization: Bearer ${ROXY_API_KEY}" \
        -H "Content-Type: application/json" \
        "$@"
}

cleanup() {
    if [ -n "${ROXY_DIR_ID:-}" ] && [ -n "${ROXY_WORKSPACE_ID:-}" ]; then
        echo ""
        echo "清理 Roxy 浏览器..."
        agent-browser --cdp "$CDP_PORT" close 2>/dev/null || true
        roxy_api POST /browser/close -d "{\"workspaceId\":$ROXY_WORKSPACE_ID,\"dirId\":\"$ROXY_DIR_ID\"}" >/dev/null 2>&1 || true
        roxy_api POST /browser/delete -d "{\"workspaceId\":$ROXY_WORKSPACE_ID,\"dirIds\":[\"$ROXY_DIR_ID\"]}" >/dev/null 2>&1 || true
        echo "已清理"
    fi
}

# 确定 state 文件路径
STATE_FILE=""
if [ $# -ge 1 ]; then
    arg="$1"
    if [ -f "$arg" ]; then
        STATE_FILE="$arg"
    elif [[ "$arg" =~ ^[0-9]+$ ]]; then
        candidate="$STATE_DIR/kiro_user_${arg}.json"
        if [ -f "$candidate" ]; then
            STATE_FILE="$candidate"
        else
            echo "错误: State 文件不存在: $candidate"
            echo "请先运行 kiro_export_web_state.sh 导出"
            exit 1
        fi
    else
        echo "错误: 无效参数: $arg"
        echo "用法: $0 [state_file | user_id]"
        exit 1
    fi
fi

# 交互式选择
if [ -z "$STATE_FILE" ]; then
    if [ ! -d "$STATE_DIR" ]; then
        echo "错误: State 目录不存在: $STATE_DIR"
        echo "请先运行 kiro_export_web_state.sh 导出"
        exit 1
    fi

    files=($(ls "$STATE_DIR"/kiro_user_*.json 2>/dev/null || true))
    if [ ${#files[@]} -eq 0 ]; then
        echo "错误: 没有找到 state 文件"
        echo "请先运行 kiro_export_web_state.sh 导出"
        exit 1
    fi

    echo "=== 可用的 Kiro 用户 ==="
    for i in "${!files[@]}"; do
        f="${files[$i]}"
        uid=$(echo "$f" | sed -n 's/.*kiro_user_\([0-9]*\)\.json/\1/p')
        user_id=$(python3 -c "
import json
with open('$f') as fh:
    data = json.load(fh)
for c in data.get('cookies', []):
    if c['name'] == 'UserId':
        print(c['value'])
        break
" 2>/dev/null || echo "unknown")
        echo "  [$((i+1))] User $uid (UserId: $user_id)"
    done

    echo ""
    read -p "选择用户编号 [1-${#files[@]}]: " choice
    idx=$((choice - 1))
    if [ $idx -lt 0 ] || [ $idx -ge ${#files[@]} ]; then
        echo "无效选择"
        exit 1
    fi
    STATE_FILE="${files[$idx]}"
fi

echo ""
echo "=== Kiro 用户状态恢复 ==="
echo "State 文件: $STATE_FILE"

# 检查 Roxy Browser API
health=$(roxy_api GET /health 2>/dev/null | jq -r '.data // empty' 2>/dev/null || echo "")
if [ "$health" != "ok" ]; then
    echo "错误: Roxy Browser API 不可用 ($ROXY_HOST)"
    exit 1
fi

# 获取 Workspace ID
ROXY_WORKSPACE_ID=$(roxy_api GET /browser/workspace 2>/dev/null | jq -r '.data.rows[0].id')
echo "Workspace: $ROXY_WORKSPACE_ID"

# 创建浏览器 Profile
user_label=$(basename "$STATE_FILE" .json | sed 's/kiro_user_/kiro-/')
ROXY_DIR_ID=$(roxy_api POST /browser/create \
    -d "{\"workspaceId\":$ROXY_WORKSPACE_ID,\"windowName\":\"$user_label\"}" \
    | jq -r '.data.dirId')
echo "Browser Profile: $ROXY_DIR_ID"

trap cleanup EXIT

# 打开浏览器获取 CDP 端口
open_resp=$(roxy_api POST /browser/open \
    -d "{\"workspaceId\":$ROXY_WORKSPACE_ID,\"dirId\":\"$ROXY_DIR_ID\"}")
cdp_url=$(echo "$open_resp" | jq -r '.data.ws')
CDP_PORT=$(echo "$cdp_url" | sed -n 's/.*127\.0\.0\.1:\([0-9]*\).*/\1/p')
echo "CDP 端口: $CDP_PORT"

# 加载 state 并打开 Kiro
echo ""
echo "正在注入 cookie 并打开 Kiro..."
agent-browser --cdp "$CDP_PORT" batch \
    "state load $STATE_FILE" \
    "open https://app.kiro.dev" \
    "wait 5000"

# 检查结果
url=$(agent-browser --cdp "$CDP_PORT" get url 2>/dev/null)
if echo "$url" | grep -q "signin"; then
    echo ""
    echo "⚠️  登录失败: 页面跳转到 signin"
    echo "可能原因: token 已过期或无效"
    echo "建议: 重新注册用户获取新的 token"
else
    echo ""
    echo "✅ 用户状态恢复成功!"
    echo "当前 URL: $url"
    agent-browser --cdp "$CDP_PORT" screenshot
fi

echo ""
echo "浏览器保持打开状态。按 Enter 关闭浏览器..."
read -r
