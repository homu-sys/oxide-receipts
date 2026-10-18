# Oxide

A fast, simple system package manager for Linux with a git-backed package registry.

```
oxide -S ripgrep        # install
oxide -Syu              # sync + upgrade everything
oxide -R bat            # remove
oxide -Ss fuzzy         # search
oxide -Q                # list installed
```

## How it works

- **Registry** — this repository. `packages/<name>/` holds a `config.toml` manifest and the package archive itself. Syncing is a shallow `git pull`.
- **Install** — archives are extracted locally from the synced registry into `/opt/oxide/<name>`, with binaries symlinked into `/usr/local/bin`. No per-package downloads.
- **Receipts** — every install writes a receipt (`/var/lib/oxide/receipts/<name>.toml`) listing all files (with sha256) and symlinks. This powers clean removal, dependency protection, `oxide files`, and `oxide verify`.

## Layout

```
packages/
  ripgrep/
    config.toml
    ripgrep-15.2.0-x86_64-unknown-linux-musl.tar.gz
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
| `oxide sync` | Pull the latest registry |
| `oxide install <pkg>...` | Install packages (resolves dependencies) |
| `oxide remove <pkg>...` | Remove packages (`--force` to override dependents) |
| `oxide update [pkg...]` | Upgrade installed packages (`--sync` to pull first) |
| `oxide search <query>` | Search the registry |
| `oxide info <pkg>` | Show package details |
| `oxide list` | List installed packages |
| `oxide outdated` | Show packages with updates available |
| `oxide files <pkg>` | List files owned by a package |
| `oxide verify [pkg...]` | Check installed files against receipt hashes |
| `oxide clean` | Remove cached staging data |

pacman-style aliases work too: `-S`, `-Sy`, `-Syu`, `-Su`, `-Ss`, `-Si`, `-Sc`, `-R`, `-Q`, `-Qi`, `-Ql`, `-Qu`.

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
```

Supported archive formats: `.tar.gz`, `.tgz`, `.tar.xz`, `.tar.zst`, `.tar`.

## Adding a package

1. Create `packages/<name>/`.
2. Drop the release archive in.
3. Write `config.toml` (see above). Check the archive's internal layout with `tar tzf <archive> | head`.
4. Commit and push. `oxide sync` picks it up.
