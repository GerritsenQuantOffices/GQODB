#!/bin/sh
# Fetch (or adopt a local copy of) one Binance Vision monthly aggTrades archive and verify it.
#
#   fetch.sh SYMBOL YYYY-MM [--out DIR] [--zip LOCAL.zip --checksum LOCAL.CHECKSUM] [--no-pin]
#
# Downloads the zip and its .CHECKSUM from data.binance.vision, compares the sha256 of
# the zip with the .CHECKSUM value (and with the value pinned in the B17 protocol for the
# two registered months), and REFUSES (exit 3) on any mismatch. With --zip the download
# is skipped and the local copy is verified the same way. No data is ever written inside
# the repository by default: the default output directory is the ignored artifacts/ tree.
# A receipt <zip>.fetch.json records the URL and the sha256 for the benchmark receipt.
set -eu

host=${B17_FETCH_BASE:-https://data.binance.vision}
prefix=data
usage() { printf 'usage: %s SYMBOL YYYY-MM [--out DIR] [--zip FILE --checksum FILE] [--no-pin]\n' "$0" >&2; exit 2; }

[ $# -ge 2 ] || usage
symbol=$1; month=$2; shift 2
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
out="$root/artifacts/public-trades"
local_zip=""; local_sum=""; pin=1
while [ $# -gt 0 ]; do
    case "$1" in
        --out) [ $# -ge 2 ] || usage; out=$2; shift 2 ;;
        --zip) [ $# -ge 2 ] || usage; local_zip=$2; shift 2 ;;
        --checksum) [ $# -ge 2 ] || usage; local_sum=$2; shift 2 ;;
        --no-pin) pin=0; shift ;;
        *) usage ;;
    esac
done
case "$symbol" in *[!A-Z0-9]*|"") usage ;; esac
case "$month" in [0-9][0-9][0-9][0-9]-[0-9][0-9]) ;; *) usage ;; esac

name="$symbol-aggTrades-$month.zip"
key="$prefix/spot/monthly/aggTrades/$symbol/$name"
url="$host/$key"
mkdir -p "$out"
zip="$out/$name"
sumfile="$zip.CHECKSUM"

sha256_of() {
    if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
    else shasum -a 256 "$1" | cut -d' ' -f1; fi
}
download() { # url destination
    if command -v curl >/dev/null 2>&1; then curl -fsSL --retry 3 --max-time 1800 -o "$2" "$1"
    else wget -q -O "$2" "$1"; fi
}

if [ -n "$local_zip" ]; then
    [ -n "$local_sum" ] || { printf 'fetch: --zip needs --checksum\n' >&2; exit 2; }
    [ -f "$local_zip" ] && [ -f "$local_sum" ] || { printf 'fetch: local file missing\n' >&2; exit 2; }
    zip=$local_zip; sumfile=$local_sum; source_kind=local
else
    [ ! -e "$zip" ] || { printf 'fetch: %s exists; remove it or use --zip\n' "$zip" >&2; exit 2; }
    download "$url" "$zip" || { printf 'fetch: download failed: %s\n' "$url" >&2; rm -f "$zip"; exit 4; }
    download "$url.CHECKSUM" "$sumfile" || { printf 'fetch: checksum download failed\n' >&2; exit 4; }
    source_kind=download
fi

actual=$(sha256_of "$zip")
declared=$(cut -d' ' -f1 < "$sumfile" | tr -d '\r\n ')
if [ "$actual" != "$declared" ]; then
    printf 'fetch: REFUSED, sha256 %s does not match the declared %s\n' "$actual" "$declared" >&2
    [ "$source_kind" = download ] && rm -f "$zip" "$sumfile"
    exit 3
fi

pinned=none
if [ "$pin" -eq 1 ]; then
    case "$symbol-$month" in
        PAXGUSDT-2025-10) pinned=de68061c03e6c7c5487dd805bf2b6b24348dee02ec33db1a4f33d2ecb8f1ad52 ;;
        PAXGUSDT-2026-02) pinned=effc512bdc63dcd1c023d1e00c55101f9ad93bd3002698c9965796e542881bd5 ;;
    esac
    if [ "$pinned" != none ] && [ "$actual" != "$pinned" ]; then
        printf 'fetch: REFUSED, sha256 %s differs from the value pinned in the protocol (%s)\n' "$actual" "$pinned" >&2
        [ "$source_kind" = download ] && rm -f "$zip" "$sumfile"
        exit 3
    fi
fi

bytes=$(wc -c < "$zip" | tr -d ' ')
receipt="$out/$name.fetch.json"
printf '{"symbol":"%s","month":"%s","host":"%s","key":"%s","source":"%s","zip_sha256":"%s","zip_bytes":%s,"declared_sha256":"%s","pinned_sha256":"%s"}\n' \
    "$symbol" "$month" "${host#https://}" "$key" "$source_kind" "$actual" "$bytes" "$declared" "$pinned" > "$receipt"
printf 'fetch: verified %s (%s bytes, sha256 %s)\n' "$name" "$bytes" "$actual"
printf '%s\n' "$zip"
