# Moon and proto for a Rust Cargo workspace

Research for [issue #2](https://github.com/jacobdrury/reviewdesk/issues/2). Plan only: do not scaffold app crates on `main`. Docs cited below are moon v2 / latest proto unless noted.

## Recommendation

| Concern | Where it lives | Rule |
| --- | --- | --- |
| Rust version + install | Root `.prototools` (`rust = "..."`), moon toolchain enabled | Moon owns Rust for tasks. Version is pinned in proto config so moon inherits it (`versionFromPrototools` defaults to `true`). |
| Moon binary (and every non-Rust tool) | Root `.prototools` only | Pin and install with proto. Do not enable `node`, `go`, etc. in `.moon/toolchains.*`. |
| Cargo workspace crates | `crates/reviewdesk`, `crates/reviewdesk-core` | Prefer one root `moon.yml` next to `Cargo.lock` with `--workspace` cargo tasks. Optionally register the two crates as moon projects if you need package-level IDs. |

**Gist:** Pin `rust` and `moon` in `.prototools`. Enable only `rust` in `.moon/toolchains.yml` (empty object is enough once the version is in `.prototools`). Init `.moon/workspace.yml`, put Cargo-facing tasks in a root `moon.yml` (handbook workspace pattern). Leave other languages out of moon's toolchain.

## Ownership model

### Moon owns Rust through proto

Moon's toolchain is built on proto: it reuses `~/.proto`, installs missing tools there, and prepends the right binaries to `PATH` for tasks.

Sources:

- [Toolchain](https://moonrepo.dev/docs/concepts/toolchain): moon piggybacks on proto at `~/.proto`; task runs activate toolchain executables on `PATH`.
- [Rust handbook](https://moonrepo.dev/docs/guides/rust/handbook): enabling Rust is either `rust: {}` (or richer settings) in `.moon/toolchains.*`, **or** pinning `rust` in the workspace-root `.prototools`. A `version` (in toolchains or via `.prototools`) is what makes moon download/install a pinned toolchain instead of trusting whatever is on `PATH`. Moon still requires `rustup` and installs channels through it.
- [.moon/toolchains `versionFromPrototools`](https://moonrepo.dev/docs/config/toolchain): when `true` (default), toolchain `version` is inherited from root `.prototools` under the same tool id (`rust`). Keeping versions in `.prototools` is the documented single source of truth (and Renovate-friendly).
- [proto Rust plugin README](https://github.com/moonrepo/plugins/blob/master/tools/rust/README.md): proto does not reinvent rustup; it calls rustup, references `~/.rustup/toolchains`, does not shim `cargo`/`rustup`, and suggests `rust-toolchain.toml` for runtime channel selection. Moon's `syncToolchainConfig` can keep that file in sync when a version is set ([Rust toolchain config](https://github.com/moonrepo/plugins/blob/master/toolchains/rust/src/config.rs)).

### Every other tool is managed directly by proto

Moon can install and pin moon itself via proto. Official install guidance prefers proto for moon and other tools so versions stay multi-install and project-pinned.

Sources:

- [Install moon](https://moonrepo.dev/docs/install): `proto install moon`; pin with `moon = "..."` in `.prototools`; "We suggest using proto to manage moon (and other tools)".
- [proto configuration](https://moonrepo.dev/docs/proto/config): project-local `.prototools` pins tool versions for that directory tree; `proto install` / pin commands update it.
- [.moon/toolchains](https://moonrepo.dev/docs/config/toolchain): languages are opt-in plugins. Enabling only `rust` means moon will not manage Node, Go, Python, etc. Those stay proto-only if you pin them in `.prototools`.

Do **not** put non-Rust language blocks in `.moon/toolchains.yml` for Review Desk. That is how "other tools managed directly by proto" stays true.

First-party example of the split (moon + other CLIs in `.prototools`, Rust version in moon toolchains): [moonrepo/plugins `.prototools`](https://github.com/moonrepo/plugins/blob/master/.prototools) and [`.moon/toolchains.yml`](https://github.com/moonrepo/plugins/blob/master/.moon/toolchains.yml). Review Desk should put the **Rust version in `.prototools`** instead, so proto is the pin file for every tool including Rust, while moon still owns Rust *as a moon toolchain* for hashing, PATH, and task orchestration.

## Minimal file layout

Illustrative pins only. Choose concrete versions when scaffolding (full semver preferred over aliases).

```text
/
├── .prototools
├── .moon/
│   ├── workspace.yml          # required after moon init
│   └── toolchains.yml         # rust only
├── crates/
│   ├── reviewdesk/            # app binary (later)
│   └── reviewdesk-core/       # library (later)
├── Cargo.toml                 # cargo workspace (later)
├── Cargo.lock
└── moon.yml                   # root Rust project + cargo --workspace tasks
```

### `.prototools` (root)

```toml
# Task runner: managed by proto, not by moon's language toolchain
moon = "2.6.0"

# Rust channel/version: proto pin; moon inherits via versionFromPrototools
rust = "1.93.0"

# Optional: lock the proto CLI used by shims / local installs
# proto = "0.62.2"
```

Sources: [Install moon](https://moonrepo.dev/docs/install); [Rust handbook](https://moonrepo.dev/docs/guides/rust/handbook) (`.prototools` rust pin); [proto config](https://moonrepo.dev/docs/proto/config) (pinning, optional `proto = "..."`).

`proto install` (no args) installs tools from parent `.prototools` files ([proto install](https://moonrepo.dev/docs/proto/commands/install)).

### `.moon/toolchains.yml`

```yaml
# Enable Rust for moon. Version comes from root .prototools (default).
rust:
  syncToolchainConfig: true
  # Optional once you care about CI parity:
  # components: ['clippy', 'rustfmt']
```

Even `rust: {}` enables the language plugin; with `rust` pinned in `.prototools`, moon treats that as the toolchain version ([Rust handbook](https://moonrepo.dev/docs/guides/rust/handbook); [`versionFromPrototools`](https://moonrepo.dev/docs/config/toolchain)).

Optional: pin the proto binary moon uses for toolchain ops:

```yaml
proto:
  version: '0.62.2'
```

([`.moon/toolchains` proto.version](https://moonrepo.dev/docs/config/toolchain))

Do not add `node`, `javascript`, `go`, etc. here.

### `.moon/workspace.yml`

Scaffold with `moon init` (or `moon init --minimal`) at the repo root ([Setup workspace](https://moonrepo.dev/docs/setup-workspace); [init](https://moonrepo.dev/docs/commands/init)). Then set projects.

**Minimal Cargo-workspace pattern (preferred for Review Desk):** one moon project at the repo root that drives `cargo … --workspace`. Root needs a `moon.yml` so the root is a project ([workspace projects](https://moonrepo.dev/docs/config/workspace): only a matched moon config creates a root-level project).

```yaml
projects:
  reviewdesk-workspace: '.'

vcs:
  defaultBranch: 'main'
```

**If package-level moon IDs are required** (`moon run reviewdesk:…`), map crates explicitly or glob manifests (moon v2.5+ file globs):

```yaml
projects:
  # Folder name becomes the project id (reviewdesk, reviewdesk-core)
  - 'crates/*/Cargo.toml'
```

or

```yaml
projects:
  reviewdesk: 'crates/reviewdesk'
  reviewdesk-core: 'crates/reviewdesk-core'
```

Sources: [workspace `projects`](https://moonrepo.dev/docs/config/workspace) (map, globs, `crates/*/Cargo.toml` example); [Create a project](https://moonrepo.dev/docs/create-project).

Handbook warning: per-crate moon projects that each spawn cargo can hit Cargo's build lock under concurrency. For a small Cargo workspace, root `--workspace` tasks avoid that ([Rust handbook](https://moonrepo.dev/docs/guides/rust/handbook)).

### Root `moon.yml` (handbook workspace tasks)

Matches the handbook "Workspaces" example: moon orchestrates cargo; it does not replace Cargo.

```yaml
language: 'rust'
layer: 'application'

env:
  CARGO_TERM_COLOR: 'always'

fileGroups:
  sources:
    - 'crates/*/src/**/*'
    - 'crates/*/Cargo.toml'
    - 'Cargo.toml'
  tests:
    - 'crates/*/benches/**/*'
    - 'crates/*/tests/**/*'

tasks:
  build:
    command: 'cargo build'
    inputs:
      - '@globs(sources)'
  check:
    command: 'cargo check --workspace'
    inputs:
      - '@globs(sources)'
  format:
    command: 'cargo fmt --all --check'
    inputs:
      - '@globs(sources)'
      - '@globs(tests)'
  lint:
    command: 'cargo clippy --workspace'
    inputs:
      - '@globs(sources)'
      - '@globs(tests)'
  test:
    command: 'cargo test --workspace'
    inputs:
      - '@globs(sources)'
      - '@globs(tests)'
```

Source: [Rust handbook](https://moonrepo.dev/docs/guides/rust/handbook) (Example `moon.*` / Workspaces).

Do not cache the whole `target/` directory as a moon output; cache a release binary path at most ([Rust handbook FAQ](https://moonrepo.dev/docs/guides/rust/handbook)).

### Per-crate `moon.yml` (only if you map crate projects)

```yaml
# crates/reviewdesk/moon.yml
language: 'rust'
layer: 'application'

# crates/reviewdesk-core/moon.yml
language: 'rust'
layer: 'library'
```

`language` / `layer` drive inheritance and layering rules ([moon project config](https://moonrepo.dev/docs/config/project)). Shared cargo tasks can live in `.moon/tasks/rust.yml` with `inheritedBy.language: rust` instead of duplicating task bodies ([Task inheritance](https://moonrepo.dev/docs/concepts/task-inheritance); [`.moon/tasks`](https://moonrepo.dev/docs/config/tasks)).

For per-crate tasks, prefer package-scoped cargo (`cargo test -p reviewdesk-core`) or a single root workspace target so parallel moon tasks do not fight over `target/`.

### Cargo workspace (later scaffold; not on this branch)

When crates exist:

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = [
  "crates/reviewdesk",
  "crates/reviewdesk-core",
]
```

Moon is not the build system; Cargo remains authoritative ([Rust handbook](https://moonrepo.dev/docs/guides/rust/handbook)).

## Bootstrap sequence (when implementing)

1. Install proto, then `proto install moon` (and pin in `.prototools`) ([Install moon](https://moonrepo.dev/docs/install)).
2. `moon init` at repo root ([Setup workspace](https://moonrepo.dev/docs/setup-workspace)).
3. Add `.prototools` pins (`moon`, `rust`) and `.moon/toolchains.yml` with `rust` only.
4. Add root `moon.yml` tasks; wire `.moon/workspace.yml` `projects`.
5. `proto install` / first `moon run …` so proto + rustup materialize the Rust toolchain.
6. Add Cargo workspace + crates in a later ticket (out of scope for this research branch).

## Alternatives considered

| Alternative | Why not for Review Desk |
| --- | --- |
| Pin Rust only in `.moon/toolchains.yml` (`rust.version`) and omit from `.prototools` | Valid per handbook, and what [moonrepo/plugins](https://github.com/moonrepo/plugins/blob/master/.moon/toolchains.yml) does. Weaker match for "every tool version in proto" / map wording. Prefer `.prototools` + inheritance. |
| Enable empty `rust: {}` with no version anywhere | Enables Cargo integration but uses ambient `PATH` Rust, not a pinned proto-managed toolchain ([Rust handbook](https://moonrepo.dev/docs/guides/rust/handbook)). |
| Per-crate moon projects as the only layout | Fine for package IDs; handbook prefers root workspace tasks for Cargo lock behavior. Use crate projects as an add-on, not the default for workspace cargo. |
| Manage moon via curl installer / npm only | Works, but docs recommend proto for multi-version + `.prototools` pins ([Install moon](https://moonrepo.dev/docs/install)). |
| Put Node/JS (or other) toolchains in moon "just in case" | Violates map constraint that non-Rust tools are proto-direct. |

## Sources (primary)

- https://moonrepo.dev/docs/guides/rust/handbook
- https://moonrepo.dev/docs/concepts/toolchain
- https://moonrepo.dev/docs/config/toolchain
- https://moonrepo.dev/docs/config/workspace
- https://moonrepo.dev/docs/config/project
- https://moonrepo.dev/docs/config/tasks
- https://moonrepo.dev/docs/concepts/task-inheritance
- https://moonrepo.dev/docs/setup-workspace
- https://moonrepo.dev/docs/commands/init
- https://moonrepo.dev/docs/create-project
- https://moonrepo.dev/docs/install
- https://moonrepo.dev/docs/proto
- https://moonrepo.dev/docs/proto/config
- https://moonrepo.dev/docs/proto/commands/install
- https://github.com/moonrepo/plugins/blob/master/tools/rust/README.md
- https://github.com/moonrepo/plugins/blob/master/toolchains/rust/src/config.rs
- https://github.com/moonrepo/plugins/blob/master/.prototools
- https://github.com/moonrepo/plugins/blob/master/.moon/toolchains.yml
