#!/usr/bin/env bash
set -euo pipefail

# sub - Deploy script (system-wide or user-level)
# Usage: ./deploy.sh [user|system|uninstall]

BOLD=$'\033[1m'
GREEN=$'\033[0;32m'
CYAN=$'\033[0;36m'
YELLOW=$'\033[1;33m'
NC=$'\033[0m'

PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"
BIN_NAME="sub"
VERSION="2.0.0"

deploy_user() {
    echo -e "${CYAN}Deploying to user-level (~/.local/bin)...${NC}"

    # Build if needed
    if [[ ! -f "${PROJECT_DIR}/target/release/${BIN_NAME}" ]]; then
        echo -e "${CYAN}Building release...${NC}"
        cargo build --release --manifest-path "${PROJECT_DIR}/Cargo.toml"
    fi

    mkdir -p "$HOME/.local/bin"
    cp "${PROJECT_DIR}/target/release/${BIN_NAME}" "$HOME/.local/bin/${BIN_NAME}"
    chmod +x "$HOME/.local/bin/${BIN_NAME}"

    echo -e "${GREEN}Installed: ~/.local/bin/${BIN_NAME}${NC}"

    # Add to PATH if needed
    if ! grep -q "\.local/bin" "$HOME/.bashrc" 2>/dev/null; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.bashrc"
        echo -e "${YELLOW}Added ~/.local/bin to ~/.bashrc${NC}"
    fi

    # Install shell completion
    local completion_path="$HOME/.local/share/bash-completion/completions"
    mkdir -p "$completion_path"
    # Generate completion if sub supports it
    if command -v "${PROJECT_DIR}/target/release/${BIN_NAME}" >/dev/null 2>&1; then
        "${PROJECT_DIR}/target/release/${BIN_NAME}" --generate-shell-completion bash > "$completion_path/${BIN_NAME}" 2>/dev/null || true
    fi

    echo -e "${GREEN}Version ${VERSION} deployed successfully${NC}"
    echo -e "Run ${CYAN}sub help${NC} to get started"
}

deploy_system() {
    echo -e "${CYAN}Deploying system-wide (/usr/local/bin)...${NC}"

    if [[ $EUID -ne 0 ]]; then
        echo -e "${YELLOW}Need sudo for system-wide install${NC}"
        exec sudo "$0" system
    fi

    # Build if needed
    if [[ ! -f "${PROJECT_DIR}/target/release/${BIN_NAME}" ]]; then
        echo -e "${CYAN}Building release...${NC}"
        cargo build --release --manifest-path "${PROJECT_DIR}/Cargo.toml"
    fi

    cp "${PROJECT_DIR}/target/release/${BIN_NAME}" "/usr/local/bin/${BIN_NAME}"
    chmod +x "/usr/local/bin/${BIN_NAME}"

    echo -e "${GREEN}Installed: /usr/local/bin/${BIN_NAME}${NC}"
    echo -e "${GREEN}Version ${VERSION} deployed system-wide${NC}"
}

uninstall() {
    echo -e "${CYAN}Uninstalling...${NC}"

    local removed=false

    if [[ -f "$HOME/.local/bin/${BIN_NAME}" ]]; then
        rm -f "$HOME/.local/bin/${BIN_NAME}"
        echo -e "${GREEN}Removed: ~/.local/bin/${BIN_NAME}${NC}"
        removed=true
    fi

    if [[ -f "/usr/local/bin/${BIN_NAME}" ]]; then
        if [[ $EUID -ne 0 ]]; then
            exec sudo "$0" uninstall
        fi
        rm -f "/usr/local/bin/${BIN_NAME}"
        echo -e "${GREEN}Removed: /usr/local/bin/${BIN_NAME}${NC}"
        removed=true
    fi

    if [[ "$removed" == "false" ]]; then
        echo -e "${YELLOW}${BIN_NAME} not found${NC}"
    else
        echo -e "${GREEN}Uninstalled${NC}"
    fi
}

usage() {
    echo -e "${BOLD}Usage:${NC} ./deploy.sh [user|system|uninstall]"
    echo
    echo -e "  ${BOLD}user${NC}       Install to ~/.local/bin (no sudo)"
    echo -e "  ${BOLD}system${NC}     Install to /usr/local/bin (requires sudo)"
    echo -e "  ${BOLD}uninstall${NC}  Remove installed binary"
}

case "${1:-user}" in
    user)       deploy_user ;;
    system)     deploy_system ;;
    uninstall)  uninstall ;;
    *)          usage ;;
esac
