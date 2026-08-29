#!/bin/bash
# Skill Garden CLI Installer
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CLI_NAME="skill-garden"

install_system() {
    ln -sf "$SCRIPT_DIR/${CLI_NAME}" "/usr/local/bin/${CLI_NAME}"
    chmod +x "$SCRIPT_DIR/${CLI_NAME}"
}

install_user_local() {
    mkdir -p "$HOME/.local/bin"
    cp "$SCRIPT_DIR/${CLI_NAME}" "$HOME/.local/bin/${CLI_NAME}"
    chmod +x "$HOME/.local/bin/${CLI_NAME}"

    if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.bashrc"
        echo "已添加 ~/.local/bin 到 PATH (~/.bashrc)"
    fi
}

if [ -w "/usr/local/bin" ]; then
    install_system
else
    install_user_local
fi
