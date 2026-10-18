# Review Desk

Desktop client for managing and doing code reviews across providers, starting with GitHub.

## Architecture (local data)

Review Desk is **local-first** for the Pull requests list and cached pull-request data:

- **LocalStore** (`reviewdesk-core` trait, `reviewdesk-localstore` Turso implementation) holds the on-disk cache.
- The **UI reads LocalStore** on the hot path so Pull requests and Review pane screens stay responsive.
- **Providers** (for example GitHub) **sync in the background** and upsert into LocalStore; they are not the default for every list navigation.
- Persistence uses **embedded Turso** (SQLite-compatible, in-process). **No Turso Cloud account** or remote database is required for local features.
- Schema stays **vanilla SQLite** so we can fall back to plain SQLite later if needed.

Decision records: [ADR-0001: LocalStore-first](docs/adr/0001-localstore-turso-local-first.md), [ADR-0004: Provider write-through and cache scope](docs/adr/0004-provider-writethrough-and-localstore-cache.md).

Agents: see root [AGENTS.md](AGENTS.md) and [GLOSSARY.md](GLOSSARY.md).

## Development

Rust workspace (edition 2024). From the repo root:

```bash
cargo test
cargo run -p reviewdesk
```

Provider auth: `GITHUB_TOKEN`, `GH_TOKEN`, or `gh auth token`. Scopes and classic-vs-fine-grained gaps: [docs/auth-github.md](docs/auth-github.md).
