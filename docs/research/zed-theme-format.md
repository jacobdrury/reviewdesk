# Zed theme file format (primary sources)

Facts only. No product decisions. Aimed at loading a Zed `.json` theme family and mapping tokens into a gpui-based app.

Research date: 2026-10-17. Sources are zed.dev docs and `github.com/zed-industries/zed` `main` (plus local install of ultraViolet for path verification).

## 1. Where themes live on disk (macOS)

Resolved in `crates/paths/src/paths.rs`:

| Kind | Path | Source |
| --- | --- | --- |
| User / local themes | `~/.config/zed/themes/` (`config_dir().join("themes")`) | [Themes docs § Local themes](https://zed.dev/docs/themes); [paths.rs `themes_dir`](https://github.com/zed-industries/zed/blob/main/crates/paths/src/paths.rs) (“themes that are not provided by extensions”) |
| Settings that select the theme | `~/.config/zed/settings.json` | same `config_dir()` |
| Extension themes (installed) | `~/Library/Application Support/Zed/extensions/installed/<extension-id>/` with theme JSON under that extension’s `themes/` (paths declared in `extension.toml`) | [Installing extensions](https://zed.dev/docs/extensions/installing-extensions); [paths.rs `extensions_dir` → `data_dir()/extensions`](https://github.com/zed-industries/zed/blob/main/crates/paths/src/paths.rs) |
| Built-in themes | Shipped in the Zed binary assets tree, e.g. `assets/themes/one/one.json` | [Themes docs](https://zed.dev/docs/themes) (points at that file); [one.json](https://github.com/zed-industries/zed/blob/main/assets/themes/one/one.json) |

macOS split (from `paths.rs`):

- `config_dir()` → `~/.config/zed` (user themes, `settings.json`)
- `data_dir()` → `~/Library/Application Support/Zed` (extensions, DBs, etc.)

Docs also document Windows user themes at `%USERPROFILE%\AppData\Roaming\Zed\themes\` and Linux/macOS user themes under `~/.config/zed/themes`.

## 2. Theme file format

### Schema URL

Documented schema: `https://zed.dev/schema/themes/v0.2.0.json`  
([extension themes docs](https://zed.dev/docs/extensions/themes), [user-themes blog](https://zed.dev/blog/user-themes-now-in-preview))

Caveat: [issue #58837](https://github.com/zed-industries/zed/issues/58837) reports the hosted v0.2.0 schema as stale vs current Rust types. Prefer the Rust structs as the source of truth:

- Family wrapper: [`ThemeFamilyContent` / `ThemeContent`](https://github.com/zed-industries/zed/blob/main/crates/theme_settings/src/schema.rs)
- Style / colors / syntax: [`crates/settings_content/src/theme.rs`](https://github.com/zed-industries/zed/blob/main/crates/settings_content/src/theme.rs)
- Color parse helper: [`try_parse_color`](https://github.com/zed-industries/zed/blob/main/crates/theme/src/schema.rs) (`gpui::Rgba::try_from` → `gpui::Hsla`)

### Top-level: theme family

```json
{
  "$schema": "https://zed.dev/schema/themes/v0.2.0.json",
  "name": "FamilyName",
  "author": "Author",
  "themes": [ /* ThemeContent[] */ ]
}
```

Required family fields: `name`, `author`, `themes` ([hosted schema](https://zed.dev/schema/themes/v0.2.0.json), `ThemeFamilyContent`).

### Per theme (`ThemeContent`)

Required: `name`, `appearance` (`"light"` | `"dark"`), `style`.

### `style` (`ThemeStyleContent`)

Flattened object. Logical groups in Rust ([settings_content `ThemeStyleContent`](https://github.com/zed-industries/zed/blob/main/crates/settings_content/src/theme.rs)):

| JSON / field | Role |
| --- | --- |
| `background.appearance` | Window chrome: `opaque` \| `transparent` \| `blurred` \| `mica_backdrop` \| `mica_alt_backdrop` (`WindowBackgroundContent`) |
| `accents` | Array of color strings (or nulls) for cycling accents |
| Flattened `ThemeColorsContent` | UI / editor / terminal color tokens (dot keys, see below) |
| Flattened `StatusColorsContent` | Status tokens (`error`, `warning`, `conflict`, … and `.background` / `.border` variants) |
| `players` | Array of `{ cursor, background, selection }` color objects |
| `syntax` | Map of capture name → highlight style |

Colors are optional strings matching `#RGB` / `#RGBA` / `#RRGGBB` / `#RRGGBBAA` (`ThemeColor` pattern in settings_content).

### Style key naming

JSON uses **dot-separated** keys. Rust fields are snake_case with explicit `#[serde(rename = "...")]`. Examples:

| JSON key | Runtime `ThemeColors` field |
| --- | --- |
| `element.background` | `element_background` |
| `elevated_surface.background` | `elevated_surface_background` |
| `editor.background` | `editor_background` |
| `text.muted` | `text_muted` |
| `terminal.ansi.red` | `terminal_ansi_red` |

Full rename list lives on `ThemeColorsContent` / `StatusColorsContent` in [`settings_content/src/theme.rs`](https://github.com/zed-industries/zed/blob/main/crates/settings_content/src/theme.rs). Concrete filled example: [`assets/themes/one/one.json`](https://github.com/zed-industries/zed/blob/main/assets/themes/one/one.json).

### Syntax highlight structure

```json
"syntax": {
  "keyword": { "color": "#6F78FF" },
  "comment": { "color": "#A09AAB", "font_style": "italic" }
}
```

`HighlightStyleContent` fields ([settings_content](https://github.com/zed-industries/zed/blob/main/crates/settings_content/src/theme.rs)):

- `color`, `background_color` (optional hex strings)
- `font_style`: `normal` \| `italic` \| `oblique`
- `font_weight`: numeric (100–900 in schema; content type clamps when converted)

Keys are free-form capture names (e.g. `comment`, `comment.doc`, `function.method`). Docs point capture lists at language-extension syntax highlighting docs ([Themes](https://zed.dev/docs/themes)).

Conversion to runtime: `syntax_overrides` in [`theme_settings/src/schema.rs`](https://github.com/zed-industries/zed/blob/main/crates/theme_settings/src/schema.rs) builds `HighlightStyle` with `gpui` font types via `into_gpui()`.

## 3. Theme switching (settings keys)

Documented in [Themes](https://zed.dev/docs/themes) and [All settings](https://zed.dev/docs/configuring-zed):

```json
{
  "theme": {
    "mode": "system",
    "light": "One Light",
    "dark": "One Dark"
  }
}
```

Also valid: static string form `"theme": "Any Theme Name"`.

| Key | Meaning |
| --- | --- |
| `theme` | `ThemeSelection`: static name **or** `{ mode, light, dark }` |
| `theme.mode` | `system` \| `light` \| `dark` (`ThemeAppearanceMode`) |
| `theme.light` / `theme.dark` | Theme **names** (the per-theme `name` field, not the family name) |
| `theme_overrides` | Map of theme name → partial `ThemeStyleContent` |
| `experimental.theme_overrides` | Single override blob applied to the active theme |

Rust: [`ThemeSelection` / `ThemeAppearanceMode`](https://github.com/zed-industries/zed/blob/main/crates/settings_content/src/theme.rs); applied via theme settings ([`theme_settings`](https://github.com/zed-industries/zed/blob/main/crates/theme_settings/src/settings.rs)).

UI: `theme selector: toggle` (cmd-k cmd-t). Mode toggle: cmd-k cmd-shift-t ([Themes docs](https://zed.dev/docs/themes)).

Defaults in code: light `"One Light"`, dark `"One Dark"` (`DEFAULT_LIGHT_THEME` / `DEFAULT_DARK_THEME` in settings_content).

## 4. What “Ultraviolet” / ultraViolet is

It is a **community theme extension**, not a built-in and not a user-local theme by default.

| Fact | Value |
| --- | --- |
| Extension id | `ultraviolet-theme` |
| Store page | https://zed.dev/extensions/ultraviolet-theme |
| Upstream repo | https://github.com/Gurvirr/zed-ultraViolet |
| Extension `themes` entry | `themes/ultraViolet.json` (from `extension.toml`) |
| Theme names in that file | `ultraViolet`, `ultraViolet Blur`, `ultraViolet Blur (Borderless)` (all `appearance: "dark"`) |
| Typical macOS install path | `~/Library/Application Support/Zed/extensions/installed/ultraviolet-theme/themes/ultraViolet.json` |

Verified on a machine with the extension installed: that path exists; `extension.toml` has `id = "ultraviolet-theme"`, `version = "0.2.0"`, `themes = ["themes/ultraViolet.json"]`.

If someone copied the JSON into user themes instead, it would live under `~/.config/zed/themes/`. That is optional, not how the extension installs.

Built-in families observed under `assets/themes/` include at least `one`, `ayu`, `gruvbox` (not ultraViolet).

## 5. Mapping to gpui / Zed `Theme`

There is **no separate gpui-native “Theme” type** that mirrors the JSON. The semantic UI theme is Zed’s **`theme` crate**:

- [`Theme`](https://github.com/zed-industries/zed/blob/main/crates/theme/src/theme.rs): `id`, `name`, `appearance`, `styles: ThemeStyles`
- [`ThemeStyles`](https://github.com/zed-industries/zed/blob/main/crates/theme/src/styles/colors.rs) (defined next to `ThemeColors`): `window_background_appearance`, `system`, `accents`, `colors`, `status`, `player`, `syntax`
- Access: `ActiveTheme` on `gpui::App` → `cx.theme().colors()` etc. (same file)

What **is** gpui:

| Piece | gpui type |
| --- | --- |
| All theme colors | `gpui::Hsla` (from hex via `try_parse_color`) |
| Window background mode | `gpui::WindowBackgroundAppearance` on `ThemeStyles` |
| Syntax font style / weight | `gpui::FontStyle`, `gpui::FontWeight` (`IntoGpui` in [`settings/src/content_into_gpui.rs`](https://github.com/zed-industries/zed/blob/main/crates/settings/src/content_into_gpui.rs)) |

Documented load → apply pipeline (for reuse in another app):

1. Deserialize JSON as `ThemeFamilyContent`.
2. For each `ThemeContent`, take `style: ThemeStyleContent`.
3. Build refinements:
   - `theme_colors_refinement(&style.colors, …)` → `ThemeColorsRefinement`
   - `status_colors_refinement(&style.status)` → `StatusColorsRefinement`
   - `syntax_overrides(&style)` → `Vec<(String, HighlightStyle)>`
4. Apply refinements onto base `ThemeColors` / `StatusColors` (`Refineable`), set `accents` / `players` / `window_background_appearance`, wrap syntax in `SyntaxTheme`.
5. Expose via something equivalent to Zed’s `ActiveTheme` / global theme if UI code expects `cx.theme()`.

JSON key → Rust field mapping is the serde rename table on `ThemeColorsContent` (dot → snake_case). That is the authoritative token map for applying a Zed theme JSON in another gpui app that reuses Zed’s `theme` types or mirrors `ThemeColors` field names.

## Practical checklist (load + apply)

1. Read a theme family `.json` (`ThemeFamilyContent`).
2. Pick a `themes[]` entry by `name` (what settings store) and/or `appearance`.
3. Parse hex colors to `Hsla`; ignore unknown keys if schema drifts.
4. Map flat style keys into a `ThemeColors`-like struct (use serde renames or an explicit table from `ThemeColorsContent`).
5. Keep `syntax` as a string-keyed map of optional color / font attrs.
6. Honor `background.appearance` if the host window supports transparency/blur.
7. Do not expect gpui alone to interpret Zed theme JSON; the interpretation layer is Zed’s `theme` + `settings_content` + `theme_settings` crates.

## Primary links

- Docs: [Themes](https://zed.dev/docs/themes), [Extension themes](https://zed.dev/docs/extensions/themes), [Installing extensions](https://zed.dev/docs/extensions/installing-extensions), [Appearance](https://zed.dev/docs/appearance), [Configuring Zed / theme setting](https://zed.dev/docs/configuring-zed)
- Schema: https://zed.dev/schema/themes/v0.2.0.json
- Blog: [User themes now in Preview](https://zed.dev/blog/user-themes-now-in-preview)
- Source: [`settings_content/src/theme.rs`](https://github.com/zed-industries/zed/blob/main/crates/settings_content/src/theme.rs), [`theme_settings/src/schema.rs`](https://github.com/zed-industries/zed/blob/main/crates/theme_settings/src/schema.rs), [`theme/src/theme.rs`](https://github.com/zed-industries/zed/blob/main/crates/theme/src/theme.rs), [`theme/src/styles/colors.rs`](https://github.com/zed-industries/zed/blob/main/crates/theme/src/styles/colors.rs), [`paths/src/paths.rs`](https://github.com/zed-industries/zed/blob/main/crates/paths/src/paths.rs), [`assets/themes/one/one.json`](https://github.com/zed-industries/zed/blob/main/assets/themes/one/one.json)
- ultraViolet: https://zed.dev/extensions/ultraviolet-theme , https://github.com/Gurvirr/zed-ultraViolet
