#!/bin/zsh
set -eo pipefail

exec cargo run -- -c ./config.json
