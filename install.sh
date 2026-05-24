#!/usr/bin/env bash
set -euo pipefail

REPO="${RUNTRAIL_INSTALL_REPO:-forjd/runtrail}"
TAG="${RUNTRAIL_INSTALL_TAG:-}"
ALLOW_LATEST="${RUNTRAIL_INSTALL_ALLOW_LATEST:-0}"
SKIP_CHECKSUM="${RUNTRAIL_INSTALL_SKIP_CHECKSUM:-0}"
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
  RUNTRAIL_INSTALL_TAG            Required immutable release tag, e.g. runtrail-v0.3.0
  RUNTRAIL_INSTALL_REPO           GitHub repo, default: forjd/runtrail
  RUNTRAIL_INSTALL_DIR            Install directory, default: ~/.local/bin
  RUNTRAIL_INSTALL_ALLOW_LATEST=1 Allow the mutable latest release tag
  RUNTRAIL_INSTALL_SKIP_CHECKSUM=1 Skip checksum verification if unavailable

Example:
  curl -fsSL https://raw.githubusercontent.com/forjd/runtrail/main/install.sh \
    | RUNTRAIL_INSTALL_TAG=runtrail-v0.3.0 bash
USAGE
}

if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
  usage
  exit 0
fi

if [[ ! "$REPO" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]; then
  echo "error: invalid RUNTRAIL_INSTALL_REPO: $REPO" >&2
  exit 1
fi

if [ -z "$TAG" ]; then
  cat >&2 <<'EOF'
error: RUNTRAIL_INSTALL_TAG is required so installs are tied to an immutable release.
Set RUNTRAIL_INSTALL_TAG=runtrail-v0.3.0, or set RUNTRAIL_INSTALL_ALLOW_LATEST=1 and RUNTRAIL_INSTALL_TAG=latest to opt into the moving latest release.
EOF
  exit 1
fi

if [[ ! "$TAG" =~ ^[A-Za-z0-9._-]+$ ]]; then
  echo "error: invalid RUNTRAIL_INSTALL_TAG: $TAG" >&2
  exit 1
fi

if [ "$TAG" = "latest" ] && [ "$ALLOW_LATEST" != "1" ]; then
  echo "error: refusing mutable latest release without RUNTRAIL_INSTALL_ALLOW_LATEST=1" >&2
  exit 1
fi

need uname
need tar
need mktemp
need chmod
need grep

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

if [ "$os_part" = "pc-windows-msvc" ] && [ "$arch_part" = "aarch64" ]; then
  echo "error: Windows ARM64 release assets are not currently published" >&2
  exit 1
fi

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

verify_checksum() {
  if [ "$SKIP_CHECKSUM" = "1" ]; then
    echo "warning: RUNTRAIL_INSTALL_SKIP_CHECKSUM=1 set; skipping checksum verification" >&2
    return
  fi

  if ! fetch "$CHECKSUM_URL" "$CHECKSUMS"; then
    echo "error: checksums unavailable at ${CHECKSUM_URL}" >&2
    echo "Set RUNTRAIL_INSTALL_SKIP_CHECKSUM=1 only if you accept this risk." >&2
    exit 1
  fi

  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$TMP_DIR" && grep -F " ${ASSET}" SHA256SUMS | sha256sum -c -)
  elif command -v shasum >/dev/null 2>&1; then
    local line expected actual
    line="$(grep -F " ${ASSET}" "$CHECKSUMS" || true)"
    expected="${line%% *}"
    actual="$(shasum -a 256 "$ARCHIVE")"
    actual="${actual%% *}"
    if [ -z "$line" ] || [ -z "$expected" ] || [ "$expected" != "$actual" ]; then
      echo "error: checksum mismatch for ${ASSET}" >&2
      exit 1
    fi
  else
    echo "error: sha256sum/shasum unavailable; cannot verify ${ASSET}" >&2
    echo "Set RUNTRAIL_INSTALL_SKIP_CHECKSUM=1 only if you accept this risk." >&2
    exit 1
  fi
}

echo "Installing runtrail ${TAG} for ${TARGET}..."
fetch "$URL" "$ARCHIVE"
verify_checksum

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
