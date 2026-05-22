#!/usr/bin/env bash
set -euo pipefail

REPO="${RUNTRAIL_INSTALL_REPO:-forjd-hermes-bot/runtrail}"
TAG="${RUNTRAIL_INSTALL_TAG:-latest}"
BIN="runtrail"
INSTALL_DIR="${RUNTRAIL_INSTALL_DIR:-${HOME}/.local/bin}"
TMP_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required command not found: $1" >&2
    exit 1
  fi
}

usage() {
  cat <<'USAGE'
Install runtrail.

Environment variables:
  RUNTRAIL_INSTALL_REPO  GitHub repo, default: forjd-hermes-bot/runtrail
  RUNTRAIL_INSTALL_TAG   Release tag, default: latest
  RUNTRAIL_INSTALL_DIR   Install directory, default: ~/.local/bin

Example:
  curl -fsSL https://raw.githubusercontent.com/forjd-hermes-bot/runtrail/main/install.sh | bash
USAGE
}

if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
  usage
  exit 0
fi

need uname
need tar
need mktemp
need chmod

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m | tr '[:upper:]' '[:lower:]')"
EXT="tar.gz"

case "$OS" in
  linux)
    os_part="unknown-linux-gnu"
    ;;
  darwin)
    os_part="apple-darwin"
    ;;
  mingw*|msys*|cygwin*)
    os_part="pc-windows-msvc"
    EXT="zip"
    BIN="runtrail.exe"
    ;;
  *)
    echo "error: unsupported OS: $OS" >&2
    exit 1
    ;;
esac

case "$ARCH" in
  x86_64|amd64)
    arch_part="x86_64"
    ;;
  arm64|aarch64)
    arch_part="aarch64"
    ;;
  *)
    echo "error: unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

TARGET="${arch_part}-${os_part}"
ASSET="runtrail-${TARGET}.${EXT}"
BASE_URL="https://github.com/${REPO}/releases/download/${TAG}"
URL="${BASE_URL}/${ASSET}"
CHECKSUM_URL="${BASE_URL}/SHA256SUMS"
ARCHIVE="${TMP_DIR}/${ASSET}"
CHECKSUMS="${TMP_DIR}/SHA256SUMS"

if [ "$EXT" = "zip" ]; then
  need unzip
fi

if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -qO "$2" "$1"; }
else
  echo "error: required command not found: curl or wget" >&2
  exit 1
fi

echo "Installing runtrail ${TAG} for ${TARGET}..."
fetch "$URL" "$ARCHIVE"

if fetch "$CHECKSUM_URL" "$CHECKSUMS"; then
  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$TMP_DIR" && grep " ${ASSET}$" SHA256SUMS | sha256sum -c -)
  elif command -v shasum >/dev/null 2>&1; then
    expected="$(grep " ${ASSET}$" "$CHECKSUMS" | awk '{print $1}')"
    actual="$(shasum -a 256 "$ARCHIVE" | awk '{print $1}')"
    if [ -z "$expected" ] || [ "$expected" != "$actual" ]; then
      echo "error: checksum mismatch for ${ASSET}" >&2
      exit 1
    fi
  else
    echo "warning: sha256sum/shasum unavailable; skipping checksum verification" >&2
  fi
else
  echo "warning: checksums unavailable; skipping checksum verification" >&2
fi

case "$EXT" in
  tar.gz)
    tar -xzf "$ARCHIVE" -C "$TMP_DIR"
    ;;
  zip)
    unzip -q "$ARCHIVE" -d "$TMP_DIR"
    ;;
esac

mkdir -p "$INSTALL_DIR"
install_path="${INSTALL_DIR}/${BIN}"
if command -v install >/dev/null 2>&1; then
  install -m 0755 "${TMP_DIR}/${BIN}" "$install_path"
else
  cp "${TMP_DIR}/${BIN}" "$install_path"
  chmod +x "$install_path"
fi

echo "Installed ${BIN} to ${install_path}"
if ! command -v "$BIN" >/dev/null 2>&1; then
  cat >&2 <<EOF
warning: ${INSTALL_DIR} is not on PATH.
Add this to your shell profile:
  export PATH="${INSTALL_DIR}:\$PATH"
EOF
fi

"$install_path" --version
