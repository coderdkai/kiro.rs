#!/usr/bin/env bash
# 从 kiro-rs Admin API 获取用户 web session 并生成 agent-browser state 文件
# 用法:
#   ./kiro_export_web_state.sh                                     # 通过 SSH 获取 API 地址和 key
#   ./kiro_export_web_state.sh --api http://host:port --key <key>  # 直接指定 API
#   ./kiro_export_web_state.sh --ssh do                            # 通过 SSH 转发
# 示例:
#   ./kiro_export_web_state.sh --ssh do
#   ./kiro_export_web_state.sh --api http://localhost:19827 --key admin-geniusk0218

set -euo pipefail

OUTPUT_DIR="${KIRO_STATE_DIR:-/tmp/kiro-web-states}"
API_URL=""
ADMIN_KEY=""
SSH_ALIAS=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --api)     API_URL="$2";    shift 2 ;;
        --key)     ADMIN_KEY="$2";  shift 2 ;;
        --ssh)     SSH_ALIAS="$2";  shift 2 ;;
        --output)  OUTPUT_DIR="$2"; shift 2 ;;
        *)         SSH_ALIAS="$1";  shift   ;;
    esac
done

mkdir -p "$OUTPUT_DIR"

# 通过 SSH 自动发现 API 地址和 key
if [ -n "$SSH_ALIAS" ] && { [ -z "$API_URL" ] || [ -z "$ADMIN_KEY" ]; }; then
    if [ -z "$ADMIN_KEY" ]; then
        ADMIN_KEY=$(ssh "$SSH_ALIAS" "docker exec kiro-rs cat config/config.json 2>/dev/null" \
            | python3 -c "import sys,json; print(json.load(sys.stdin).get('adminApiKey',''))" 2>/dev/null || true)
    fi
    if [ -z "$API_URL" ]; then
        PORT=$(ssh "$SSH_ALIAS" "docker port kiro-rs 8990/tcp 2>/dev/null" | head -1 | awk -F: '{print $NF}')
        if [ -n "$PORT" ]; then
            API_URL="http://localhost:${PORT}"
        fi
    fi
fi

if [ -z "$API_URL" ] || [ -z "$ADMIN_KEY" ]; then
    echo "错误: 缺少 API_URL 或 ADMIN_KEY"
    echo "用法: $0 --api <url> --key <admin_key>"
    echo "  或: $0 --ssh <ssh_alias>"
    exit 1
fi

echo "=== Kiro Web State 导出工具 ==="
echo "API: $API_URL"
echo ""

# 调用 web-sessions API
FETCH_CMD="curl -s ${API_URL}/api/admin/web-sessions -H 'Authorization: Bearer ${ADMIN_KEY}'"
if [ -n "$SSH_ALIAS" ]; then
    RESPONSE=$(ssh "$SSH_ALIAS" "$FETCH_CMD" 2>/dev/null)
else
    RESPONSE=$(eval "$FETCH_CMD" 2>/dev/null)
fi

if [ -z "$RESPONSE" ]; then
    echo "错误: API 返回空响应"
    exit 1
fi

# 解析并保存 state 文件
echo "$RESPONSE" | python3 -c "
import sys, json, os

output_dir = '$OUTPUT_DIR'
data = json.load(sys.stdin)

if 'error' in data:
    print(f'API 错误: {data[\"error\"]}')
    sys.exit(1)

total = data.get('total', 0)
sessions = data.get('sessions', [])

if not sessions:
    print('没有找到有 web session 的用户')
    sys.exit(0)

print(f'找到 {total} 个有 web session 的用户:')
print('')

for s in sessions:
    uid = s['id']
    email = s.get('email', 'unknown')
    user_id = s.get('userId', '')

    state = s['state']
    filename = f'kiro_user_{uid}.json'
    filepath = os.path.join(output_dir, filename)
    with open(filepath, 'w') as f:
        json.dump(state, f, indent=2)
    print(f'  [{uid}] {email} (UserId: {user_id[:30]}...) → {filepath}')

print('')
print(f'State 文件已导出到: {output_dir}')
"
