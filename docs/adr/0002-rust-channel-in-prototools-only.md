# Rust channel pinned only in `.prototools`

The Rust channel is the `rust = "..."` pin in root `.prototools`. Do not commit `rust-toolchain.toml`: it is gitignored so moon’s `syncToolchainConfig` can rewrite the channel without dirtying the tree, and so Renovate has only one Rust pin to bump.

Moon’s sync updates an existing toolchain file but cannot create one from nothing. CI therefore seeds `rust-toolchain.toml` from the `.prototools` pin before `setup-toolchain`; local clones need the same seed once (or any stub channel) before the first moon run. Re-adding a committed `rust-toolchain.toml` would invite a second Renovate PR and undo this decision.
