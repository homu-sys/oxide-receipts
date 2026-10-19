# Oxide

A fast, simple system package manager for Linux with a single-file registry index.

```
oxide -S ripgrep        # install
oxide -Syu              # sync + upgrade everything
oxide -R bat            # remove
oxide -Ss fuzzy         # search
oxide -Q                # list installed
```

## How it works

- **Registry** — a single `index` file at the repo root describes every package. `oxide sync` downloads just that one file (fast, even with thousands of packages) instead of cloning the repo, and reports what changed.
- **Install** — the archive is fetched from the package's `url` (archives committed under `packages/<name>/` are served via their raw URL), extracted into `/opt/oxide/<name>`, and its binaries symlinked into `/usr/local/bin`.
- **Receipts** — every install writes a receipt (`/var/lib/oxide/receipts/<name>.toml`) listing all files (with sha256) and symlinks. This powers clean removal, dependency protection, `oxide files`, and `oxide verify`.

## Layout

```
index             # single-file registry: every package manifest, as JSON
packages/
  ripgrep/
    config.toml
  fd/
    ...
oxide/            # the tool (Rust)
```

## Building

```sh
cd oxide
cargo build --release
sudo cp target/release/oxide /usr/local/bin/
```

## Usage

| Command | Description |
|---|---|
| `oxide sync` | Download the registry `index` (fast, single file) |
| `oxide install <pkg>...` | Install packages (resolves dependencies) |
| `oxide remove <pkg>...` | Remove packages (`--force` to override dependents) |
| `oxide update [pkg...]` | Upgrade installed packages (`--sync` to pull first) |
| `oxide search <query>` | Search the registry |
| `oxide request` | List all packages available in the registry |
| `oxide info <pkg>` | Show package details |
| `oxide list` | List installed packages |
| `oxide outdated` | Show packages with updates available |
| `oxide files <pkg>` | List files owned by a package |
| `oxide verify [pkg...]` | Check installed files against receipt hashes |
| `oxide clean` | Remove cached staging data |

pacman-style aliases work too: `-S`, `-Sy`, `-Syu`, `-Su`, `-Ss`, `-Sl`, `-Si`, `-Sc`, `-R`, `-Q`, `-Qi`, `-Ql`, `-Qu`.

Registry URL defaults to this repository; override with `--registry <url-or-path>`, `OXIDE_REGISTRY`, or `/etc/oxide/config.toml`:

```toml
[registry]
url = "https://github.com/homu-sys/oxide-receipts.git"
```

`--root <dir>` (or `OXIDE_ROOT`) installs into an alternate root — handy for testing without sudo.

## Package manifest

```toml
[package]
name = "ripgrep"
version = "15.2.0"
description = "Line-oriented search tool"
homepage = "https://github.com/BurntSushi/ripgrep"
license = "MIT OR Unlicense"
arch = ["x86_64"]          # omit or empty = all architectures

[dependencies]
depends = []                # other package names

[archive]
file = "ripgrep-15.2.0-x86_64-unknown-linux-musl.tar.gz"
strip_prefix = "ripgrep-15.2.0-x86_64-unknown-linux-musl"

[install]
bins = ["rg"]               # paths relative to extracted root; omit to auto-detect executables
# rename on install with "src -> name", e.g. bins = ["yq_linux_amd64 -> yq"]
```

Supported archive formats: `.tar.gz`, `.tgz`, `.tar.xz`, `.tar.zst`, `.tar`.

## Adding a package

1. Create `packages/<name>/`.
2. Write `config.toml` (see above), setting `[archive] url` to the release archive.
   Drop the archive into the directory too if you'd rather commit it. Check the
   archive's internal layout with `tar tzf <archive> | head`.
3. Rebuild the top-level `index` from `packages/*/config.toml`, then commit and
   push. `oxide sync` picks it up.

## Full system upgrade

```sh
sudo oxide -Syu     # sync the index, then upgrade every installed package
```

Output mirrors pacman: it shows whether the index changed and lists the
`name-version` set being upgraded.
