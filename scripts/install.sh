#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"

BINARY_NAME="gdscript-formatter-mcp"
INSTALL_ROOT="${MCP_INSTALL_ROOT:-$HOME/.local/share/mcp/$BINARY_NAME}"
BIN_DIR="${MCP_BIN_DIR:-$HOME/.local/bin}"
PUBLIC_LINK="$BIN_DIR/$BINARY_NAME"
CURRENT_LINK="$INSTALL_ROOT/current"
PROTOCOL_VERSION="2024-11-05"

GITHUB_REPO="poyu0692/gdscript-formatter-mcp"
RELEASES_URL="https://github.com/${GITHUB_REPO}/releases"

usage() {
  cat <<USAGE
Usage: scripts/install.sh <command> [args]

Commands:
  install [options]   Install from prebuilt binary (default) or source.
    --from-source     Build from source instead of downloading prebuilt binary
    --version VERSION Install specific version (default: latest release)
  link [version]      Re-point links to installed version (default: current Cargo.toml version).
  uninstall [version] Uninstall one version (default: current Cargo.toml version).
  uninstall --all     Remove all installed versions and links.
  status              Show install state and active links.
  doctor              Verify executable exists and MCP handshake/tools list work.
USAGE
}

package_version() {
  awk -F '"' '/^version = "/ { print $2; exit }' "$REPO_ROOT/Cargo.toml"
}

detect_platform() {
  local os arch

  case "$(uname -s)" in
    Linux*)
      os="unknown-linux-gnu"
      ;;
    Darwin*)
      os="apple-darwin"
      ;;
    *)
      echo "Unsupported OS: $(uname -s)" >&2
      exit 1
      ;;
  esac

  case "$(uname -m)" in
    x86_64|amd64)
      arch="x86_64"
      ;;
    aarch64|arm64)
      arch="aarch64"
      ;;
    *)
      echo "Unsupported architecture: $(uname -m)" >&2
      exit 1
      ;;
  esac

  echo "${arch}-${os}"
}

get_latest_version() {
  local latest
  latest=$(curl -fsSL "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
  if [[ -z "$latest" ]]; then
    echo "Failed to fetch latest version" >&2
    exit 1
  fi
  echo "$latest"
}

download_and_install() {
  local version="$1"
  local target="$2"

  # Remove 'v' prefix if present
  local clean_version="${version#v}"
  local version_dir="$INSTALL_ROOT/$clean_version"
  local target_bin="$version_dir/$BINARY_NAME"

  if [[ -x "$target_bin" ]]; then
    echo "Version $clean_version is already installed at $target_bin"
    ln -sfn "$version_dir" "$CURRENT_LINK"
    ln -sfn "$CURRENT_LINK/$BINARY_NAME" "$PUBLIC_LINK"
    echo "Active command: $PUBLIC_LINK"
    return
  fi

  local download_url="${RELEASES_URL}/download/${version}/${BINARY_NAME}-${target}.tar.gz"
  local temp_dir
  temp_dir="$(mktemp -d)"
  trap "rm -rf '$temp_dir'" EXIT

  echo "Downloading from: $download_url"
  if ! curl -fL "$download_url" -o "$temp_dir/archive.tar.gz"; then
    echo "Failed to download binary for $target" >&2
    echo "URL: $download_url" >&2
    exit 1
  fi

  echo "Extracting archive..."
  tar xzf "$temp_dir/archive.tar.gz" -C "$temp_dir"

  mkdir -p "$version_dir" "$BIN_DIR"
  install -m 0755 "$temp_dir/$BINARY_NAME" "$target_bin"

  ln -sfn "$version_dir" "$CURRENT_LINK"
  ln -sfn "$CURRENT_LINK/$BINARY_NAME" "$PUBLIC_LINK"

  echo "Installed: $target_bin"
  echo "Active command: $PUBLIC_LINK"
}

install_from_source() {
  local version
  version="$(package_version)"
  local version_dir="$INSTALL_ROOT/$version"
  local source_bin="$REPO_ROOT/target/release/$BINARY_NAME"
  local target_bin="$version_dir/$BINARY_NAME"

  echo "Building release binary from source..."
  cargo build --release --manifest-path "$REPO_ROOT/Cargo.toml"

  mkdir -p "$version_dir" "$BIN_DIR"
  install -m 0755 "$source_bin" "$target_bin"

  ln -sfn "$version_dir" "$CURRENT_LINK"
  ln -sfn "$CURRENT_LINK/$BINARY_NAME" "$PUBLIC_LINK"

  echo "Installed: $target_bin"
  echo "Active command: $PUBLIC_LINK"
}

ensure_version_installed() {
  local version="$1"
  local target="$INSTALL_ROOT/$version/$BINARY_NAME"
  if [[ ! -x "$target" ]]; then
    echo "Not installed: $target" >&2
    exit 1
  fi
}

install_cmd() {
  local from_source=false
  local requested_version=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --from-source)
        from_source=true
        shift
        ;;
      --version)
        requested_version="$2"
        shift 2
        ;;
      *)
        echo "Unknown option: $1" >&2
        usage
        exit 1
        ;;
    esac
  done

  if [[ "$from_source" == true ]]; then
    install_from_source
    return
  fi

  local target
  target="$(detect_platform)"

  local version
  if [[ -n "$requested_version" ]]; then
    version="$requested_version"
  else
    echo "Fetching latest release version..."
    version="$(get_latest_version)"
  fi

  echo "Installing version: $version"
  echo "Platform: $target"

  download_and_install "$version" "$target"
}

link_cmd() {
  local version="${1:-$(package_version)}"
  ensure_version_installed "$version"

  mkdir -p "$BIN_DIR"
  ln -sfn "$INSTALL_ROOT/$version" "$CURRENT_LINK"
  ln -sfn "$CURRENT_LINK/$BINARY_NAME" "$PUBLIC_LINK"

  echo "Linked to version: $version"
  echo "Active command: $PUBLIC_LINK"
}

uninstall_cmd() {
  local arg="${1:-$(package_version)}"

  if [[ "$arg" == "--all" ]]; then
    rm -rf "$INSTALL_ROOT"
    rm -f "$PUBLIC_LINK"
    echo "Removed all installations under $INSTALL_ROOT"
    return
  fi

  local version="$arg"
  local target_dir="$INSTALL_ROOT/$version"

  if [[ -d "$target_dir" ]]; then
    rm -rf "$target_dir"
    echo "Removed: $target_dir"
  else
    echo "Version not found: $version"
  fi

  if [[ -L "$CURRENT_LINK" ]]; then
    local current_target
    current_target="$(readlink "$CURRENT_LINK")"
    if [[ "$current_target" == "$target_dir" ]]; then
      rm -f "$CURRENT_LINK"
    fi
  fi

  if [[ -L "$PUBLIC_LINK" && ! -e "$PUBLIC_LINK" ]]; then
    rm -f "$PUBLIC_LINK"
  fi
}

status_cmd() {
  echo "Install root: $INSTALL_ROOT"
  echo "Binary link:  $PUBLIC_LINK"

  if [[ -L "$PUBLIC_LINK" ]]; then
    echo "Link target: $(readlink "$PUBLIC_LINK")"
  elif [[ -x "$PUBLIC_LINK" ]]; then
    echo "Link target: (regular executable)"
  else
    echo "Link target: (missing)"
  fi

  if [[ -L "$CURRENT_LINK" ]]; then
    echo "Current dir: $(readlink "$CURRENT_LINK")"
  else
    echo "Current dir: (missing)"
  fi

  echo "Installed versions:"
  if [[ -d "$INSTALL_ROOT" ]]; then
    find "$INSTALL_ROOT" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | grep -v '^current$' | sort -V || true
  fi
}

make_mcp_message() {
  local json="$1"
  printf 'Content-Length: %d\r\n\r\n%s' "${#json}" "$json"
}

doctor_cmd() {
  local exe="$PUBLIC_LINK"
  if [[ ! -x "$exe" ]]; then
    echo "Executable not found: $exe" >&2
    exit 1
  fi

  local init_msg
  local list_msg
  init_msg='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"'"$PROTOCOL_VERSION"'","capabilities":{},"clientInfo":{"name":"doctor","version":"0.0.0"}}}'
  list_msg='{"jsonrpc":"2.0","id":2,"method":"tools/list"}'

  local output
  output="$({ make_mcp_message "$init_msg"; make_mcp_message "$list_msg"; } | "$exe")"

  if ! grep -q '"gdscript_format"' <<<"$output"; then
    echo "Doctor failed: gdscript_format tool not found in tools/list output." >&2
    exit 1
  fi
  if ! grep -q '"gdscript_lint"' <<<"$output"; then
    echo "Doctor failed: gdscript_lint tool not found in tools/list output." >&2
    exit 1
  fi

  echo "Doctor OK"
}

main() {
  local cmd="${1:-}"
  case "$cmd" in
    install)
      shift
      install_cmd "$@"
      ;;
    link)
      shift
      link_cmd "$@"
      ;;
    uninstall)
      shift
      uninstall_cmd "$@"
      ;;
    status)
      shift
      status_cmd "$@"
      ;;
    doctor)
      shift
      doctor_cmd "$@"
      ;;
    -h|--help|help|"")
      usage
      ;;
    *)
      echo "Unknown command: $cmd" >&2
      usage
      exit 1
      ;;
  esac
}

main "$@"
