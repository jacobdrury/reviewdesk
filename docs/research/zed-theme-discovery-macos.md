# Zed theme discovery on macOS

Facts only. No product decisions. Answers: where Zed stores the active theme selection and theme JSON on macOS, and how a name such as Ultraviolet resolves to a file and appearance.

Research date: 2026-10-17. Sources are zed.dev docs, `github.com/zed-industries/zed` `main`, and a local Zed install on this Mac.

Related: [zed-theme-format.md](./zed-theme-format.md) covers JSON schema and token shape. This note covers path discovery and name resolution only.

## 1. macOS path roots

From [`crates/paths/src/paths.rs`](https://github.com/zed-industries/zed/blob/main/crates/paths/src/paths.rs):

| Function | macOS path |
| --- | --- |
| `config_dir()` | `~/.config/zed` |
| `data_dir()` | `~/Library/Application Support/Zed` |
| `settings_file()` | `~/.config/zed/settings.json` |
| `themes_dir()` | `~/.config/zed/themes` (“themes that are not provided by extensions”) |
| `extensions_dir()` | `~/Library/Application Support/Zed/extensions` |

Extension host then uses `extensions_dir()/installed` as the install root and `extensions_dir()/index.json` as the theme-name index ([`extension_host.rs`](https://github.com/zed-industries/zed/blob/main/crates/extension_host/src/extension_host.rs)).

Local verify (this machine):

- `~/.config/zed/settings.json` present
- `~/.config/zed/themes/` empty (no user themes)
- `~/Library/Application Support/Zed/extensions/installed/ultraviolet-theme/` present
- `~/Library/Application Support/Zed/extensions/index.json` present

## 2. Active theme selection

Documented in [Themes: Configuring a Theme](https://zed.dev/docs/themes). Stored only in `~/.config/zed/settings.json` under `"theme"`.

Two shapes (`ThemeSelection` in [`crates/settings_content/src/theme.rs`](https://github.com/zed-industries/zed/blob/main/crates/settings_content/src/theme.rs) and [`theme_settings/src/settings.rs`](https://github.com/zed-industries/zed/blob/main/crates/theme_settings/src/settings.rs)):

1. Static string: `"theme": "One Dark"`
2. Dynamic object:

```json
{
  "theme": {
    "mode": "system",
    "light": "One Light",
    "dark": "One Dark"
  }
}
```

`mode` is `system` | `light` | `dark` (`ThemeAppearanceMode`, default `system`).

Resolution to a single theme **name** (`ThemeSelection::name`):

| `mode` | Name used |
| --- | --- |
| `light` | `light` |
| `dark` | `dark` |
| `system` | `light` or `dark` from OS appearance (`SystemAppearance`) |

Defaults if unset: light `"One Light"`, dark `"One Dark"` (`DEFAULT_LIGHT_THEME` / `DEFAULT_DARK_THEME`).

Local verify (`~/.config/zed/settings.json` on this Mac):

```json
"theme": {
  "mode": "system",
  "light": "One Light",
  "dark": "ultraViolet"
}
```

So under dark system appearance the active name is `ultraViolet` (exact casing). Not `Ultraviolet`.

## 3. Where theme JSON lives

### 3.1 Built-ins (bundled assets)

Loaded by `load_bundled_themes` in [`theme_settings.rs`](https://github.com/zed-industries/zed/blob/main/crates/theme_settings/src/theme_settings.rs): list asset paths under `themes/` ending in `.json`, parse each as a theme family, insert into `ThemeRegistry`.

Source tree examples:

- [`assets/themes/one/one.json`](https://github.com/zed-industries/zed/blob/main/assets/themes/one/one.json) (family `"One"`; themes `"One Dark"` / `"One Light"`)
- Also `assets/themes/ayu/`, `assets/themes/gruvbox/`, etc.

On a shipped macOS app they are **not** loose files under `Zed.app/Contents/Resources`. They are embedded in the `zed` binary asset blob. Local check: `strings` on `/Applications/Zed.app/Contents/MacOS/zed` shows `themes/one/one.json`, `themes/ayu/ayu.json`, `themes/gruvbox/gruvbox.json`. There is no on-disk copy to open for built-ins without extracting from the binary or reading upstream assets.

### 3.2 User / local themes

[Themes docs § Local themes](https://zed.dev/docs/themes#local-themes): drop `*.json` in `~/.config/zed/themes`.

`load_user_themes_in_background` in [`crates/zed/src/main.rs`](https://github.com/zed-industries/zed/blob/main/crates/zed/src/main.rs) reads every entry in `themes_dir()`, deserializes via `load_user_theme`, registers all per-theme names from each family’s `themes[]`.

### 3.3 Extension themes

Install layout ([`extension_host.rs`](https://github.com/zed-industries/zed/blob/main/crates/extension_host/src/extension_host.rs)):

```
~/Library/Application Support/Zed/extensions/
  index.json
  installed/<extension-id>/
    extension.toml
    themes/<file>.json
```

Manifest lists theme files (`themes = ["themes/ultraViolet.json"]`). Index rebuild walks `installed/<id>/themes/`, calls `list_theme_names` (reads each JSON family’s `themes[].name`), and stores:

```json
"themes": {
  "<ThemeContent.name>": {
    "extension": "<extension-id>",
    "path": "themes/<file>.json"
  }
}
```

`path_to_extension_theme(name)` → `installed/<extension>/<path>`.

Eager load ([`eager_load_active_theme_and_icon_theme`](https://github.com/zed-industries/zed/blob/main/crates/zed/src/zed.rs)): if the configured name is not already in the registry, resolve via `path_to_extension_theme`, load that JSON file, then `reload_theme`. Full extension load later registers the same files through `theme_extension`.

## 4. Resolving a name to a file and appearance

### Algorithm (external reader, matching Zed)

1. Parse `~/.config/zed/settings.json` → `theme` → pick name with `mode` + system appearance (section 2).
2. Look up that **exact** string:
   - In `~/Library/Application Support/Zed/extensions/index.json` → `themes[name]` → file under `extensions/installed/…`
   - Else scan `~/.config/zed/themes/*.json` for a family whose `themes[].name` equals the name
   - Else treat as built-in: name appears inside an embedded `assets/themes/**/*.json` family (or fall back to `"One Dark"` / `"One Light"` as Zed does when missing after extensions load)
3. Open the family JSON. Find the object in `themes[]` with matching `name`. Read that object’s `"appearance"` (`"light"` | `"dark"`) and `"style"`.

Registry behavior: themes are keyed by each **variant** `ThemeContent.name`, not the family `name` ([`ThemeRegistry::insert_themes`](https://github.com/zed-industries/zed/blob/main/crates/theme/src/registry.rs)). One JSON file can register several selectable names (e.g. One Dark + One Light; ultraViolet + variants).

Missing name after extensions loaded: log error and fall back to `default_theme(appearance)` (`"One Light"` / `"One Dark"`) ([`configured_theme`](https://github.com/zed-industries/zed/blob/main/crates/theme_settings/src/theme_settings.rs)).

### Appearance vs settings mode

- Settings `theme.mode` / system appearance choose **which theme name** is active.
- The theme JSON’s own `"appearance"` field is metadata on that variant (used when refining colors / classifying the theme). It is not a separate settings key.
- A dark-named theme can still be selected as the `"light"` slot (or vice versa); Zed does not require the JSON appearance to match the slot.

## 5. Ultraviolet on this Mac (verified)

| Item | Value |
| --- | --- |
| Extension id | `ultraviolet-theme` |
| Install dir | `~/Library/Application Support/Zed/extensions/installed/ultraviolet-theme/` |
| Manifest | `extension.toml`: `id = "ultraviolet-theme"`, `themes = ["themes/ultraViolet.json"]` |
| Theme file | `…/themes/ultraViolet.json` |
| Family `name` | `ultraViolet` |
| Selectable theme names | `ultraViolet`, `ultraViolet Blur`, `ultraViolet Blur (Borderless)` |
| Each appearance | `"dark"` |
| Index entries | All three names → `{ "extension": "ultraviolet-theme", "path": "themes/ultraViolet.json" }` |
| Settings dark slot | `"ultraViolet"` (exact) |

Resolved path for active dark theme on this machine:

`~/Library/Application Support/Zed/extensions/installed/ultraviolet-theme/themes/ultraViolet.json`

then the array element with `"name": "ultraViolet"` and `"appearance": "dark"`.

Note: colloquial “Ultraviolet” / “Ultraviolet-theme” is the extension branding. The settings / index / JSON theme name is camelCase **`ultraViolet`**. Lookup is case-sensitive.

## 6. Practical recipe for Review Desk (macOS)

To mirror Zed’s active theme without running Zed:

1. Read `~/.config/zed/settings.json` → resolve name (section 2). Respect `theme_overrides` only if you choose to apply them later (out of scope for path discovery).
2. Prefer `extensions/index.json` name → path under `extensions/installed/`.
3. Else scan `~/.config/zed/themes/`.
4. Else ship / embed the same built-in families Zed ships, or fall back to a bundled neutral dark theme when the name is missing.
5. Parse family JSON; select `themes[]` entry by exact `name`; use that entry’s `appearance` and `style` (including `syntax`).

## Sources

- [zed.dev/docs/themes](https://zed.dev/docs/themes)
- [zed.dev/docs/extensions/themes](https://zed.dev/docs/extensions/themes)
- [`crates/paths/src/paths.rs`](https://github.com/zed-industries/zed/blob/main/crates/paths/src/paths.rs)
- [`crates/theme_settings/src/theme_settings.rs`](https://github.com/zed-industries/zed/blob/main/crates/theme_settings/src/theme_settings.rs)
- [`crates/theme_settings/src/settings.rs`](https://github.com/zed-industries/zed/blob/main/crates/theme_settings/src/settings.rs)
- [`crates/settings_content/src/theme.rs`](https://github.com/zed-industries/zed/blob/main/crates/settings_content/src/theme.rs)
- [`crates/theme/src/registry.rs`](https://github.com/zed-industries/zed/blob/main/crates/theme/src/registry.rs)
- [`crates/extension_host/src/extension_host.rs`](https://github.com/zed-industries/zed/blob/main/crates/extension_host/src/extension_host.rs)
- [`crates/zed/src/zed.rs` (`eager_load_active_theme_and_icon_theme`)](https://github.com/zed-industries/zed/blob/main/crates/zed/src/zed.rs)
- [`crates/zed/src/main.rs` (user theme load/watch)](https://github.com/zed-industries/zed/blob/main/crates/zed/src/main.rs)
- Local: `~/.config/zed/settings.json`, `~/Library/Application Support/Zed/extensions/{index.json,installed/ultraviolet-theme/}`, `/Applications/Zed.app/Contents/MacOS/zed`
