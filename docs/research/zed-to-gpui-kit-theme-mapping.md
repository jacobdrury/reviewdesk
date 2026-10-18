# Zed theme JSON → gpui-kit 0.7.3 Theme mapping

Facts only. No product decisions. Answers [issue #11](https://github.com/jacobdrury/reviewdesk/issues/11) on map [#9](https://github.com/jacobdrury/reviewdesk/issues/9).

Research date: 2026-10-17.

Primary sources:

- Zed style / color / syntax content types: [`crates/settings_content/src/theme.rs`](https://github.com/zed-industries/zed/blob/main/crates/settings_content/src/theme.rs) (`ThemeStyleContent`, `ThemeColorsContent`, `StatusColorsContent`, `HighlightStyleContent`)
- Example filled theme: [`assets/themes/one/one.json`](https://github.com/zed-industries/zed/blob/main/assets/themes/one/one.json)
- Prior path/format notes: [`docs/research/zed-theme-format.md`](zed-theme-format.md)
- gpui-kit 0.7.3 re-exports gpui-component 0.7.3 theme APIs (`~/.cargo/registry/.../gpui-kit-0.7.3`, `.../gpui-component-0.7.3`)
- Kit types: `theme/schema.rs` (`ThemeSet`, `ThemeConfig`, `ThemeConfigColors`), `theme/theme_color.rs` (`ThemeColor`, `ThemeTokens`), `theme/mod.rs` (`Theme`, `ThemeMode`), `theme/registry.rs` (`ThemeRegistry`), `highlighter/registry.rs` (`HighlightTheme`, `HighlightThemeStyle`, `StatusColors`, `SyntaxColors`, `ThemeStyle`)

## 1. Shape mismatch (why raw Zed JSON cannot load as a kit theme)

| Layer | Zed | gpui-kit 0.7.3 |
| --- | --- | --- |
| Family | `ThemeFamilyContent`: `name`, `author`, `themes[]` | `ThemeSet`: `name`, `author?`, `url?`, `themes[]` |
| Per theme | `ThemeContent`: `name`, `appearance` (`light`\|`dark`), `style` | `ThemeConfig`: `name`, `mode` (`ThemeMode`), optional font/radius/shadow, `colors`, `highlight?` |
| Chrome + editor + status + syntax | One flat `style` object | Split: UI chrome → `colors` (`ThemeConfigColors` → `ThemeColor`); editor/status/syntax → `highlight` (`HighlightThemeStyle` → `Theme.highlight_theme`) |

`ThemeRegistry::load_themes_from_str` / `watch_dir` deserialize **`ThemeSet` only**. A Zed family file has `appearance` and `style`, not `mode` / `colors` / `highlight`. Serde defaults leave `colors` empty and `mode` as light; Zed chrome keys are ignored. An adapter that rewrites each theme entry is required before registry load or `Theme::apply_config`.

`Theme::apply_config` copies `config.highlight` into `Theme.highlight_theme` when present, then runs `ThemeColor::apply_config` for `colors` (with kit fallbacks for unset fields). Source: `schema.rs` `Theme::apply_config` / `ThemeColor::apply_config`.

## 2. Clean mappings (same JSON key → kit field, after lift)

These Zed `style` keys already use the same serde rename strings as kit APIs. They map **only after** moving them into the right kit section (`colors` vs `highlight`).

### 2.1 Exact name → `ThemeConfig.colors` / `ThemeColor`

| Zed `style` key | Kit `ThemeConfigColors` / `ThemeColor` field |
| --- | --- |
| `background` | `background` |
| `border` | `border` |
| `drop_target.background` | `drop_target` (`drop_target.background`) |
| `scrollbar.thumb.background` | `scrollbar_thumb` |
| `status_bar.background` | `status_bar` |
| `tab_bar.background` | `tab_bar` |
| `title_bar.background` | `title_bar` |

That is the full exact-name intersection of Zed `ThemeColorsContent` (144 keys) with kit `ThemeConfigColors` (~140 keys), excluding status keys that collide by name but live in different kit sections (see §4).

### 2.2 Exact name → `ThemeConfig.highlight` / `HighlightThemeStyle`

Lifted from Zed flat `style` into kit `highlight`:

| Zed `style` key | Kit field |
| --- | --- |
| `editor.background` | `editor_background` |
| `editor.foreground` | `editor_foreground` |
| `editor.active_line.background` | `editor_active_line` |
| `editor.line_number` | `editor_line_number` |
| `editor.active_line_number` | `editor_active_line_number` |
| `editor.invisible` | `editor_invisible` |
| `editor.gutter.background` | `editor_gutter_background` |

### 2.3 Exact name → `HighlightThemeStyle.status` (`StatusColors`)

| Zed status key | Kit `StatusColors` |
| --- | --- |
| `error`, `error.background`, `error.border` | yes |
| `warning`, `warning.background`, `warning.border` | yes |
| `info`, `info.background`, `info.border` | yes |
| `success`, `success.background`, `success.border` | yes |
| `hint`, `hint.background`, `hint.border` | yes |

Kit documents highlight as Zed-style compatible (`HighlightTheme` docs link Zed language highlighting; `ThemeConfig.highlight` comment points at Zed `style`). Hex strings deserialize via gpui `Hsla` ← `Rgba` hex visitor (`#RGB` / `#RGBA` / `#RRGGBB` / `#RRGGBBAA`).

### 2.4 `syntax` → `HighlightThemeStyle.syntax` (`SyntaxColors`)

Zed: `style.syntax` map of capture → `{ color?, background_color?, font_style?, font_weight? }`.

Kit: `highlight.syntax` (`SyntaxColors`) of capture → `ThemeStyle` `{ color?, font_style?, font_weight? }`.

Known captures with serde renames matching Zed names (and `HIGHLIGHT_NAMES`): `attribute`, `boolean`, `comment`, `constant`, `constructor`, `embedded`, `emphasis`, `emphasis.strong`, `enum`, `function`, `hint`, `keyword`, `label`, `link_text`, `link_uri`, `number`, `operator`, `predictive`, `preproc`, `primary`, `property`, `punctuation`, `punctuation.bracket`, `punctuation.delimiter`, `punctuation.list_marker`, `punctuation.special`, `string`, `string.escape`, `string.regex`, `string.special`, `string.special.symbol`, `tag`, `tag.doctype`, `text.code.span`, `text.literal`, `title`, `type`, `variable`, `variable.special`, `variant`.

Runtime lookup falls back from `a.b` to `a` when the exact capture is missing (`SyntaxColors::style`).

## 3. Partial mappings (same idea, different key, incomplete field, or dual home)

### 3.1 Wrapper / mode

| Zed | Kit | Notes |
| --- | --- | --- |
| `appearance`: `"light"` \| `"dark"` | `mode`: `ThemeMode` | Same words; different field name. Kit defaults to light if omitted. |
| Family `name` / `author` | `ThemeSet.name` / `author` | Clean. Kit also allows `url`. |
| Per-theme `name` | `ThemeConfig.name` | Clean. |

### 3.2 Chrome: semantic cousins (adapter table, not drop-in)

Illustrative, not exhaustive. These are common Zed chrome keys with no identical kit key; a mapper must choose a kit token (or leave kit default):

| Zed key(s) | Plausible kit target(s) | Why partial |
| --- | --- | --- |
| `text` | `foreground` | Different name |
| `text.muted` / `text.disabled` / `text.placeholder` | `muted.foreground` (and maybe others) | One kit muted text token |
| `text.accent` | `accent.foreground` / `primary` | Ambiguous |
| `surface.background` / `elevated_surface.background` / `panel.background` | `background`, `sidebar.background`, `popover.background`, `list.background` | Several kit surfaces |
| `element.background` / `.hover` / `.active` / `.selected` | `accent.*`, `list.hover`, `list.active.*`, `secondary.*` | Zed element stack vs shadcn tokens |
| `ghost_element.*` | often transparent / accent overlays | No ghost tokens in kit |
| `tab.active_background` / `tab.inactive_background` | `tab.active.background` / `tab.background` | Underscore vs dotted kit names |
| `scrollbar.thumb.hover_background` / `scrollbar.track.background` | `scrollbar.thumb.hover.background` / `scrollbar.background` | Naming drift; track ≠ kit scrollbar bg semantics |
| `title_bar.inactive_background` | (none dedicated) | Only `title_bar.background` / `.border` |
| `link_text.hover` | `link.hover` | Close; kit also has `link` / `link.active` |
| `editor.foreground` / `editor.background` | Also inform UI `foreground` / `background` if chrome and editor should match | In kit these live under **highlight**, not `ThemeColor`, unless duplicated into `colors` |
| `created` / `deleted` / `modified` (status) | Diff UI uses `ThemeColor.success` / `danger` (see `diff/mod.rs`), not highlight status | Must map status → chrome colors for hunk tinting |
| `version_control.*` | same diff `success` / `danger` (or custom app tokens) | No kit fields with those names |

### 3.3 Syntax field gaps inside an otherwise clean capture

| Zed `HighlightStyleContent` | Kit `ThemeStyle` |
| --- | --- |
| `color` | `color` |
| `font_weight` | `font_weight` (`FontWeightContent` 100–900) |
| `font_style`: `normal` \| `italic` \| `oblique` | `FontStyle`: `normal` \| `italic` \| `underline` (`underline` maps to gpui Normal). **No `oblique`.** |
| `background_color` | **Absent.** Dropped on import. |

### 3.4 `comment.doc` serde quirk

`SyntaxColors` has `comment_doc` **without** `#[serde(rename = "comment.doc")]`, while other dotted captures have renames. Zed (and kit’s own `default-theme.json`) use `"comment.doc"`. That key does not populate `comment_doc`; highlight time falls back to `comment` via the dotted-prefix rule. Treat as partial unless the adapter renames to `comment_doc` or a rename is fixed upstream.

### 3.5 Name collision: status vs chrome `info` / `success` / `warning`

Zed flat `style` has `info.background` (status). Kit has:

- `colors["info.background"]` → `ThemeColor.info` (component / badge chrome)
- `highlight` flattened status → `StatusColors.info_background` (editor diagnostics)

Same string, two destinations. An adapter must decide whether a Zed status key fills highlight status, chrome danger/info/success, or both.

### 3.6 Kit-only chrome (no Zed source key)

Almost all shadcn-style kit keys have no Zed counterpart: `button.*`, `primary.*`, `secondary.*`, `chart.*`, `base.red` / `base.green` / …, `accordion.*`, `group_box.*`, `table.*`, `slider.*`, `switch.*`, `ring`, `overlay`, `window.border`, radius/font/shadow theme options, etc. `ThemeColor::apply_config` synthesizes many of these from defaults and blends when omitted. A Zed import that only sets the exact/partial chrome keys still boots with kit fallbacks for the rest.

## 4. No home in gpui-kit 0.7.3 today

### 4.1 Zed style meta

- `background.appearance` (`opaque` \| `transparent` \| `blurred` \| mica variants)
- `accents` array
- `players` (`cursor` / `background` / `selection` per player)

### 4.2 Zed `ThemeColorsContent` with no kit twin (representative groups)

Everything in `ThemeColorsContent` except the seven exact chrome matches in §2.1, including:

- Borders: `border.variant`, `.focused`, `.selected`, `.transparent`, `.disabled`
- Surfaces / panels / panes: `elevated_surface.*`, `surface.*`, `panel.*`, `pane.*`, `pane_group.*`, `toolbar.*`
- Elements / ghost elements (full stacks)
- Text / icon ladders beyond a single muted token
- Editor extras: wrap guides, indent guides, document highlights, debugger line, code lens, hover line number, **all `editor.diff_hunk.*`**
- Search match colors
- Scrollbar thumb border / active; full minimap thumb set
- Entire `terminal.*` / ANSI set
- Entire `vim.*` / helix set
- `debugger.accent`
- Entire `version_control.*` set
- `drop_target.border` (kit has background only)

### 4.3 Zed status keys missing from kit `StatusColors`

`conflict`, `created`, `deleted`, `hidden`, `ignored`, `modified`, `predictive`, `renamed`, `unreachable`, and each one’s `.background` / `.border`. (Some names exist as syntax captures or as chrome `ThemeColor` success/danger, but not as highlight status fields.)

### 4.4 Runtime APIs that do not accept Zed files

- `ThemeRegistry` expects kit `ThemeSet` JSON (see §1).
- `SemanticThemeConfig` / `SemanticThemeTokens` is a separate smaller token set; not the Zed import path documented on `ThemeConfig.highlight`.
- Diff hunk tinting reads `cx.theme().success` / `danger` (`ThemeColor`), not Zed `version_control.*` or status `created`/`deleted`.

## 5. Practical import pipeline (factual)

1. Deserialize Zed family (`ThemeFamilyContent` / equivalent JSON).
2. Pick a theme by `name` and/or `appearance`.
3. Build a `ThemeConfig`:
   - `name` ← theme name; `mode` ← `appearance`.
   - `colors` ← map §2.1 exact keys + any chosen §3.2 semantic map; leave the rest unset for kit fallbacks.
   - `highlight` ← copy §2.2 editor keys + §2.3 status subset + entire `syntax` object (with §3.3 / `comment.doc` caveats).
4. Apply via `Theme::apply_config` and/or insert into `ThemeRegistry` as a kit theme set (not the original Zed file).
5. For Review Desk “everything including diff syntax”: syntax colors come from `Theme.highlight_theme`; hunk chrome still needs an explicit map into `ThemeColor` success/danger (or future tokens), because those Zed keys have no kit home.

## 6. Coverage snapshot (orders of magnitude)

| Bucket | Approx. count | Role |
| --- | --- | --- |
| Zed chrome color keys (`ThemeColorsContent`) | 144 | UI chrome |
| Exact chrome key matches to kit `colors` | 7 | Drop-in after lift |
| Zed status keys | 42 | Diagnostics / VCS-ish |
| Kit highlight status keys | 15 (5 × 3) | Subset of Zed status |
| Kit highlight editor keys | 7 | Subset of Zed editor.* |
| Syntax | Free-form in Zed; 41 named slots in kit + prefix fallback | Strongest “Zed-compatible” surface |
| Kit `ThemeConfigColors` with no Zed twin | ~130 | Filled by kit defaults / blends |

## Primary links

- Issue: https://github.com/jacobdrury/reviewdesk/issues/11
- Map: https://github.com/jacobdrury/reviewdesk/issues/9
- Zed settings_content theme: https://github.com/zed-industries/zed/blob/main/crates/settings_content/src/theme.rs
- Zed One Dark example: https://github.com/zed-industries/zed/blob/main/assets/themes/one/one.json
- Zed themes docs: https://zed.dev/docs/themes
- gpui-kit / gpui-component 0.7.3 crates.io packages (local registry checkout used for this note)
