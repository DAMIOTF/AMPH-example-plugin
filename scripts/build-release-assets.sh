#!/usr/bin/env bash
# Build and stage the two release assets expected by AMPH:
#
#   dist/plugin.json
#   dist/plugin.wasm
#
# This script does not create a GitHub release and does not commit anything.
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd -- "$script_dir/.." && pwd)
target=wasm32-unknown-unknown

cd "$repo_root"

command -v cargo >/dev/null 2>&1 || {
    printf 'build-release-assets: cargo is not installed\n' >&2
    exit 1
}

libdir=$(rustc --print target-libdir --target "$target" 2>/dev/null || true)
if [[ -z "$libdir" || ! -d "$libdir" ]]; then
    printf 'build-release-assets: target %s is not installed.\n' "$target" >&2
    printf 'Install it with: rustup target add %s\n' "$target" >&2
    exit 1
fi

lock_args=()
[[ -f Cargo.lock ]] && lock_args=(--locked)

# AMPH disables Wasm reference types. Some Rust/LLVM outputs encode the
# call_indirect table index as an overlong 5-byte LEB128 zero; wasmparser
# rejects that encoding at the strict AMPH boundary. Forcing the MVP CPU keeps
# the table index in the minimal MVP form.
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-Ctarget-cpu=mvp"
printf 'build-release-assets: RUSTFLAGS=%s\n' "$RUSTFLAGS"

cargo build --release --target "$target" "${lock_args[@]}"

artifact="target/${target}/release/amph_example_plugin.wasm"
[[ -f "$artifact" ]] || {
    printf 'build-release-assets: expected artifact missing: %s\n' "$artifact" >&2
    exit 1
}

rm -rf -- "$repo_root/dist"
mkdir -p -- "$repo_root/dist"
cp -- "$repo_root/plugin.json" "$repo_root/dist/plugin.json"
cp -- "$artifact" "$repo_root/dist/plugin.wasm"

# Some Rust/LLVM builds encode the call_indirect table index as a five-byte
# LEB128 zero. AMPH's strict validator requires the canonical MVP encoding, and
# -Ctarget-cpu=mvp alone did not fix this toolchain output. A WABT binary ->
# text -> binary roundtrip rewrites the overlong LEBs to their canonical form.
for tool in wasm2wat wat2wasm wasm-validate; do
    command -v "$tool" >/dev/null 2>&1 || {
        printf 'build-release-assets: %s is required for WABT canonicalization and strict validation.\n' "$tool" >&2
        printf 'Install WABT (for example: sudo pacman -S wabt).\n' >&2
        exit 1
    }
done

roundtrip_dir=$(mktemp -d "$repo_root/dist/.wabt-roundtrip.XXXXXX")
cleanup_roundtrip() {
    rm -rf -- "$roundtrip_dir"
}
trap cleanup_roundtrip EXIT

printf 'build-release-assets: canonicalizing dist/plugin.wasm with WABT\n'
wasm2wat "$repo_root/dist/plugin.wasm" -o "$roundtrip_dir/plugin.wat" || {
    printf 'build-release-assets: wasm2wat failed to decode dist/plugin.wasm\n' >&2
    exit 1
}
wat2wasm "$roundtrip_dir/plugin.wat" -o "$roundtrip_dir/plugin.wasm" || {
    printf 'build-release-assets: wat2wasm failed to encode the canonical module\n' >&2
    exit 1
}
mv -f -- "$roundtrip_dir/plugin.wasm" "$repo_root/dist/plugin.wasm"

"$script_dir/check-wasm.sh" "$repo_root/dist/plugin.wasm"

printf '\nRelease assets staged in %s:\n' "$repo_root/dist"
(cd "$repo_root/dist" && sha256sum plugin.json plugin.wasm)
printf '\nLocal SHA-256 values are for inspection only. AMPH reads the digest\n'
printf 'that GitHub computes for each uploaded release asset, not a checksum file.\n'
