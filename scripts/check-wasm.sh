#!/usr/bin/env bash
# Validate a built AMPH plugin module without relying on AMPH being installed.
#
# This checks the properties that are cheap and unambiguous to inspect:
#   - the bytes pass strict WABT validation with reference types disabled;
#   - no imports (WASI, wasm-bindgen, host functions);
#   - no start function;
#   - exactly one linear memory, exported as "memory";
#   - the three ABI functions are exported;
#   - the module is within AMPH's 2 MiB module limit.
#
# WABT is required for the strict check. `wasm-validate
# --disable-reference-types` catches the overlong call_indirect table-index LEB
# that AMPH's wasmparser rejects. AMPH's own wasm validator remains
# authoritative for exact function signatures, memory limits and ABI behavior.
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

for tool in wasm-validate wasm2wat; do
    command -v "$tool" >/dev/null 2>&1 || \
        fail "WABT is required for the strict AMPH check; install it (for example: sudo pacman -S wabt)"
done

wasm-validate --disable-reference-types "$wasm" || \
    fail "strict WABT validation failed (reference types disabled; overlong call_indirect LEB or another unsupported encoding)"
text=$(wasm2wat "$wasm") || fail "wasm2wat rejected the module"

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
