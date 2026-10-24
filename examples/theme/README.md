# Theme manifest example

`plugin.json` in this directory is a **reference-only** theme manifest. It is
not packaged by this repository's release workflow.

AMPH resolves exactly one `plugin.json` from the latest stable GitHub release of
a repository. A repository therefore publishes **one plugin at a time**. Do not
upload this file next to the functionality manifest at the repository root.

To publish a theme, create a separate repository (or replace the root manifest
in a dedicated theme repository) whose only release asset is:

- `plugin.json` — a theme manifest with a repository-specific `id` and a
  GitHub-provided digest.

A theme must not declare `api` or `entry`. Color fields accept only the
documented base ANSI names or an exact `#RRGGBB` value; unknown names, `#RGB`
and `rgb(...)` are rejected. The backend validates colors on parse and again on
apply.

The backend stores the manifest under `$XDG_DATA_HOME/amph/plugins/<id>`. In
AMPH, select the enabled theme in the **Installed** plugin manager section and
press `T` to apply it to the **current session only**. The choice is not saved,
so restarting AMPH restores the previous theme. `NO_COLOR` keeps the interface
monochrome. A theme cannot define custom screens, widgets, commands or any other
host capability.

The root `scripts/verify-release.sh` is specific to the functionality plugin
and expects both `plugin.json` and `plugin.wasm`; do not reuse it unchanged for
a theme-only repository.
