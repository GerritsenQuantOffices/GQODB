#!/bin/sh
# Build, test and install the two open GQODB command-line tools from source.
# Prerequisite installation requires explicit consent. The engine is not included.
set -eu

repo=${GQODB_REPOSITORY:-https://github.com/GerritsenQuantOffices/GQODB.git}
ref=${GQODB_REF:-main}
prefix=${GQODB_PREFIX:-"$HOME/.local"}
toolchain=${GQODB_RUST_TOOLCHAIN:-1.96.0}
force=0

usage() {
    cat <<'EOF'
Usage: ./install.sh [--prefix DIR] [--ref GIT_REF] [--force]

Downloads a clean source checkout, runs the locked workspace tests, builds the
release binaries and installs gqodb-codec and ob_store under DIR/bin.

Defaults:
  --prefix "$HOME/.local"
  --ref    main

Environment overrides:
  GQODB_REPOSITORY       Git repository URL
  GQODB_RUST_TOOLCHAIN   rustup toolchain (default: 1.96.0)

Missing Linux build dependencies and Rust are offered with separate [y/N]
prompts. Only confirmed system-package installation uses sudo. Declining
or reaching end-of-input stops without installing that prerequisite.
Automatic system-package setup supports Debian/Ubuntu with apt-get.
Existing binaries are not replaced unless --force is given.
EOF
}

confirm() {
    printf '%s [y/N] ' "$1" >&2
    answer=n
    IFS= read -r answer || answer=n
    case "$answer" in y|Y|yes|YES|Yes) return 0 ;; *) return 1 ;; esac
}

as_root() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
    elif command -v sudo >/dev/null 2>&1; then
        sudo "$@"
    else
        printf 'sudo is required to install system packages; ask your administrator.\n' >&2
        exit 1
    fi
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --prefix) [ "$#" -ge 2 ] || { usage >&2; exit 2; }; prefix=$2; shift 2 ;;
        --ref) [ "$#" -ge 2 ] || { usage >&2; exit 2; }; ref=$2; shift 2 ;;
        --force) force=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) printf 'unknown argument: %s\n' "$1" >&2; usage >&2; exit 2 ;;
    esac
done

for command_name in id uname sha256sum install mktemp; do
    command -v "$command_name" >/dev/null 2>&1 || {
        printf 'required command not found: %s\n' "$command_name" >&2
        exit 1
    }
done

[ "$(uname -s)" = Linux ] || {
    printf 'This source installer currently supports Linux.\n' >&2
    exit 1
}

bin_dir=$prefix/bin
share_dir=$prefix/share/gqodb
for binary in gqodb-codec ob_store; do
    destination=$bin_dir/$binary
    if { [ -e "$destination" ] || [ -L "$destination" ]; } && [ "$force" -ne 1 ]; then
        printf 'refusing to replace %s; rerun with --force if intended\n' "$destination" >&2
        exit 1
    fi
done

work=$(mktemp -d "${TMPDIR:-/tmp}/gqodb-install.XXXXXX")
trap 'rm -rf "$work"' EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM

packages=
command -v git >/dev/null 2>&1 || packages="$packages git"
command -v curl >/dev/null 2>&1 || packages="$packages curl"
if ! command -v cc >/dev/null 2>&1 || ! command -v make >/dev/null 2>&1; then
    packages="$packages build-essential"
elif command -v dpkg-query >/dev/null 2>&1 &&
    [ "$(dpkg-query -W -f='${db:Status-Status}' libc6-dev 2>/dev/null || true)" != installed ]; then
    packages="$packages build-essential"
fi
if [ ! -s /etc/ssl/certs/ca-certificates.crt ] && [ ! -s /etc/pki/tls/certs/ca-bundle.crt ]; then
    packages="$packages ca-certificates"
fi
if [ -n "$packages" ]; then
    command -v apt-get >/dev/null 2>&1 || {
        printf 'Missing prerequisites:%s\nInstall equivalent packages for your Linux distribution and rerun.\n' "$packages" >&2
        exit 1
    }
    if ! confirm "Install missing system packages:$packages (apt-get update + install, using sudo if needed)?"; then
        printf 'System-package installation declined; stopping.\n' >&2
        exit 1
    fi
    as_root apt-get update
    # These package names are fixed identifiers, not user-provided shell text.
    as_root apt-get install -y --no-install-recommends $packages
fi
for command_name in git curl cc make; do
    command -v "$command_name" >/dev/null 2>&1 || {
        printf 'required command still missing: %s\n' "$command_name" >&2
        exit 1
    }
done

cargo_dir=${CARGO_HOME:-"$HOME/.cargo"}
if ! command -v rustup >/dev/null 2>&1 && [ -x "$cargo_dir/bin/rustup" ]; then
    PATH="$cargo_dir/bin:$PATH"
    export PATH
fi
use_rustup=1
if ! rustup run "$toolchain" rustc --version >/dev/null 2>&1; then
    current_rustc=$(rustc --version 2>/dev/null || true)
    case "$current_rustc" in
        "rustc $toolchain "*)
            command -v cargo >/dev/null 2>&1 || {
                printf 'The matching active Rust installation has no cargo; repair it first.\n' >&2
                exit 1
            }
            use_rustup=0
            ;;
        *)
            if command -v rustup >/dev/null 2>&1; then
                if ! confirm "Install Rust $toolchain with your existing Rustup (keeping your default toolchain)?"; then
                    printf 'Rust toolchain installation declined; stopping.\n' >&2
                    exit 1
                fi
                rustup toolchain install "$toolchain" --profile minimal
            else
                if ! confirm "Install Rustup and Rust $toolchain for your user from https://sh.rustup.rs (no sudo)?"; then
                    printf 'Rust installation declined; stopping.\n' >&2
                    exit 1
                fi
                curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs -o "$work/rustup-init.sh"
                sh "$work/rustup-init.sh" -y --no-modify-path --profile minimal --default-toolchain "$toolchain"
                PATH="$cargo_dir/bin:$PATH"
                export PATH
            fi
            rustup run "$toolchain" rustc --version
            ;;
    esac
fi

run_rust() {
    if [ "$use_rustup" -eq 1 ]; then
        rustup run "$toolchain" "$@"
    else
        "$@"
    fi
}

source_dir=$work/source

printf 'Cloning %s\n' "$repo"
git clone --quiet --filter=blob:none --no-tags "$repo" "$source_dir"
git -C "$source_dir" fetch --quiet origin "$ref"
git -C "$source_dir" checkout --quiet --detach FETCH_HEAD
commit=$(git -C "$source_dir" rev-parse HEAD)

printf 'Testing commit %s with Rust %s\n' "$commit" "$toolchain"
(cd "$source_dir" && CARGO_TARGET_DIR="$source_dir/target" run_rust cargo test --workspace --all-targets --locked)
(cd "$source_dir" && CARGO_TARGET_DIR="$source_dir/target" run_rust cargo build --workspace --bins --release --locked)

bin_dir=$prefix/bin
share_dir=$prefix/share/gqodb
for binary in gqodb-codec ob_store; do
    destination=$bin_dir/$binary
    if { [ -e "$destination" ] || [ -L "$destination" ]; } && [ "$force" -ne 1 ]; then
        printf 'refusing to replace %s; rerun with --force if intended\n' "$destination" >&2
        exit 1
    fi
done

install -d "$bin_dir" "$share_dir"
for binary in gqodb-codec ob_store; do
    install -m 0755 "$source_dir/target/release/$binary" "$bin_dir/$binary"
done
install -m 0644 "$source_dir/LICENSE" "$source_dir/NOTICE" "$share_dir/"

receipt=$share_dir/install-receipt.txt
{
    printf 'repository=%s\n' "$repo"
    printf 'requested_ref=%s\n' "$ref"
    printf 'commit=%s\n' "$commit"
    printf 'rustc=%s\n' "$(run_rust rustc --version)"
    printf 'cargo=%s\n' "$(run_rust cargo --version)"
    sha256sum "$bin_dir/gqodb-codec" "$bin_dir/ob_store"
} > "$receipt"
chmod 0644 "$receipt"

printf 'Installed open-layer tools in %s\n' "$bin_dir"
printf 'Receipt: %s\n' "$receipt"
printf 'Add %s to PATH if needed.\n' "$bin_dir"
