#!/bin/zsh
set -eo pipefail

echo "==> Setting up workspace: ${CONDUCTOR_WORKSPACE_NAME}"

# Copy config.json and replace port with CONDUCTOR_PORT
if [ -n "$CONDUCTOR_ROOT_PATH" ] && [ -f "$CONDUCTOR_ROOT_PATH/config.json" ]; then
  cp "$CONDUCTOR_ROOT_PATH/config.json" config.json
  if [ -n "$CONDUCTOR_PORT" ]; then
    sed -i '' "s/\"port\": [0-9]*/\"port\": $CONDUCTOR_PORT/" config.json
    echo "  Copied config.json with port=$CONDUCTOR_PORT"
  else
    echo "  Copied config.json (default port)"
  fi
fi

# Symlink database files from root (shared across worktrees)
if [ -n "$CONDUCTOR_ROOT_PATH" ] && [ -d "$CONDUCTOR_ROOT_PATH/config" ]; then
  mkdir -p config
  for f in "$CONDUCTOR_ROOT_PATH"/config/credentials.db*; do
    if [ -f "$f" ]; then
      fname=$(basename "$f")
      ln -sf "$f" "config/$fname"
      echo "  Symlinked config/$fname"
    fi
  done
fi

# Install frontend dependencies
if [ -d "admin-ui" ]; then
  echo "  Installing admin-ui dependencies..."
  cd admin-ui && pnpm install && cd ..
fi

# Build Rust backend
echo "  Building Rust backend..."
cargo build

echo "==> Setup complete"
