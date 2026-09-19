#!/bin/sh
# Build, test and install the two open GQODB command-line tools from source.
# This script never installs the commercial engine and never uses sudo.
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

Existing binaries are not replaced unless --force is given. No sudo is used.
EOF
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

for command_name in git rustup sha256sum install mktemp; do
    command -v "$command_name" >/dev/null 2>&1 || {
        printf 'required command not found: %s\n' "$command_name" >&2
        exit 1
    }
done

rustup run "$toolchain" rustc --version >/dev/null 2>&1 || {
    printf 'Rust toolchain %s is required but not installed.\n' "$toolchain" >&2
    printf 'Install it explicitly with: rustup toolchain install %s --profile minimal\n' "$toolchain" >&2
    exit 1
}

work=$(mktemp -d "${TMPDIR:-/tmp}/gqodb-install.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM
source_dir=$work/source

printf 'Cloning %s\n' "$repo"
git clone --quiet --filter=blob:none --no-tags "$repo" "$source_dir"
git -C "$source_dir" fetch --quiet origin "$ref"
git -C "$source_dir" checkout --quiet --detach FETCH_HEAD
commit=$(git -C "$source_dir" rev-parse HEAD)

printf 'Testing commit %s with Rust %s\n' "$commit" "$toolchain"
(cd "$source_dir" && rustup run "$toolchain" cargo test --workspace --all-targets --locked)
(cd "$source_dir" && rustup run "$toolchain" cargo build --workspace --bins --release --locked)

bin_dir=$prefix/bin
share_dir=$prefix/share/gqodb
for binary in gqodb-codec ob_store; do
    destination=$bin_dir/$binary
    if [ -e "$destination" ] && [ "$force" -ne 1 ]; then
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
    printf 'rustc=%s\n' "$(rustup run "$toolchain" rustc --version)"
    printf 'cargo=%s\n' "$(rustup run "$toolchain" cargo --version)"
    sha256sum "$bin_dir/gqodb-codec" "$bin_dir/ob_store"
} > "$receipt"
chmod 0644 "$receipt"

printf 'Installed open-layer tools in %s\n' "$bin_dir"
printf 'Receipt: %s\n' "$receipt"
printf 'Add %s to PATH if needed.\n' "$bin_dir"
