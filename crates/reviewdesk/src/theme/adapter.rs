use gpui_kit::component::ThemeConfig;
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdaptError {
    InvalidJson(String),
    MissingThemes,
    ThemeNotFound(String),
    InvalidTheme(String),
}

impl std::fmt::Display for AdaptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(message) => write!(f, "invalid theme JSON: {message}"),
            Self::MissingThemes => write!(f, "theme family has no themes"),
            Self::ThemeNotFound(name) => write!(f, "theme not found: {name}"),
            Self::InvalidTheme(message) => write!(f, "invalid adapted theme: {message}"),
        }
    }
}

impl std::error::Error for AdaptError {}

/// Rewrite a Zed theme family JSON document into kit `ThemeConfig` entries.
pub fn adapt_zed_family(family_json: &str) -> Result<Vec<ThemeConfig>, AdaptError> {
    let family: Value = serde_json::from_str(family_json)
        .map_err(|error| AdaptError::InvalidJson(error.to_string()))?;
    let themes = family
        .get("themes")
        .and_then(Value::as_array)
        .ok_or(AdaptError::MissingThemes)?;

    let mut adapted = Vec::with_capacity(themes.len());
    for theme in themes {
        adapted.push(adapt_zed_theme_value(theme)?);
    }
    Ok(adapted)
}

/// Rewrite a single Zed theme object (`name` / `appearance` / `style`) into a kit `ThemeConfig`.
#[cfg_attr(not(test), allow(dead_code))]
pub fn adapt_zed_theme(theme_json: &str) -> Result<ThemeConfig, AdaptError> {
    let theme: Value = serde_json::from_str(theme_json)
        .map_err(|error| AdaptError::InvalidJson(error.to_string()))?;
    adapt_zed_theme_value(&theme)
}

fn adapt_zed_theme_value(theme: &Value) -> Result<ThemeConfig, AdaptError> {
    let name = theme
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| AdaptError::InvalidTheme("missing theme name".into()))?;
    let appearance = theme
        .get("appearance")
        .and_then(Value::as_str)
        .unwrap_or("light");
    let mode = match appearance {
        "dark" => "dark",
        _ => "light",
    };

    let style = theme
        .get("style")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let colors = map_chrome_colors(&style);
    let highlight = map_highlight(&style);

    let kit_theme = serde_json::json!({
        "name": name,
        "mode": mode,
        "colors": colors,
        "highlight": highlight,
    });

    serde_json::from_value(kit_theme)
        .map_err(|error| AdaptError::InvalidTheme(error.to_string()))
}

fn map_chrome_colors(style: &Map<String, Value>) -> Map<String, Value> {
    let mut colors = Map::new();

    // Exact chrome (7).
    copy_string(style, "background", &mut colors, "background");
    copy_string(style, "border", &mut colors, "border");
    copy_string(
        style,
        "drop_target.background",
        &mut colors,
        "drop_target.background",
    );
    copy_string(
        style,
        "scrollbar.thumb.background",
        &mut colors,
        "scrollbar.thumb.background",
    );
    copy_string(
        style,
        "status_bar.background",
        &mut colors,
        "status_bar.background",
    );
    copy_string(style, "tab_bar.background", &mut colors, "tab_bar.background");
    copy_string(
        style,
        "title_bar.background",
        &mut colors,
        "title_bar.background",
    );

    // Partial chrome.
    copy_string(style, "text", &mut colors, "foreground");
    if let Some(muted) = first_string(
        style,
        &[
            "text.muted",
            "text.disabled",
            "text.placeholder",
        ],
    ) {
        colors.insert("muted.foreground".into(), Value::String(muted));
    }
    copy_string(style, "text.accent", &mut colors, "accent.foreground");

    if let Some(surface) = first_string(
        style,
        &[
            "panel.background",
            "surface.background",
            "elevated_surface.background",
        ],
    ) {
        insert_if_absent(&mut colors, "background", &surface);
        colors.insert("sidebar.background".into(), Value::String(surface.clone()));
        colors.insert("list.background".into(), Value::String(surface.clone()));
        colors.insert("popover.background".into(), Value::String(surface));
    }

    copy_string(
        style,
        "tab.active_background",
        &mut colors,
        "tab.active.background",
    );
    copy_string(
        style,
        "tab.inactive_background",
        &mut colors,
        "tab.background",
    );
    copy_string(
        style,
        "element.selected",
        &mut colors,
        "list.active.background",
    );
    copy_string(
        style,
        "element.hover",
        &mut colors,
        "list.hover.background",
    );

    // Status dual-write into chrome info/success/warning/danger.
    // Diff hunks: created → success, deleted → danger (else status success/error).
    if let Some(success) = first_string(
        style,
        &[
            "created.background",
            "created",
            "success.background",
            "success",
        ],
    ) {
        colors.insert("success.background".into(), Value::String(success));
    }
    if let Some(danger) = first_string(
        style,
        &[
            "deleted.background",
            "deleted",
            "error.background",
            "error",
        ],
    ) {
        colors.insert("danger.background".into(), Value::String(danger));
    }
    if let Some(info) = first_string(style, &["info.background", "info"]) {
        colors.insert("info.background".into(), Value::String(info));
    }
    if let Some(warning) = first_string(style, &["warning.background", "warning"]) {
        colors.insert("warning.background".into(), Value::String(warning));
    }

    colors
}

fn map_highlight(style: &Map<String, Value>) -> Map<String, Value> {
    let mut highlight = Map::new();

    for (zed_key, kit_key) in [
        ("editor.background", "editor.background"),
        ("editor.foreground", "editor.foreground"),
        ("editor.active_line.background", "editor.active_line.background"),
        ("editor.line_number", "editor.line_number"),
        ("editor.active_line_number", "editor.active_line_number"),
        ("editor.invisible", "editor.invisible"),
        ("editor.gutter.background", "editor.gutter.background"),
    ] {
        copy_string(style, zed_key, &mut highlight, kit_key);
    }

    for key in [
        "error",
        "error.background",
        "error.border",
        "warning",
        "warning.background",
        "warning.border",
        "info",
        "info.background",
        "info.border",
        "success",
        "success.background",
        "success.border",
        "hint",
        "hint.background",
        "hint.border",
    ] {
        copy_string(style, key, &mut highlight, key);
    }

    let syntax = style
        .get("syntax")
        .and_then(Value::as_object)
        .map(map_syntax)
        .unwrap_or_default();
    // HighlightThemeStyle.syntax is required (not Option).
    highlight.insert("syntax".into(), Value::Object(syntax));

    highlight
}

fn map_syntax(syntax: &Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::new();
    for (capture, style_value) in syntax {
        let Some(entry) = style_value.as_object() else {
            continue;
        };

        let mut mapped = Map::new();
        if let Some(color) = entry.get("color").and_then(Value::as_str) {
            mapped.insert("color".into(), Value::String(color.to_string()));
        }
        if let Some(weight) = entry.get("font_weight") {
            if !weight.is_null() {
                mapped.insert("font_weight".into(), weight.clone());
            }
        }
        if let Some(font_style) = entry.get("font_style").and_then(Value::as_str) {
            match font_style {
                "italic" | "normal" | "underline" => {
                    mapped.insert("font_style".into(), Value::String(font_style.to_string()));
                }
                // Kit has no oblique; drop per mapping research.
                "oblique" => {}
                _ => {}
            }
        }
        // background_color has no kit home; drop.

        if mapped.is_empty() {
            continue;
        }

        let key = if capture.as_str() == "comment.doc" {
            // SyntaxColors.comment_doc lacks #[serde(rename = "comment.doc")].
            "comment_doc".to_string()
        } else {
            capture.clone()
        };
        out.insert(key, Value::Object(mapped));
    }
    out
}

fn copy_string(
    style: &Map<String, Value>,
    from: &str,
    into: &mut Map<String, Value>,
    to: &str,
) {
    if let Some(value) = style.get(from).and_then(Value::as_str) {
        into.insert(to.into(), Value::String(value.to_string()));
    }
}

fn first_string(style: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| style.get(*key).and_then(Value::as_str))
        .map(str::to_string)
}

fn insert_if_absent(colors: &mut Map<String, Value>, key: &str, value: &str) {
    if colors.contains_key(key) {
        return;
    }
    colors.insert(key.into(), Value::String(value.to_string()));
}

/// True when `family_json` deserializes as a kit `ThemeSet` without a Zed rewrite.
///
/// Used in tests to prove raw Zed JSON is not a usable kit theme family.
#[cfg(test)]
pub(crate) fn raw_zed_loads_as_kit_theme_set(
    family_json: &str,
) -> Option<gpui_kit::component::ThemeSet> {
    serde_json::from_str(family_json).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::fallback::{fallback_one_dark, ONE_FAMILY_JSON, FALLBACK_THEME_NAME};
    use gpui_kit::component::ThemeMode;
    use serde_json::json;

    fn color(colors: &gpui_kit::component::ThemeConfigColors, get: impl FnOnce(&gpui_kit::component::ThemeConfigColors) -> &Option<gpui_kit::SharedString>) -> Option<String> {
        get(colors).as_ref().map(|value| value.to_string())
    }

    fn highlight_json(config: &ThemeConfig) -> Value {
        serde_json::to_value(config.highlight.as_ref().expect("highlight")).unwrap()
    }

    #[test]
    fn adapts_one_dark_must_map_exact_chrome() {
        let config = fallback_one_dark().unwrap();
        assert_eq!(config.name.as_ref(), FALLBACK_THEME_NAME);
        assert_eq!(config.mode, ThemeMode::Dark);

        let cases = [
            ("background", color(&config.colors, |c| &c.background), "#3b414dff"),
            ("border", color(&config.colors, |c| &c.border), "#464b57ff"),
            (
                "drop_target",
                color(&config.colors, |c| &c.drop_target),
                "#83899480",
            ),
            (
                "scrollbar_thumb",
                color(&config.colors, |c| &c.scrollbar_thumb),
                "#c8ccd44c",
            ),
            (
                "status_bar",
                color(&config.colors, |c| &c.status_bar),
                "#3b414dff",
            ),
            ("tab_bar", color(&config.colors, |c| &c.tab_bar), "#2f343eff"),
            (
                "title_bar",
                color(&config.colors, |c| &c.title_bar),
                "#3b414dff",
            ),
        ];
        for (name, got, want) in cases {
            assert_eq!(got.as_deref(), Some(want), "{name}");
        }
    }

    #[test]
    fn adapts_partial_chrome_and_status_dual_write() {
        let config = fallback_one_dark().unwrap();
        let cases = [
            ("foreground", color(&config.colors, |c| &c.foreground), "#dce0e5ff"),
            (
                "muted_foreground",
                color(&config.colors, |c| &c.muted_foreground),
                "#a9afbcff",
            ),
            (
                "accent_foreground",
                color(&config.colors, |c| &c.accent_foreground),
                "#74ade8ff",
            ),
            ("sidebar", color(&config.colors, |c| &c.sidebar), "#2f343eff"),
            ("list", color(&config.colors, |c| &c.list), "#2f343eff"),
            ("popover", color(&config.colors, |c| &c.popover), "#2f343eff"),
            ("tab", color(&config.colors, |c| &c.tab), "#2f343eff"),
            (
                "tab_active",
                color(&config.colors, |c| &c.tab_active),
                "#282c33ff",
            ),
            (
                "list_active",
                color(&config.colors, |c| &c.list_active),
                "#454a56ff",
            ),
            (
                "list_hover",
                color(&config.colors, |c| &c.list_hover),
                "#363c46ff",
            ),
            (
                "success_from_created",
                color(&config.colors, |c| &c.success),
                "#a1c1811a",
            ),
            (
                "danger_from_deleted",
                color(&config.colors, |c| &c.danger),
                "#d072771a",
            ),
            ("info", color(&config.colors, |c| &c.info), "#74ade81a"),
            ("warning", color(&config.colors, |c| &c.warning), "#dec1841a"),
        ];
        for (name, got, want) in cases {
            assert_eq!(got.as_deref(), Some(want), "{name}");
        }
    }

    #[test]
    fn adapts_highlight_editor_status_and_syntax() {
        let config = fallback_one_dark().unwrap();
        let highlight = highlight_json(&config);

        assert_eq!(highlight["editor.background"], json!("#282c33ff"));
        assert_eq!(highlight["editor.foreground"], json!("#acb2beff"));
        assert_eq!(highlight["success"], json!("#a1c181ff"));
        assert_eq!(highlight["error"], json!("#d07277ff"));
        assert_eq!(highlight["syntax"]["keyword"]["color"], json!("#b477cfff"));
        // comment.doc → comment_doc workaround
        assert_eq!(
            highlight["syntax"]["comment_doc"]["color"],
            json!("#878e98ff")
        );
        assert!(highlight["syntax"].get("comment.doc").is_none());
    }

    #[test]
    fn missing_must_map_keys_leave_kit_defaults() {
        let theme = r##"{
            "name": "Sparse",
            "appearance": "dark",
            "style": {
                "background": "#111111ff",
                "syntax": { "keyword": { "color": "#aabbccff" } }
            }
        }"##;
        let config = adapt_zed_theme(theme).unwrap();
        assert_eq!(color(&config.colors, |c| &c.background).as_deref(), Some("#111111ff"));
        assert!(config.colors.border.is_none());
        assert!(config.colors.foreground.is_none());
        assert!(config.colors.success.is_none());
        assert!(config.colors.danger.is_none());
        let highlight = highlight_json(&config);
        assert!(
            highlight
                .get("editor.background")
                .is_none_or(Value::is_null),
            "missing editor.background must stay unset"
        );
        assert_eq!(highlight["syntax"]["keyword"]["color"], json!("#aabbccff"));
    }

    #[test]
    fn drops_syntax_background_color_and_oblique() {
        let theme = r##"{
            "name": "Drop",
            "appearance": "dark",
            "style": {
                "syntax": {
                    "comment": {
                        "color": "#999999ff",
                        "background_color": "#000000ff",
                        "font_style": "oblique"
                    },
                    "keyword": {
                        "color": "#bbbbbbff",
                        "font_style": "italic"
                    }
                }
            }
        }"##;
        let config = adapt_zed_theme(theme).unwrap();
        let highlight = highlight_json(&config);
        let comment = &highlight["syntax"]["comment"];
        assert_eq!(comment["color"], json!("#999999ff"));
        assert!(comment.get("background_color").is_none());
        assert!(
            comment.get("font_style").is_none_or(Value::is_null),
            "oblique must be dropped"
        );
        assert_eq!(highlight["syntax"]["keyword"]["font_style"], json!("italic"));
    }

    #[test]
    fn raw_zed_family_is_not_usable_kit_theme_set() {
        // Serde may accept the family wrapper, but without rewrite mode stays light
        // and highlight/chrome from Zed `style` are absent.
        let raw = raw_zed_loads_as_kit_theme_set(ONE_FAMILY_JSON).expect("parses as ThemeSet shell");
        let one_dark = raw
            .themes
            .iter()
            .find(|theme| theme.name.as_ref() == FALLBACK_THEME_NAME)
            .expect("One Dark entry");
        assert_eq!(one_dark.mode, ThemeMode::Light);
        assert!(one_dark.highlight.is_none());
        assert!(one_dark.colors.background.is_none());

        let adapted = fallback_one_dark().unwrap();
        assert_eq!(adapted.mode, ThemeMode::Dark);
        assert!(adapted.highlight.is_some());
        assert!(adapted.colors.background.is_some());
    }

    #[test]
    fn appearance_maps_to_mode_table() {
        let cases = [
            ("dark", ThemeMode::Dark),
            ("light", ThemeMode::Light),
        ];
        for (appearance, mode) in cases {
            let theme = format!(
                r#"{{"name":"T","appearance":"{appearance}","style":{{}}}}"#
            );
            let config = adapt_zed_theme(&theme).unwrap();
            assert_eq!(config.mode, mode, "appearance={appearance}");
        }
    }
}
