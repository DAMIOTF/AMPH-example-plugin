#!/usr/bin/env bash
# Verify a published GitHub release the same way AMPH discovers it:
# latest-release API metadata, exact asset names, GitHub-provided sha256 digests,
# and byte-for-byte downloads.
#
# Usage:
#   ./scripts/verify-release.sh v1.0.1 [owner/repo]
#
# Requires: curl, python3, sha256sum. Set GH_TOKEN for a higher API rate limit.
set -euo pipefail

tag="${1:-}"
repo="${2:-DAMIOTF/AMPH-example-plugin}"
api_version=2022-11-28

fail() {
    printf 'verify-release: %s\n' "$*" >&2
    exit 1
}

[[ -n "$tag" ]] || fail "usage: $0 <tag> [owner/repo]"
command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v python3 >/dev/null 2>&1 || fail "python3 is required"
command -v sha256sum >/dev/null 2>&1 || fail "sha256sum is required"

tmpdir=$(mktemp -d)
trap 'rm -rf -- "$tmpdir"' EXIT

json="$tmpdir/release.json"
assets="$tmpdir/assets.tsv"
api="https://api.github.com/repos/${repo}/releases/latest"

auth=()
if [[ -n "${GH_TOKEN:-}" ]]; then
    auth=(-H "Authorization: Bearer ${GH_TOKEN}")
fi

# Asset digests are computed by GitHub asynchronously after upload. Poll for a
# short period so a release created in CI is not reported as broken too early.
# The endpoint is `latest` because that is exactly what AMPH resolves.
ready=0
for attempt in $(seq 1 20); do
    if curl --fail --silent --show-error --location \
        --proto '=https' --proto-redir '=https' \
        --connect-timeout 10 --max-time 30 \
        --max-filesize 2097152 \
        -H 'Accept: application/vnd.github+json' \
        -H "X-GitHub-Api-Version: ${api_version}" \
        "${auth[@]}" \
        "$api" -o "$json"; then
        if python3 - "$json" "$tag" "$repo" >"$assets" 2>"$tmpdir/error" <<'PY'
import json
import re
import sys

path, tag, repo = sys.argv[1:4]
try:
    with open(path, encoding="utf-8") as handle:
        release = json.load(handle)
except (OSError, json.JSONDecodeError) as error:
    print(f"cannot parse release JSON: {error}", file=sys.stderr)
    raise SystemExit(1)

if not isinstance(release, dict):
    print("release JSON is not an object", file=sys.stderr)
    raise SystemExit(1)
if release.get("tag_name") != tag:
    print(f"release tag is {release.get('tag_name')!r}, expected {tag!r}", file=sys.stderr)
    raise SystemExit(1)
if release.get("draft") or release.get("prerelease"):
    print("release is a draft or prerelease; AMPH installs stable releases only", file=sys.stderr)
    raise SystemExit(1)

assets = release.get("assets")
if not isinstance(assets, list):
    print("release has no asset list", file=sys.stderr)
    raise SystemExit(1)

rows = []
for name in ("plugin.json", "plugin.wasm"):
    matches = [
        asset
        for asset in assets
        if isinstance(asset, dict) and asset.get("name") == name
    ]
    if len(matches) != 1:
        print(f"expected exactly one {name} asset, found {len(matches)}", file=sys.stderr)
        raise SystemExit(1)
    asset = matches[0]

    digest = str(asset.get("digest") or "").lower()
    if not re.fullmatch(r"sha256:[0-9a-f]{64}", digest):
        print(f"asset {name} has no usable GitHub sha256 digest", file=sys.stderr)
        raise SystemExit(1)

    url = str(asset.get("browser_download_url") or "")
    prefix = f"https://github.com/{repo}/releases/download/{tag}/"
    if not url.startswith(prefix):
        print(f"asset {name} URL does not belong to {repo} {tag}", file=sys.stderr)
        raise SystemExit(1)

    size = asset.get("size")
    if not isinstance(size, int) or isinstance(size, bool) or size < 0:
        print(f"asset {name} has an invalid size", file=sys.stderr)
        raise SystemExit(1)

    rows.append((name, url, digest, size))

for row in rows:
    print("\t".join(str(value) for value in row))
PY
        then
            ready=1
            break
        fi
        printf 'verify-release: attempt %s/20: %s\n' \
            "$attempt" "$(tr '\n' ' ' <"$tmpdir/error")" >&2
    else
        printf 'verify-release: attempt %s/20: cannot fetch %s\n' "$attempt" "$api" >&2
    fi
    sleep 2
done

if [[ "$ready" -ne 1 ]]; then
    fail "release ${tag} of ${repo} is not valid for AMPH (missing assets, digests, or not stable)"
fi

while IFS=$'\t' read -r name url digest size; do
    case "$name" in
        plugin.json) limit=65536 ;;
        plugin.wasm) limit=2097152 ;;
        *) fail "unexpected asset in verification stream: $name" ;;
    esac

    ((size <= limit)) || fail "asset $name is ${size} bytes; limit is ${limit}"
    download="$tmpdir/$name"
    curl --fail --silent --show-error --location \
        --proto '=https' --proto-redir '=https' \
        --connect-timeout 10 --max-time 60 \
        --max-filesize "$limit" \
        "$url" -o "$download"

    actual_size=$(wc -c <"$download" | tr -d '[:space:]')
    [[ "$actual_size" == "$size" ]] || \
        fail "asset $name size mismatch: API ${size}, download ${actual_size}"

    actual_digest="sha256:$(sha256sum "$download" | cut -d' ' -f1)"
    [[ "$actual_digest" == "$digest" ]] || \
        fail "asset $name digest mismatch: API ${digest}, download ${actual_digest}"

    printf 'verify-release: %s OK (%s bytes, %s)\n' "$name" "$actual_size" "$actual_digest"
done <"$assets"

printf 'verify-release: %s %s is a valid AMPH release (plugin.json + plugin.wasm + digests)\n' \
    "$repo" "$tag"
