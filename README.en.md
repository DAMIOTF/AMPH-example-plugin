# AMPH Example Plugin — `example.sizes`

[Wersja polska](README.md)

A standalone, minimal example plugin repository for
[AMPH](https://github.com/DAMIOTF/Arch-Meta-Package-Harmonizer). It documents
and implements the complete **functionality** plugin contract of AMPH
0.3.0-beta:

- `plugin.json` manifest schema **1**;
- ABI **v1** (`amph_abi_version`, `amph_alloc`, `amph_analyze`);
- analysis of the package snapshot handed over by AMPH;
- a binary WebAssembly module built for `wasm32-unknown-unknown`;
- zero imports, no WASI, no `wasm-bindgen`, no host capabilities;
- GitHub Release distribution with `plugin.json` and `plugin.wasm`, both
  carrying GitHub-provided `sha256:<64 hex>` digests.

This repository is independent of the AMPH checkout and does not modify it.

AMPH wires the real plugin manager (`StorePluginHandler`) by default. Once
installed, the plugin appears in the manager and can be managed from the TUI.

## One repository = one plugin

AMPH resolves **one** `plugin.json` from the **latest stable** GitHub Release of
a repository. Therefore one repository publishes **one plugin at a time**.

- The root of this repository publishes the functionality plugin
  `example.sizes`.
- `examples/theme/plugin.json` is reference material only and is **not**
  included in the release.
- Publish a theme in a separate repository whose root `plugin.json` is the
  theme manifest.

Do not upload two manifests to one release. The backend deliberately picks the
single asset named `plugin.json`.

## Contents

- [Quick start](#quick-start)
- [Repository layout](#repository-layout)
- [Manifest `plugin.json`](#manifest-pluginjson)
- [Security and sandbox](#security-and-sandbox)
- [ABI v1 and input](#abi-v1-and-input)
- [Example analyzer](#example-analyzer)
- [Building a valid module](#building-a-valid-module)
- [Publishing on GitHub](#publishing-on-github)
- [AMPH plugin manager](#amph-plugin-manager)
- [Limits and errors](#limits-and-errors)
- [Compatibility](#compatibility)
- [Development, tests and CI](#development-tests-and-ci)
- [License](#license)

## Quick start

Requirements: Rust with `wasm32-unknown-unknown`, `cargo`, and either
`wasm-tools` or WABT (`wasm-validate` + `wasm2wat`). GitHub CLI (`gh`) or the
GitHub web UI is needed to publish.

```sh
rustup target add wasm32-unknown-unknown
cargo fmt --all -- --check
cargo check --locked
cargo test --locked --lib
./scripts/build-release-assets.sh

# After publishing:
./scripts/verify-release.sh v1.0.0 DAMIOTF/AMPH-example-plugin
```

Install in AMPH:

```text
Shift+P -> "Add" tab -> paste https://github.com/DAMIOTF/AMPH-example-plugin
        -> Enter (review) -> Enter (install)
```

Then in **Installed**:

- `a` — analyze the current snapshot (functionality; review and confirm),
- `space` — enable/disable,
- `r` — remove (with confirmation),
- `T` — apply a theme to the current session,
- `c` — check for updates,
- `U` — update all available plugins.

The URLs assume `DAMIOTF/AMPH-example-plugin`. If the repository uses another
owner or name, update `Cargo.toml`, the READMEs, scripts and workflow.

## Repository layout

```text
AMPH-example-plugin/
├── Cargo.toml                       # cdylib + rlib; release profile for Wasm
├── Cargo.lock                       # no runtime dependencies
├── plugin.json                      # the ONLY published manifest
├── src/lib.rs                       # ABI v1, snapshot parser, tests
├── examples/theme/
│   ├── plugin.json                  # reference theme, NOT published here
│   └── README.md                    # how to publish a theme separately
├── scripts/
│   ├── build-release-assets.sh      # build and stage dist/
│   ├── check-wasm.sh                # imports/start/exports/size checks
│   └── verify-release.sh            # GitHub digests and downloads
├── .github/workflows/
│   ├── ci.yml
│   └── release.yml
├── README.md                        # Polish primary
├── README.en.md                     # this file
└── LICENSE                          # MIT
```

## Manifest `plugin.json`

The manifest is strict JSON. Unknown fields, wrong types, unsafe identifiers
and kind-incompatible fields are rejected. It is the release asset named
exactly `plugin.json`, at most **64 KiB**.

### Common fields

| Field | Required | Rules |
|---|---:|---|
| `schema` | yes | Exactly `1`. |
| `id` | yes | 1–64 chars; lowercase ASCII, digits, `-`, `_`, `.`; must start and end alphanumeric; no `..`. Also the store directory name. |
| `name` | yes | 1–80 chars; no leading/trailing whitespace or control characters. |
| `version` | yes | 1–64 chars; ASCII letters, digits, `.`, `-`, `+`, `_`. This is the plugin version, not the ABI. |
| `kind` | yes | `"theme"` or `"functionality"`. |
| `description` | no | 0–512 chars; no control characters. |
| `author` | no | 0–80 chars; no control characters. |

Kind-specific rules:

- `theme`: requires `theme`, forbids `api` and `entry`;
- `functionality`: requires `api: 1` and an `entry` ending in `.wasm`, forbids
  `theme`.

`entry` is a bare asset file name (no `/`, `\`, NUL), 1–128 chars, with ASCII
letters, digits, `.`, `-`, `_`, `+`. The names `plugin.json`, `install.json` and
`disabled` are reserved.

### Functionality manifest (published here)

```json
{
  "schema": 1,
  "id": "example.sizes",
  "name": "Example Snapshot Analyzer",
  "version": "1.0.0",
  "kind": "functionality",
  "description": "Summarizes the AMPH package snapshot: totals, largest package, explicit and foreign counts, and dependency links.",
  "author": "AMPH example contributors",
  "api": 1,
  "entry": "plugin.wasm"
}
```

The release must contain `plugin.json` and the asset named by `entry` — here
`plugin.wasm`. Both need GitHub digests.

### Theme manifest (reference only)

```json
{
  "schema": 1,
  "id": "example.theme",
  "name": "Example Gruvbox Theme",
  "version": "1.0.0",
  "kind": "theme",
  "theme": {
    "palette": "gruvbox",
    "accent": "#83A598",
    "good": "#B8BB26",
    "warn": "#FABD2F",
    "bad": "#FB4934",
    "muted": "#928374",
    "info": "#458588"
  }
}
```

Theme rules:

- `palette` must be built in: `catppuccin`, `tokyo-night`, `gruvbox` or `nord`
  (the backend also accepts `mocha`, `tokyo`, `tokyonight`);
- `accent`, `good`, `warn`, `bad`, `muted`, `info` accept **exactly**:
  - a base ANSI color name: `black`, `red`, `green`, `yellow`, `blue`,
    `magenta`, `cyan`, `white`, `gray`/`grey`, `dark gray`/`dark grey`,
    `bright black`, `bright red`, `bright green`, `bright yellow`,
    `bright blue`, `bright magenta`, `bright cyan`, `bright white`, or
  - an exact `#RRGGBB` value: `#` plus six hex digits (case-insensitive);
    `#RGB`, `rgb(...)` and arbitrary CSS names are rejected;
- defaults are `cyan`, `green`, `yellow`, `red`, `bright black`, `blue`;
- the `theme` object rejects unknown fields;
- a theme has no executable entry, hence no `api`/`entry`.

The backend validates colors when parsing the manifest and again when applying
the theme. A theme defines only semantic interface colors. It **cannot** define
custom screens, widgets, commands, shortcuts, permissions or TUI behavior.

### Forbidden manifest content

There are no permission fields. A plugin cannot request files, network,
processes, environment variables, the clock, clipboard or host callbacks. There
are no install scripts, dependency URLs, shell hooks, `preinstall`/`postinstall`
hooks or screen definitions. Adding e.g. `"permissions": ["network"]` fails
with `unknown field`.

## Security and sandbox

A functionality plugin is a stateless analyze hook executed by the `wasmi`
interpreter — no JIT and no native code. The host runs analyze only when the
user explicitly selects an enabled functionality plugin and confirms the `a`
action. Install, refresh, update checks and AMPH startup never send the package
snapshot to a plugin.

The confirmed snapshot contains only package data: `name`, `version`, `size`,
`explicit`, `foreign`, `depends`, `provides` plus `scanned_at`. It contains no
paths, handles, environment variables, configuration, pacman history or host
capabilities. The confirmation screen shows the package count before sending.

Sandbox rules:

- **no imports** — the host linker is empty; any import, including
  `wasi_snapshot_preview1`, rejects the module;
- **no start function**;
- **32-bit linear memory only** — no `memory64`;
- **no WASI** — no args, env, clock, random, file descriptors;
- **no reference types, SIMD, tail calls, extended const, custom page sizes,
  multi-memory or wide arithmetic**;
- a practical core-Wasm subset is allowed: mutable globals, sign extension,
  saturating float-to-int, multi-value, bulk memory and floats;
- **no persistent state** — every `amph_analyze` gets a fresh `Store` and
  instance; memory and globals do not survive a call;
- **no host access** — there is no function to import;
- **bounded resources** — module, memory, input, output, fuel, recursion and
  instance counts are limited;
- **integrity** — HTTPS, exact GitHub URL, latest stable release, and matching
  SHA-256 digests for every selected asset;
- **atomicity** — staging plus `rename`; removal renames out of the visible
  namespace first; both symlinked plugin directories and symlinked store files
  (`plugin.json`, `install.json`, the `disabled` marker, and the functionality
  `.wasm` entry) are rejected.

This is narrower than an OS process sandbox: there is no host interface and no
I/O. The plugin is still untrusted code; review the repository you install from.
The analysis confirmation screen is the last line of defense, not a security
audit.

Runtime limits:

| Resource | Limit |
|---|---:|
| Manifest size | 64 KiB |
| Wasm module size | 2 MiB |
| Linear memory | 16 MiB |
| JSON input | 1 MiB |
| JSON output | 1 MiB |
| Fuel per analyze | 20,000,000 |
| Recursion depth | 256 |
| Memories / tables / instances | 1 / 1 / 2 |
| Table elements | 1024 |
| Network timeout | 20 s |

These limits are enforced before parsing, by `wasmi`, by declared memory, by a
resource limiter, by fuel and by JSON bounds.

## ABI v1 and input

Required exports:

| Export | Signature | Meaning |
|---|---|---|
| `memory` | 32-bit linear memory | Input/output buffer. |
| `amph_abi_version` | `() -> i32` | Must return `1`; checked on every analyze. |
| `amph_alloc` | `(i32) -> i32` | Reserve `len` bytes, return a pointer; negative means failure. |
| `amph_analyze` | `(i32, i32) -> i64` | Receive `(ptr, len)` JSON and return a packed result. |

The module must not import anything or define a start function. The host
validates the exact signatures.

### Analyze input

The exact envelope AMPH sends after the user confirms `a`:

```json
{
  "api": 1,
  "op": "analyze",
  "snapshot": {
    "scanned_at": "2026-10-25 00:00",
    "packages": [
      {
        "name": "linux",
        "version": "6.12.1",
        "size": 1073741824,
        "explicit": true,
        "foreign": false,
        "depends": ["glibc", "bash"],
        "provides": []
      },
      {
        "name": "example-foreign",
        "version": "2.0",
        "size": 1048576,
        "explicit": false,
        "foreign": true,
        "depends": [],
        "provides": ["example"]
      }
    ]
  }
}
```

Package fields:

| Field | Type | Meaning |
|---|---|---|
| `name` | string | Package name. |
| `version` | string | Installed version. |
| `size` | integer | Size in bytes. |
| `explicit` | bool | Whether it was explicitly installed. |
| `foreign` | bool | Whether it is foreign (outside repositories). |
| `depends` | string[] | Dependency names. |
| `provides` | string[] | Provided symbols. |

`snapshot.packages` is AMPH's current inventory; order is not part of the
contract. A plugin should tolerate extra object fields so a future host can add
metadata. This example does, while requiring `api=1`, `op="analyze"` and
`snapshot.packages`.

### Call sequence

1. Validate the module, create a fresh `Store` and instance.
2. Call `amph_abi_version()`, require `1`.
3. Call `amph_alloc(input.len())`; require a non-negative pointer within the
   memory after adding `len`.
4. Copy the JSON request bytes to the returned pointer.
5. Call `amph_analyze(ptr, len)`.
6. Unpack `packed = (output_ptr as u64) << 32 | output_len as u64`.
7. Check the output limit and bounds, and copy the bytes before dropping the
   instance.
8. The output must be one valid JSON value. Empty output, multiple values or
   non-JSON bytes are errors.
9. The next call gets a new instance; there is no cross-call state.

ABI v1 has no error channel. Return a valid JSON object if the request is bad.
AMPH does not interpret the response semantics; it only validates JSON and
limits.

### Response contract of this example

The example returns an object with `summary` (string) and `findings` (array).
The TUI uses `summary` as a short message and `findings` as details.

```json
{
  "summary": "Analyzed 2 packages; total 1 GiB; 1 explicit, 1 foreign; largest linux 6.12.1 (1 GiB).",
  "findings": [
    {
      "kind": "totals",
      "packages": 2,
      "bytes": 1074790400,
      "explicit": 1,
      "foreign": 1,
      "dependency_links": 2,
      "provide_links": 1
    },
    {
      "kind": "largest",
      "name": "linux",
      "version": "6.12.1",
      "bytes": 1073741824
    }
  ]
}
```

Empty snapshot:

```json
{"summary":"Analyzed 0 packages; the snapshot is empty.","findings":[{"kind":"totals","packages":0,"bytes":0,"explicit":0,"foreign":0,"dependency_links":0,"provide_links":0}]}
```

Malformed request:

```json
{"summary":"Invalid AMPH analyze request","detail":"expected api=1, op=analyze and snapshot.packages"}
```

This is not a schema imposed on every AMPH plugin — AMPH requires only one
valid JSON value. It is this example's contract, aligned with how the TUI
summarizes responses.

## Example analyzer

`src/lib.rs` is dependency-free:

- `#![no_std]` in the Wasm build — no `std`, WASI or system allocator;
- two static buffers: 1 MiB input and 256 KiB output;
- no imports and no start function;
- a small JSON parser for the `api/op/snapshot/packages` envelope;
- tolerates reordered fields and skips unknown fields;
- calculates package count, total size, explicit, foreign, dependency/provide
  link counts and the largest package;
- renders `summary` and `findings` without allocation;
- returns malformed requests as valid JSON.

ABI essentials:

```rust
#[no_mangle]
pub extern "C" fn amph_abi_version() -> i32 {
    1
}

#[no_mangle]
pub extern "C" fn amph_alloc(len: i32) -> i32 {
    // ... validation and a pointer into the static buffer ...
}

#[no_mangle]
pub extern "C" fn amph_analyze(pointer: i32, length: i32) -> i64 {
    // ... snapshot -> summary/findings, then (ptr << 32) | len ...
}
```

What it cannot do:

- read files, the pacman database, the network, environment variables or the
  clock;
- receive paths, handles, permissions or configuration;
- retain state between analyses;
- define custom screens, widgets, commands or TUI shortcuts;
- run analyze without the user's `a` action and confirmation;
- change the interface on its own. Only `T` applies a declarative theme.

## Building a valid module

```sh
rustup target add wasm32-unknown-unknown
cargo fmt --all -- --check
cargo check --locked
cargo test --locked --lib
cargo build --release --target wasm32-unknown-unknown --locked
```

Artifact:

```text
target/wasm32-unknown-unknown/release/amph_example_plugin.wasm
```

Stage the exact release names:

```sh
./scripts/build-release-assets.sh
# dist/plugin.json
# dist/plugin.wasm
```

The script requires `wasm-tools` or WABT:

```sh
cargo install wasm-tools --locked
# or on Arch Linux:
sudo pacman -S wabt
```

It checks module validity, no imports, no start, exactly one memory, the memory
export, the three ABI function exports, and the 2 MiB limit. AMPH remains
authoritative for exact signatures, memory limits, features and ABI behavior.

Manual inspection:

```sh
wasm-tools validate dist/plugin.wasm
wasm-tools print dist/plugin.wasm | grep -E '\(import |\(start '
# The grep above should print nothing.

wasm-tools print dist/plugin.wasm | grep -E '\(export "(memory|amph_abi_version|amph_alloc|amph_analyze)"'
# Four exports.

wasm-objdump -x dist/plugin.wasm   # with WABT
```

The module must be binary and start with `\0asm`. Text WAT is not accepted as
the entry. Do not use a WASI target or `wasm-bindgen`: the resulting imports
are rejected.

`cargo test --locked --lib` is intentional: the Wasm `cdylib` is `no_std` with
`panic = "abort"`, while the Rust test harness needs a different setup. The
host tests cover the parser/renderer.

## Publishing on GitHub

Rules:

- publish only **stable** tags, preferably `vMAJOR.MINOR.PATCH`, e.g. `v1.0.0`;
  the workflow rejects `v1.0.0-rc.1`;
- `version` in `plugin.json` must match the tag (`v` + version);
- do not mark the release draft or prerelease — AMPH uses `releases/latest`.
  GitHub's "latest" marker is not automatically the highest semver; make sure
  the intended release is marked latest;
- functionality assets must be exactly `plugin.json` and `plugin.wasm`; a
  theme release contains only `plugin.json` (no Wasm), but that manifest still
  needs a digest;
- GitHub computes `sha256:<64 hex>` for each uploaded asset; AMPH reads that
  API field. A `plugin.wasm.sha256` file is not a substitute;
- every selected asset needs a digest, or installation is refused;
- one release = one plugin; do not add the reference theme manifest here.

`.github/workflows/release.yml` runs on `v*`, checks the stable tag and version
match, builds and validates the module, creates the release with `gh`, and
verifies the digests and downloaded bytes.

Repository requirements: Actions may create releases; the workflow requests
`contents: write`; `main` contains the commit; the tag points at that commit.

Once the GitHub remote exists:

```sh
git init -b main
git add .
git commit -m "Add AMPH example functionality plugin"
git remote add origin https://github.com/DAMIOTF/AMPH-example-plugin.git
git push -u origin main

git tag v1.0.0
git push origin v1.0.0
```

Manual release:

```sh
./scripts/build-release-assets.sh
gh release create v1.0.0 \
  dist/plugin.json \
  dist/plugin.wasm \
  --title "Example Snapshot Analyzer v1.0.0" \
  --notes "Example AMPH functionality plugin (ABI v1)."
```

In the web UI: **Releases → Draft a new release → choose the tag → attach
`plugin.json` and `plugin.wasm` → Publish**. Do not tick "prerelease" if AMPH
should see it.

After publishing:

```sh
./scripts/verify-release.sh v1.0.0 DAMIOTF/AMPH-example-plugin
```

The script resolves the latest stable release, then checks its tag,
draft/prerelease state, asset names, digests, URLs, sizes and SHA-256 of
downloaded bytes, with retries while GitHub computes digests. Set `GH_TOKEN` to
raise the API limit.

Do not silently replace assets under the same tag. The backend detects at check
time a change in the release tag, the `plugin.json` digest, and — for
functionality plugins — the selected Wasm entry digest, but new content should
still get a new version and tag. Bump `version` in `plugin.json` and
`Cargo.toml`, commit, tag `vX.Y.Z`, push the tag.

## AMPH plugin manager

AMPH wires `StorePluginHandler` and the per-user store at
`$XDG_DATA_HOME/amph/plugins` (fallback `~/.local/share/amph/plugins`). The
manager opens with `Shift+P` from any tab.

### Install

1. Start AMPH.
2. Press `Shift+P`.
3. Open **Add** (`2`).
4. Paste exactly `https://github.com/DAMIOTF/AMPH-example-plugin`. The backend
   accepts only `https://github.com/{owner}/{repo}` (one trailing slash is
   tolerated). HTTP, GitLab, extra path segments, queries, fragments,
   credentials and other hosts are rejected.
5. Use the exact GitHub `owner/repo` letter case. The backend compares the
   asset URL prefix with the supplied URL case-sensitively; different case can
   fail even if the browser link works.
6. Press `Enter` to review, then `Enter` to install.
7. AMPH fetches the latest stable release, verifies digests, validates the
   manifest and module, then publishes the plugin directory atomically.

Installed layout:

```text
${XDG_DATA_HOME:-$HOME/.local/share}/amph/plugins/example.sizes/
├── plugin.json       # verified manifest
├── plugin.wasm       # functionality entry
├── install.json      # provenance: repository, tag, time, SHA-256
└── disabled          # only when disabled
```

`install.json` records `repository`, `release_tag`, `manifest_sha256`,
`wasm_sha256`, etc. It is used for check/update.

### Installed shortcuts

| Key | Action |
|---|---|
| `j`/`k`, arrows | select a plugin |
| `space` | enable/disable the selected plugin |
| `a` | run the selected enabled functionality plugin's analyzer |
| `T` | apply the selected enabled theme plugin to the current session |
| `r` | remove the selected plugin (confirm with `Enter`) |
| `c` | check updates (switches to **Updates**) |
| `U` | update all plugins with available updates |
| `R` | refresh the list |
| `Tab`/`Shift+Tab` | switch sections |
| `Esc` | close the manager |

In **Add**, typing/pasting a URL, `Enter` (review) and `Esc` work. In
**Updates**, `c` checks, and `Enter` or `U` updates all. There is no separate
single-row update action.

### Analyze (`a`)

For a selected, **enabled** functionality plugin, press `a`:

1. AMPH shows `ANALYZE PACKAGES` with the plugin name and package count.
2. Nothing has been sent yet. Only `Enter` hands the current inventory
   snapshot to the plugin.
3. The host runs the module in the sandbox and displays `summary`/`findings`
   from the JSON response.
4. `Esc` cancels. Analyze does not run for a disabled plugin, a theme, or a
   plugin without an entry.

The snapshot contains only package `name`, `version`, `size`, `explicit`,
`foreign`, `depends`, `provides` plus `scanned_at`. It contains no paths,
handles, configuration or host capabilities. This is the only point at which
inventory is sent to a functionality plugin.

This does not create a plugin-defined screen. The TUI shows a bounded message
and detail text; the plugin cannot draw UI, open windows or run commands.

### Themes (`T`)

For a selected, **enabled** theme plugin, press `T`:

- AMPH re-validates the palette and colors from the manifest;
- the theme is applied to the **current session only**;
- nothing is written to configuration; restarting AMPH restores the previous
  theme;
- `NO_COLOR` wins; the palette stays monochrome;
- a disabled plugin, a functionality plugin, or missing theme data results in
  a message with no visual change.

This is a declarative change to six semantic colors, not plugin UI. A plugin
cannot define custom screens, widgets or arbitrary capabilities.

### Check updates

- `c` in **Installed** or **Updates** runs `check_all`.
- A check queries the latest stable release and compares the release tag, the
  `plugin.json` digest, and — for functionality plugins — the selected Wasm
  entry digest with `install.json`.
- "Update available" means a change in the release tag, the `plugin.json` digest,
  **or** (for functionality) the Wasm entry digest; this is not a semver
  comparison. Themes have no Wasm entry, so only the tag and `plugin.json`
  digest apply to them.
- A plugin without `install.json` is `NotTracked` and cannot be checked.
- Per-plugin errors are reported separately; the manager continues.

### Update all (`U`)

- `U` in **Installed** or **Updates** updates every plugin with an available
  update.
- `Enter` in **Updates** also runs update all (there is no separate single-row
  update).
- `update_all` continues after per-plugin failures and reports each result.
- Updates preserve the disabled marker.
- Publishing is atomic; a failed update should not leave a half-written plugin.

### Enable/disable (`space`)

In **Installed**, select a plugin and press `space`. AMPH calls `set_enabled`
and writes/removes an empty `disabled` marker in the plugin directory. The
state persists across AMPH restarts and is preserved by updates.

- a disabled functionality plugin cannot run `a`;
- a disabled theme plugin cannot be applied with `T`;
- disabling does not remove the plugin or change the manifest;
- do not edit `plugin.json`, `install.json` or the marker by hand — use
  `space`.

### Remove (`r`)

In **Installed**, select the plugin, press `r`, then confirm with `Enter`. AMPH
re-validates the manifest, renames the directory out of the visible namespace,
then deletes it. Removal is permanent.

### If the plugin store cannot be read

If AMPH cannot read the plugin store, it falls back to a read-only handler
and shows a warning. This is an emergency state (bad permissions, bad
`XDG_DATA_HOME`/`HOME`, corrupt directory), not the normal AMPH behavior.
Check the messages and permissions on `.../amph/plugins`.

## Limits and errors

Common message fragments:

| Message | Meaning / fix |
|---|---|
| `plugin source must be an https://github.com/{owner}/{repo} URL` | Use the exact public GitHub URL, no extra path/query. |
| `no 'plugin.json' asset` | The latest **stable** release lacks the manifest; check draft/prerelease state and exact asset name. |
| `no 'plugin.wasm' asset` | The manifest's `entry` is missing; names must match exactly. |
| `has no SHA-256 digest` | GitHub did not return a digest yet. Wait/check the API; a `.sha256` file is not used. |
| `release asset digest must use ...` | Malformed API digest; a release/metadata problem. |
| `SHA-256 mismatch` | Downloaded bytes differ from the digest. Retry; investigate replaced assets/caches. |
| `Wasm imports are not allowed` | Rebuild without imports (WASI, wasm-bindgen, host functions). |
| `plugin does not export 'memory'` | Export 32-bit linear memory. |
| `does not export 'amph_alloc'` / `amph_analyze` / `amph_abi_version` | Missing or misnamed export; check `#[no_mangle]` and `cdylib`. |
| `invalid signature` | Wrong ABI function signature; compare with the table. |
| `reported ABI N ... supports ABI 1` | Return `1`. |
| `plugin returned invalid JSON` | Return exactly one valid JSON value, e.g. an object with `summary`. |
| `output is too large` / `input is too large` | Exceeded 1 MiB. |
| `Wasm module is too large` | Exceeded 2 MiB; use `opt-level = "s"`, LTO, strip, fewer deps. |
| `initial memory ... exceeds` | Declared memory exceeds 16 MiB. |
| `out of fuel` | Reduce work; 20M fuel per call. |
| `plugin 'x' is a theme plugin and has no JSON analyzer` | `a` works only for functionality. |
| `plugin 'x' is disabled` | Enable it with `space` before analyze. |
| `plugin ... is a symbolic link` | Symlinked plugin directories and store files (`plugin.json`, `install.json`, the `disabled` marker, and the functionality `.wasm` entry) are rejected; use regular files. |
| `functionality plugin 'x' has no Wasm entry` | Inconsistent manifest/entry; reinstall a correct release. |
| `No plugin selected` | Select a row with `j`/`k`. |
| `This plugin does not analyze` | `a` is functionality-only; themes use `T`. |
| `This plugin is not a theme` | `T` is theme-only. |
| `Theme not applied` | Check palette and colors: only documented ANSI names or `#RRGGBB` are accepted. |
| `cannot run curl` | Install `curl`. |
| `could not read installed plugins` / `READ ONLY` | Emergency read-only fallback; check permissions/XDG. |

Troubleshooting:

- **Release exists but AMPH does not see it.** Publish it (not draft), do not
  mark prerelease, and mark it latest. `releases/latest` ignores drafts and
  prereleases.
- **Checksum file added, digest error remains.** AMPH reads the GitHub API
  `digest` field; wait for GitHub to compute it.
- **Works locally but not in AMPH.** Confirm `wasm32-unknown-unknown` (not
  WASI), no imports and no start. Run `scripts/check-wasm.sh`; AMPH performs the
  authoritative ABI check.
- **`a` is unavailable.** The selected plugin must be enabled and have
  `kind: functionality`. Themes use `T`, not `a`.
- **`T` does not change colors.** The plugin must be an enabled theme with
  accepted colors. `T` applies only to the current session and never saves
  configuration. `NO_COLOR` keeps the UI monochrome.
- **The analyzer rejects the request.** AMPH sends the `api/op/snapshot`
  envelope; do not run it with your own JSON outside AMPH. Use the current
  build and this analyzer version.
- **No update is offered.** If the tag, `plugin.json` digest, and — for
  functionality — the Wasm entry digest are unchanged, the plugin is current.
  Bump the version and publish a new tag.
- **Want theme + functionality in one repo.** Not possible in one release;
  split repositories.
- **Manager shows READ ONLY.** Not normal. AMPH could not read the store; check
  permissions and `XDG_DATA_HOME`/`HOME`, then restart.

## Compatibility

| Item | Requirement |
|---|---|
| Manifest schema | `1` |
| ABI | `1` |
| Analyze request | `api: 1`, `op: "analyze"`, `snapshot.packages` |
| Backend | AMPH 0.3.0-beta `src/plugin/` + `StorePluginHandler` |
| Target | `wasm32-unknown-unknown`, binary `.wasm` |
| Memory | 32-bit, max 16 MiB |
| Imports | none |
| Start function | none |
| WASI | none |
| Host capabilities | none |
| TUI | `Shift+P`; `space`, `a`, `T`, `r`, `c`, `U`, `R` |
| GitHub | public `https://github.com/{owner}/{repo}` |
| Release | latest stable, no draft/prerelease |
| Digests | `sha256:<64 hex>` for `plugin.json` and entry |
| Platform | Linux/Arch (AMPH); `curl` required |
| Example license | MIT |

- `schema` and `api` are versioned independently of the plugin `version`.
- There is no downgrade negotiation: an ABI 2 module will not install on an
  ABI 1 build.
- The manifest is strict; unknown future fields are rejected by older builds.
  Do not add speculative fields.
- The Wasm feature set is narrower than a browser. Do not assume SIMD,
  references, threads, atomics, tail calls or memory64.
- No pacman DB, files, network or environment access. The host provides data
  only in an explicitly confirmed analyze.
- Themes are declarative data (six colors plus a palette), not UI plugins.
- Updates are based on the release tag, `plugin.json` digest, and — for
  functionality — the Wasm entry digest, not semver. A change to any of those
  can mark an update; changing only `version` without changing them does not.

## Development, tests and CI

```sh
cargo fmt --all -- --check
cargo check --locked
cargo test --locked --lib
./scripts/build-release-assets.sh
./scripts/verify-release.sh v1.0.0 DAMIOTF/AMPH-example-plugin  # after publish
```

Unit tests cover the envelope parser, reordered/unknown fields, empty
snapshots, maximum values and rejected requests. They do not need the Wasm
target; the release build does.

CI installs stable Rust with the Wasm target and WABT, checks formatting, runs
host checks/tests, builds and validates the release assets, validates the
manifest, and uploads `dist/`. The release workflow repeats the checks, creates
the GitHub Release and verifies GitHub digests.

Rules for changes:

- an ABI change needs a new ABI version and a matching host change; do not
  alter signatures under `api: 1`;
- a manifest schema change needs a new `schema`; old backends reject unknown
  fields;
- check imports and module size before adding runtime dependencies;
- do not commit `dist/` or `target/`;
- update tests and both READMEs with the contract.

## License

The example code is **MIT** — see [LICENSE](LICENSE). AMPH itself is a separate
PolyForm Noncommercial 1.0.0 project; this repository is not part of AMPH and
does not change its license.
