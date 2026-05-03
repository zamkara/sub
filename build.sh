#!/usr/bin/env bash
set -euo pipefail

# sub - Build & Deploy script
# Usage: ./build.sh [dev|release|install|clean]

BOLD=$'\033[1m'
GREEN=$'\033[0;32m'
CYAN=$'\033[0;36m'
YELLOW=$'\033[1;33m'
NC=$'\033[0m'

PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"
TARGET_DIR="$PROJECT_DIR/target"
BIN_NAME="sub"

build_dev() {
    echo -e "${CYAN}Building dev...${NC}"
    cd "$PROJECT_DIR"
    cargo build
    echo -e "${GREEN}Binary: ${TARGET_DIR}/debug/${BIN_NAME}${NC}"
}

build_release() {
    echo -e "${CYAN}Building release...${NC}"
    cd "$PROJECT_DIR"
    cargo build --release
    local size
    size=$(du -h "${TARGET_DIR}/release/${BIN_NAME}" | cut -f1)
    echo -e "${GREEN}Binary: ${TARGET_DIR}/release/${BIN_NAME} (${size})${NC}"
}

install() {
    echo -e "${CYAN}Installing to ~/.local/bin...${NC}"
    mkdir -p "$HOME/.local/bin"
    if [[ ! -f "${TARGET_DIR}/release/${BIN_NAME}" ]]; then
        build_release
    fi
    cp "${TARGET_DIR}/release/${BIN_NAME}" "$HOME/.local/bin/${BIN_NAME}"
    chmod +x "$HOME/.local/bin/${BIN_NAME}"
    echo -e "${GREEN}Installed: ~/.local/bin/${BIN_NAME}${NC}"

    if ! echo "$PATH" | grep -q "$HOME/.local/bin"; then
        echo -e "${YELLOW}Add to PATH: echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc${NC}"
    fi
}

clean() {
    echo -e "${CYAN}Cleaning...${NC}"
    cd "$PROJECT_DIR"
    cargo clean
    echo -e "${GREEN}Cleaned${NC}"
}

usage() {
    echo -e "${BOLD}Usage:${NC} ./build.sh [dev|release|install|clean]"
    echo
    echo -e "  ${BOLD}dev${NC}      Build debug binary"
    echo -e "  ${BOLD}release${NC}  Build release binary (optimized)"
    echo -e "  ${BOLD}install${NC}  Build & install to ~/.local/bin"
    echo -e "  ${BOLD}clean${NC}    Remove build artifacts"
}

case "${1:-release}" in
    dev)     build_dev ;;
    release) build_release ;;
    install) install ;;
    clean)   clean ;;
    *)       usage ;;
esac
