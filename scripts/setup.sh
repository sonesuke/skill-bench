#!/bin/sh
set -eu

REPO="sonesuke/skill-bench"
GITHUB="https://github.com"
API="https://api.github.com"

main() {
  get_os
  get_arch
  check_deps
  download
  install
}

get_os() {
  os="$(uname -s)"
  case "$os" in
    Linux)  os="linux"  ;;
    Darwin) os="macos"  ;;
    *)      fail "Unsupported OS: $os" ;;
  esac
}

get_arch() {
  arch="$(uname -m)"
  case "$arch" in
    x86_64|amd64) arch="x86_64" ;;
    aarch64|arm64) arch="arm64" ;;
    *) fail "Unsupported architecture: $arch" ;;
  esac
}

check_deps() {
  need curl tar
}

need() {
  for cmd in "$@"; do
    command -v "$cmd" >/dev/null 2>&1 || fail "'$cmd' is required"
  done
}

download() {
  tag="$(get_latest_tag)"
  artifact="skill-bench-${os}-${arch}"
  url="${GITHUB}/${REPO}/releases/download/${tag}/${artifact}.tar.gz"

  printf "Downloading skill-bench %s (%s/%s) ...\n" "$tag" "$os" "$arch"

  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT

  curl -fsSL "$url" -o "${tmp}/${artifact}.tar.gz" || fail "Download failed"
  tar xzf "${tmp}/${artifact}.tar.gz" -C "$tmp" || fail "Extraction failed"
  bin="${tmp}/skill-bench"
  [ -f "$bin" ] || fail "Binary not found in archive"
  mv "$bin" "${tmp}/skill-bench-install"
  INSTALL_PATH="${tmp}/skill-bench-install"
}

install() {
  if [ -w "${HOME}/.local/bin" ] || mkdir -p "${HOME}/.local/bin" 2>/dev/null; then
    mv "$INSTALL_PATH" "${HOME}/.local/bin/skill-bench"
    chmod +x "${HOME}/.local/bin/skill-bench"
    printf "Installed to %s/.local/bin/skill-bench\n" "$HOME"
    case ":${PATH}:" in
      *":${HOME}/.local/bin:"*) ;;
      *) printf "Add %s/.local/bin to your PATH to use skill-bench\n" "$HOME" ;;
    esac
  else
    fail "Cannot write to ~/.local/bin"
  fi
}

get_latest_tag() {
  curl -fsSL "${API}/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' \
    | head -1 \
    | sed -E 's/.*"([^"]+)".*/\1/'
}

fail() {
  printf "Error: %s\n" "$1" >&2
  exit 1
}

main
