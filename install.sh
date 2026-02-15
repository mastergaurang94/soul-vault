#!/usr/bin/env bash
# Soul Vault installer — downloads the latest release binary for your platform.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/user/soul-vault/main/install.sh | bash
#
# Options:
#   SOUL_VAULT_INSTALL_DIR  — override install directory (default: ~/.local/bin)
#   SOUL_VAULT_VERSION      — install a specific version (default: latest)

set -euo pipefail

REPO="mastergaurang94/soul-vault"
BINARY_NAME="soul"
DEFAULT_INSTALL_DIR="$HOME/.local/bin"
INSTALL_DIR="${SOUL_VAULT_INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"

# ─── Helpers ───────────────────────────────────────────────────────────────────

info()  { printf "\033[0;36m%s\033[0m\n" "$*"; }
ok()    { printf "\033[0;32m✓ %s\033[0m\n" "$*"; }
err()   { printf "\033[0;31m✗ %s\033[0m\n" "$*" >&2; }
die()   { err "$@"; exit 1; }

need() {
    command -v "$1" > /dev/null 2>&1 || die "Required tool '$1' not found. Please install it."
}

# ─── Detect platform ──────────────────────────────────────────────────────────

detect_platform() {
    local os arch

    case "$(uname -s)" in
        Darwin) os="macos"  ;;
        Linux)  os="linux"  ;;
        *)      die "Unsupported OS: $(uname -s). Soul Vault supports macOS and Linux." ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64)       arch="x86_64" ;;
        arm64|aarch64)      arch="arm64"  ;;
        *)                  die "Unsupported architecture: $(uname -m). Soul Vault supports x86_64 and arm64." ;;
    esac

    echo "${os}-${arch}"
}

# ─── Resolve version ──────────────────────────────────────────────────────────

resolve_version() {
    if [ -n "${SOUL_VAULT_VERSION:-}" ]; then
        echo "$SOUL_VAULT_VERSION"
        return
    fi

    need curl
    need grep

    local latest
    latest="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' \
        | head -1 \
        | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')" \
        || die "Failed to fetch latest version from GitHub."

    [ -n "$latest" ] || die "Could not determine latest version."
    echo "$latest"
}

# ─── Download & install ───────────────────────────────────────────────────────

main() {
    info "Soul Vault Installer"
    info "─────────────────────────────────"

    need curl

    local platform version artifact url

    platform="$(detect_platform)"
    info "Platform: ${platform}"

    version="$(resolve_version)"
    info "Version:  ${version}"

    artifact="${BINARY_NAME}-${platform}"
    url="https://github.com/${REPO}/releases/download/${version}/${artifact}"

    info "Downloading ${artifact}..."
    local tmpdir
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    curl -fSL --progress-bar "$url" -o "${tmpdir}/${BINARY_NAME}" \
        || die "Download failed. Check that version '${version}' exists and has a '${artifact}' binary."

    # Verify checksum if available
    local checksum_url="${url}.sha256"
    if curl -fsSL "$checksum_url" -o "${tmpdir}/checksum.sha256" 2>/dev/null; then
        info "Verifying checksum..."
        local expected actual
        expected="$(awk '{print $1}' "${tmpdir}/checksum.sha256")"
        if command -v sha256sum > /dev/null 2>&1; then
            actual="$(sha256sum "${tmpdir}/${BINARY_NAME}" | awk '{print $1}')"
        elif command -v shasum > /dev/null 2>&1; then
            actual="$(shasum -a 256 "${tmpdir}/${BINARY_NAME}" | awk '{print $1}')"
        else
            info "  (no sha256sum/shasum found, skipping verification)"
            actual="$expected"
        fi
        if [ "$expected" != "$actual" ]; then
            die "Checksum mismatch!\n  Expected: ${expected}\n  Got:      ${actual}"
        fi
        ok "Checksum verified"
    fi

    # Install
    chmod +x "${tmpdir}/${BINARY_NAME}"
    mkdir -p "$INSTALL_DIR"

    if [ -w "$INSTALL_DIR" ]; then
        mv "${tmpdir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    else
        info "Need sudo to install to ${INSTALL_DIR}"
        sudo mv "${tmpdir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    fi

    ok "Installed to ${INSTALL_DIR}/${BINARY_NAME}"

    # Check PATH
    if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
        echo ""
        info "⚠ ${INSTALL_DIR} is not in your PATH."
        info "  Add it to your shell config:"
        info "    export PATH=\"${INSTALL_DIR}:\$PATH\""
        echo ""
    fi

    echo ""
    ok "Soul Vault ${version} installed successfully!"
    echo ""
    info "Get started:"
    info "  soul          — launch interactive menu"
    info "  soul init     — set up your vault"
    info "  soul --help   — see all commands"
    echo ""
}

main "$@"
