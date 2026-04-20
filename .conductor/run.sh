#!/bin/zsh
set -eo pipefail

exec cargo run -- -c ./config.json ${CONDUCTOR_ROOT_PATH:+--credentials "$CONDUCTOR_ROOT_PATH/credentials.json"}
