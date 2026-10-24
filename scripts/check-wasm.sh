#!/usr/bin/env bash
# Validate a built AMPH plugin module without relying on AMPH being installed.
#
# This checks the properties that are cheap and unambiguous to inspect:
#   - the bytes are a valid WebAssembly module;
#   - no imports (WASI, wasm-bindgen, host functions);
#   - no start function;
#   - exactly one linear memory, exported as "memory";
#   - the three ABI functions are exported;
#   - the module is within AMPH's 2 MiB module limit.
#
# AMPH's own wasm validator remains authoritative for exact function
# signatures, memory limits, feature set and ABI version behavior.
set -euo pipefail

wasm="${1:-dist/plugin.wasm}"
limit=$((2 * 1024 * 1024))

fail() {
    printf 'check-wasm: %s\n' "$*" >&2
    exit 1
}

[[ -f "$wasm" ]] || fail "missing module: $wasm"

size=$(wc -c <"$wasm" | tr -d '[:space:]')
((size <= limit)) || fail "module is ${size} bytes; AMPH limit is ${limit} bytes"

if command -v wasm-tools >/dev/null 2>&1; then
    wasm-tools validate "$wasm" || fail "wasm-tools rejected the module"
    text=$(wasm-tools print "$wasm")
elif command -v wasm-validate >/dev/null 2>&1 && command -v wasm2wat >/dev/null 2>&1; then
    wasm-validate "$wasm" || fail "wasm-validate rejected the module"
    text=$(wasm2wat "$wasm")
else
    fail "install wasm-tools (cargo install wasm-tools) or WABT (wasm-validate + wasm2wat)"
fi

if grep -Eq '\(import[[:space:]]' <<<"$text"; then
    fail "module contains imports; AMPH forbids every import (including WASI)"
fi

if grep -Eq '\(start[[:space:]]' <<<"$text"; then
    fail "module contains a start function; AMPH forbids start functions"
fi

memories=$(grep -Ec '^[[:space:]]*\(memory[[:space:]]' <<<"$text" || true)
[[ "$memories" -eq 1 ]] || fail "expected exactly one linear memory, found ${memories}"

for export in memory amph_abi_version amph_alloc amph_analyze; do
    if ! grep -Fq "(export \"$export\"" <<<"$text"; then
        fail "missing ABI export: $export"
    fi
done

# Reject optional proposals that AMPH disables even when the validator accepts
# them. A plain Rust wasm32-unknown-unknown build should never emit these.
if grep -Eq 'v128|ref\.|return_call|memory\.atomic' <<<"$text"; then
    fail "module uses a vector/reference/tail-call/atomic feature outside the AMPH subset"
fi

printf 'check-wasm: OK (%s bytes, no imports, no start, ABI exports present)\n' "$size"
