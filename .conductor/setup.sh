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

# Symlink credentials files from root (shared across worktrees)
if [ -n "$CONDUCTOR_ROOT_PATH" ]; then
  for f in credentials.json; do
    if [ -f "$CONDUCTOR_ROOT_PATH/$f" ]; then
      ln -sf "$CONDUCTOR_ROOT_PATH/$f" "$f"
      echo "  Symlinked $f"
    fi
  done
  # Also symlink any credentials.*.json files
  for f in "$CONDUCTOR_ROOT_PATH"/credentials.*.json; do
    if [ -f "$f" ]; then
      fname=$(basename "$f")
      ln -sf "$f" "$fname"
      echo "  Symlinked $fname"
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
