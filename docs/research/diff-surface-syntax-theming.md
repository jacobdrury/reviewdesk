# Diff surface syntax theming in gpui-kit 0.7.3

Facts only. No product decisions. Aimed at mapping a Zed theme `syntax` block onto Review Desk's Diff surface via gpui-kit's Theme / Diff pipeline.

Research date: 2026-10-17. Sources are crates.io `gpui-component` / `gpui-kit` **0.7.3** (local registry sources under `~/.cargo/registry/src/.../gpui-component-0.7.3/`), Review Desk `crates/reviewdesk`, and the Zed theme shape already covered in [`zed-theme-format.md`](./zed-theme-format.md).

## Verdict

Diff does **not** read Zed JSON directly. At paint time it resolves Tree-sitter capture names through `cx.theme().highlight_theme` (`HighlightStyleResolver`). That `HighlightTheme` is installed only when a gpui-kit `ThemeConfig` with a non-empty `highlight` object is applied (`Theme::apply_config`). Inside that object, `syntax` is the Zed-compatible capture map.

For a Zed theme's `style.syntax` to drive Diff coloring in Review Desk, all of the following must be true:

1. Zed `style` (or at least its editor tokens + `syntax`) is mapped into gpui-kit `ThemeConfig.highlight` (not left under Zed's top-level `style` key).
2. That `ThemeConfig` is applied so `Theme.highlight_theme` updates (`Theme::apply_config` / `Theme::change` path).
3. `Diff` keeps `syntax_highlight` enabled (default `true`; Review Desk does not disable it).
4. Background presentation produced capture spans, which requires the `tree-sitter` feature (and language grammars) on `gpui-kit` / `gpui-component`. Review Desk currently depends on `gpui-kit = "0.7.3"` with defaults only, so Diff paints with an empty capture list and theme syntax colors never appear.

Add/delete chrome (gutter bars / ± signs / inline-change wash) uses `cx.theme().danger` / `cx.theme().success` (UI `ThemeColor`), not `highlight.syntax`.

## 1. Paint path: Diff → highlight_theme → syntax

`Diff` is documented as a themed readonly code-review surface (`gpui-component` `src/diff/mod.rs`). Syntax is on by default:

```rust
syntax_highlight: true, // Diff::new defaults
pub fn syntax_highlight(mut self, value: bool) -> Self { ... }
```

Per-line highlights are built in `line_highlights`:

1. If presentation exists and `syntax_highlight` is true, call  
   `presentation.syntax_highlights(..., cx.theme().highlight_theme.as_ref())`.
2. Optionally layer inline-change backgrounds (`status.opacity(0.3)`).
3. Optionally layer selection (`cx.theme().selection`).

`FilePresentation::syntax_highlights` (`src/diff/presentation.rs`) is theme-agnostic storage plus a late color lookup:

- Background prep stores **capture name indices**, not colors (`//! Presentation prepared on a background thread: theme-independent syntax`).
- At paint: `theme.style(&self.names[*name as usize])?` via `HighlightStyleResolver`.

`HighlightTheme` implements that trait (`src/highlighter/registry.rs`):

```rust
impl gpui_base::input::HighlightStyleResolver for HighlightTheme {
    fn style(&self, name: &str) -> Option<HighlightStyle> {
        self.style.syntax.style(name)
    }
}
```

So the only theme field Diff syntax coloring consults is `Theme.highlight_theme.style.syntax` (plus selection / danger / success for non-syntax overlays).

## 2. Where `highlight_theme` comes from

### Active Theme

`Theme` (`src/theme/mod.rs`) holds:

- `colors: ThemeColor` (UI tokens: `background`, `danger`, `success`, …)
- `highlight_theme: Arc<HighlightTheme>` (editor + syntax)

`gpui_kit::init` → `gpui_component::init` → `Theme::change(ThemeMode::Light, …)`, which loads the registered light `ThemeConfig` through `apply_config`.

### ThemeConfig JSON shape (gpui-kit, not raw Zed)

`ThemeConfig` (`src/theme/schema.rs`):

| Field | Role |
| --- | --- |
| `name`, `mode` (`light` \| `dark`) | Identity / appearance |
| `colors` | UI `ThemeColor` map (shadcn-style keys) |
| `highlight` | `Option<HighlightThemeStyle>`; comment in source: compatibility with the Zed theme **`style`** section |

`Theme::apply_config`:

```rust
if let Some(style) = &config.highlight {
    self.highlight_theme = Arc::new(HighlightTheme {
        name: config.name.to_string(),
        appearance: config.mode,
        style: style.clone(),
    });
}
```

If `highlight` is `None`, the previous `highlight_theme` is **left unchanged**.

`ThemeRegistry` loads `ThemeSet` JSON (`load_themes_from_str`, `watch_dir`, bundled `default-theme.json`). Default themes include a full `highlight` block with nested `syntax`.

### HighlightThemeStyle

(`src/highlighter/registry.rs`; documented as Zed highlight / theme compatible)

JSON keys under `highlight`:

- Editor: `editor.background`, `editor.foreground`, `editor.active_line.background`, `editor.line_number`, `editor.active_line_number`, `editor.invisible`, `editor.gutter.background`
- Flattened status colors: `error`, `warning`, `info`, `success`, `hint`, …
- `syntax`: map of capture → `{ color?, font_style?, font_weight? }`

`ThemeStyle` (per capture) has **only** `color`, `font_style`, `font_weight`. Zed's optional `background_color` on syntax entries is not modeled; unknown JSON fields are ignored by serde defaults.

`SyntaxColors::style(name)` matches a fixed set of capture names (`HIGHLIGHT_NAMES`, 41 entries: `keyword`, `string`, `comment.doc`, `punctuation.bracket`, …). Unknown dotted names fall back to the prefix before the first `.` (e.g. `function.method` → `function`, `string.regexp` → `string`). Empty name → no style.

Hex colors in highlight JSON deserialize via gpui `Hsla` ← `Rgba` string forms (`#RGB` / `#RRGGBB` / …) (`gpui-pre` `color.rs`).

## 3. Zed theme vs gpui-kit ThemeConfig

From [`zed-theme-format.md`](./zed-theme-format.md) and a local ultraViolet family file:

| Zed family / theme | gpui-kit `ThemeSet` / `ThemeConfig` |
| --- | --- |
| `name`, `author`, `themes[]` | Same idea (`ThemeSet`) |
| Per theme: `name`, `appearance` (`light`\|`dark`) | `name`, **`mode`** (`light`\|`dark`). No `appearance` alias on `ThemeConfig`. |
| `style` (flat UI colors + `syntax` + status + …) | Split: UI → `colors` (different key vocabulary); highlight/syntax → **`highlight`** |
| `style.syntax` | `highlight.syntax` |
| `style.editor.*` | `highlight.editor.*` |

**Raw Zed theme JSON is not a valid `ThemeSet`.** Deserializing ultraViolet (or One) as `ThemeSet` fails the structural mapping (`appearance` vs `mode`, `style` vs `colors`+`highlight`). An adapter must build a `ThemeConfig` (or mutate `Theme.highlight_theme` directly) and call `apply_config` / `Theme::update`.

Minimal adapter for Diff syntax only:

1. Read Zed `themes[i].style.syntax` (and optionally `editor.*` for editor chrome tokens Diff may use via `Theme::editor_background`).
2. Place them under `ThemeConfig { mode: from appearance, highlight: Some(HighlightThemeStyle { syntax, editor_*, .. }), colors: … }`.
3. `Theme::apply_config` (or register in `ThemeRegistry` then `Theme::change`).

UI chrome still needs a separate Zed `style` → `ThemeConfig.colors` map (out of scope for this ticket except noting Diff gutters use `danger` / `success` from that UI half).

### Capture name overlap (example: ultraViolet)

Zed free-form keys; kit resolves known names + prefix fallback. ultraViolet keys **not** in kit's exact list include `function.method`, `function.builtin`, `string.regexp`, `variable.builtin`, `type.builtin`, `parameter`, `field`, `namespace`, `conditional`, `bracket`, `strong`. Many still color via prefix fallback (`function.method` → `function`). Exact-only kit keys missing from that file leave those captures unstyled unless another theme supplies them.

Quirk: `SyntaxColors.comment_doc` has **no** `#[serde(rename = "comment.doc")]`, while other dotted fields do and `default-theme.json` uses `"comment.doc"`. Exact `comment.doc` JSON may not populate `comment_doc` unless fixed upstream or renamed on import. Runtime lookup still asks for `"comment.doc"`.

## 4. Review Desk today

| Fact | Source |
| --- | --- |
| Depends on `gpui-kit = "0.7.3"` (default features: `component`, `assets`) | `crates/reviewdesk/Cargo.toml` |
| Boot: `gpui_kit::init(cx)` only; no `ThemeRegistry::watch_dir`, no custom `ThemeConfig`, no Zed load | `crates/reviewdesk/src/main.rs` |
| UI styling via `cx.theme()` tokens (`border`, `muted_foreground`, `danger`, `secondary`, …) | `crates/reviewdesk/src/app.rs` |
| Diff: `DiffState::new([], …).with_mode(DiffMode::Split)`; `Diff::new(&self.diff).size_full().annotations(…)` | same; does **not** call `.syntax_highlight(false)` |
| Files from `DiffFile::parse(patch)` | `parse_patch` in `app.rs` |
| Language: path detection inside kit (`detected_language`); no `with_language` override from Review Desk | kit `diff/document.rs`, `presentation.rs` |

`Cargo.lock` shows `gpui-component` 0.7.3 **without** `tree-sitter` crates. With `tree-sitter` off, `highlighter` is the WASM stub: `SyntaxHighlighter::capture_names` returns `[]`, so presentation stores no spans and Diff never applies `highlight_theme` colors to text. Enabling `gpui-kit` features such as `tree-sitter` / `tree-sitter-languages` (or per-language features) is a prerequisite for visible syntax coloring.

## 5. What must be true (checklist)

For Zed `syntax` to drive Diff surface token colors:

1. **Map** Zed `style.syntax` → gpui-kit `ThemeConfig.highlight.syntax` (and apply that config so `Theme.highlight_theme` updates).
2. **Do not** expect `ThemeRegistry` / `serde` to ingest raw Zed theme files unchanged (`appearance`/`style` vs `mode`/`colors`+`highlight`).
3. Keep **`Diff` syntax highlighting on** (default; Review Desk already does).
4. Enable **`tree-sitter`** (and grammars for languages you care about) on the `gpui-kit` dependency so presentation emits capture spans.
5. Prefer capture names in kit's `HIGHLIGHT_NAMES` (or dotted forms that prefix-fallback to them). Accept that Zed-only keys without a useful prefix stay uncolored.
6. Treat Diff **add/delete** chrome as UI tokens (`danger` / `success`), mapped from Zed UI colors separately if those should match Zed too.
7. After theme swap, Diff does not need a presentation rebuild for syntax **colors** (colors resolve at paint). Capture spans only rebuild when files / presentation options change.

## Primary links

- Registry sources (local): `gpui-component-0.7.3` `src/diff/mod.rs`, `src/diff/presentation.rs`, `src/theme/{mod,schema,registry}.rs`, `src/theme/default-theme.json`, `src/highlighter/{registry,mod,wasm_stub}.rs`
- crates.io: [gpui-kit 0.7.3](https://crates.io/crates/gpui-kit/0.7.3), [gpui-component 0.7.3](https://crates.io/crates/gpui-component/0.7.3)
- Upstream: [longbridge/gpui-kit](https://github.com/longbridge/gpui-kit)
- Docs: [docs.rs/gpui-component](https://docs.rs/gpui-component/0.7.3), kit notes on Zed-style highlight ([Zed syntax highlighting docs](https://zed.dev/docs/extensions/languages#syntax-highlighting) linked from `HighlightTheme`)
- Review Desk: `crates/reviewdesk/src/{main,app}.rs`
- Related research: [`zed-theme-format.md`](./zed-theme-format.md)
)
