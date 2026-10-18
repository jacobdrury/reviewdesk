use std::rc::Rc;

use gpui_kit::component::notification::Notification;
use gpui_kit::component::{Theme, ThemeConfig};
use gpui_kit::{Anchor, App};
use serde_json::Value;

use super::adapter::{AdaptError, adapt_zed_family};
use super::fallback::{FALLBACK_THEME_NAME, fallback_one_dark};

const DEFAULT_LIGHT_THEME: &str = "One Light";
const DEFAULT_DARK_THEME: &str = "One Dark";

/// System appearance for dynamic `{mode, light, dark}` settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Light,
    Dark,
}

/// Toast policy for a sync attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToastIntent {
    None,
    /// Named theme from settings could not be resolved.
    MissingTheme {
        name: String,
    },
}

/// Injectable discovery materials (extension index → user → bundled).
#[derive(Debug, Clone, Default)]
pub struct DiscoveryInputs<'a> {
    /// Raw `extensions/index.json` text.
    pub extension_index: Option<&'a str>,
    /// Installed extension families: `(extension_id, relative_path, family_json)`.
    pub extension_families: &'a [(&'a str, &'a str, &'a str)],
    /// User theme family JSON documents from `~/.config/zed/themes`.
    pub user_families: &'a [&'a str],
    /// Bundled/vendored built-in families (e.g. One).
    pub bundled_families: &'a [&'a str],
}

/// Injectable inputs for the Zed theme sync seam.
#[derive(Debug, Clone)]
pub struct SyncInputs<'a> {
    /// False when Zed is not installed / not discoverable.
    pub zed_present: bool,
    /// Zed `settings.json` text when readable.
    pub settings_text: Option<&'a str>,
    /// System appearance for dynamic theme name selection.
    pub system_appearance: Appearance,
    /// Lookup materials for the active theme name.
    pub discovery: DiscoveryInputs<'a>,
}

impl SyncInputs<'static> {
    /// Inputs that always take the silent Fallback path (no Zed / no settings).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn absent_zed() -> Self {
        Self {
            zed_present: false,
            settings_text: None,
            system_appearance: Appearance::Dark,
            discovery: DiscoveryInputs::default(),
        }
    }
}

/// Result of one sync attempt, ready for the shell to apply.
#[derive(Debug, Clone)]
#[allow(dead_code)] // theme_name / used_fallback observed by tests and future shell UI.
pub struct SyncResult {
    pub theme_name: String,
    pub config: ThemeConfig,
    pub used_fallback: bool,
    pub toast: ToastIntent,
}

/// Resolve a theme from injectable discovery inputs.
///
/// Silent Fallback when Zed or settings are missing. Named theme that cannot
/// be resolved → Fallback + [`ToastIntent::MissingTheme`].
pub fn sync_theme(inputs: SyncInputs<'_>) -> SyncResult {
    if !inputs.zed_present || inputs.settings_text.is_none() {
        return silent_fallback();
    }

    let settings_text = inputs.settings_text.expect("checked above");
    let Some(theme_name) = resolve_active_theme_name(settings_text, inputs.system_appearance)
    else {
        return silent_fallback();
    };

    match lookup_theme(&theme_name, &inputs.discovery) {
        Some(config) => SyncResult {
            theme_name: config.name.to_string(),
            config,
            used_fallback: false,
            toast: ToastIntent::None,
        },
        None => missing_theme_fallback(theme_name),
    }
}

fn silent_fallback() -> SyncResult {
    let config = fallback_one_dark().expect("vendored One Dark must adapt");
    SyncResult {
        theme_name: FALLBACK_THEME_NAME.to_string(),
        config,
        used_fallback: true,
        toast: ToastIntent::None,
    }
}

fn missing_theme_fallback(name: String) -> SyncResult {
    let config = fallback_one_dark().expect("vendored One Dark must adapt");
    SyncResult {
        theme_name: FALLBACK_THEME_NAME.to_string(),
        config,
        used_fallback: true,
        toast: ToastIntent::MissingTheme { name },
    }
}

/// Parse Zed `settings.json` text (JSONC: comments + trailing commas).
fn parse_settings_value(settings_text: &str) -> Option<Value> {
    let mut buf = settings_text.to_owned();
    json_strip_comments::strip(&mut buf).ok()?;
    serde_json::from_str(&buf).ok()
}

/// Resolve the active Zed theme name from settings + system appearance.
pub fn resolve_active_theme_name(settings_text: &str, appearance: Appearance) -> Option<String> {
    let value = parse_settings_value(settings_text)?;
    let Some(theme) = value.get("theme") else {
        return Some(default_theme_name(appearance).to_string());
    };

    if let Some(name) = theme.as_str() {
        return Some(name.to_string());
    }

    let obj = theme.as_object()?;
    let mode = obj.get("mode").and_then(Value::as_str).unwrap_or("system");
    let use_dark = match mode {
        "light" => false,
        "dark" => true,
        _ => matches!(appearance, Appearance::Dark),
    };

    let (key, default) = if use_dark {
        ("dark", DEFAULT_DARK_THEME)
    } else {
        ("light", DEFAULT_LIGHT_THEME)
    };

    Some(
        obj.get(key)
            .and_then(Value::as_str)
            .unwrap_or(default)
            .to_string(),
    )
}

fn default_theme_name(appearance: Appearance) -> &'static str {
    match appearance {
        Appearance::Dark => DEFAULT_DARK_THEME,
        Appearance::Light => DEFAULT_LIGHT_THEME,
    }
}

fn lookup_theme(name: &str, discovery: &DiscoveryInputs<'_>) -> Option<ThemeConfig> {
    if let Some(json) = family_from_extension_index(name, discovery) {
        if let Ok(config) = adapt_named_theme(json, name) {
            return Some(config);
        }
    }

    for family_json in discovery.user_families {
        if let Ok(config) = adapt_named_theme(family_json, name) {
            return Some(config);
        }
    }

    for family_json in discovery.bundled_families {
        if let Ok(config) = adapt_named_theme(family_json, name) {
            return Some(config);
        }
    }

    None
}

fn family_from_extension_index<'a>(name: &str, discovery: &DiscoveryInputs<'a>) -> Option<&'a str> {
    let index_json = discovery.extension_index?;
    let value: Value = serde_json::from_str(index_json).ok()?;
    let entry = value.get("themes")?.get(name)?;
    let extension_id = entry.get("extension")?.as_str()?;
    let path = entry.get("path")?.as_str()?;

    discovery
        .extension_families
        .iter()
        .find(|(id, rel, _)| *id == extension_id && *rel == path)
        .map(|(_, _, json)| *json)
}

fn adapt_named_theme(family_json: &str, name: &str) -> Result<ThemeConfig, AdaptError> {
    let themes = adapt_zed_family(family_json)?;
    themes
        .into_iter()
        .find(|theme| theme.name.as_ref() == name)
        .ok_or_else(|| AdaptError::ThemeNotFound(name.to_string()))
}

/// Apply a sync result to the live gpui-kit theme (after `gpui_kit::init`).
pub fn apply_sync_result(cx: &mut App, result: &SyncResult) {
    let config = Rc::new(result.config.clone());
    Theme::update(cx, |theme| {
        theme.apply_config(&config);
    });
}

/// Human-readable toast message for a missing theme name.
pub fn missing_theme_toast_message(name: &str) -> String {
    format!("Theme \"{name}\" not found. Using {FALLBACK_THEME_NAME}.")
}

/// Marker so repeated missing-theme toasts replace instead of stacking.
struct MissingThemeToast;

/// Warning toast for an unresolved settings theme name (stable id, top-right).
pub fn missing_theme_notification(name: &str) -> Notification {
    Notification::warning(missing_theme_toast_message(name))
        .id::<MissingThemeToast>()
        .placement(Anchor::TopRight)
}

/// After launch sync in `ReviewDesk::new`, skip the first activation re-sync.
///
/// Returns whether activation should call sync. Clears the skip flag when set.
pub fn should_sync_on_activation(skip_next_activation_sync: &mut bool) -> bool {
    if !*skip_next_activation_sync {
        return true;
    }
    *skip_next_activation_sync = false;
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::fallback::ONE_FAMILY_JSON;
    use gpui_kit::component::ThemeMode;

    const EXT_FAMILY: &str = r##"{
      "name": "Ext",
      "themes": [
        {
          "name": "Ext Dark",
          "appearance": "dark",
          "style": {
            "background": "#111111ff",
            "border": "#222222ff",
            "text": "#eeeeeeff",
            "editor.background": "#111111ff",
            "editor.foreground": "#eeeeeeff",
            "syntax": { "keyword": { "color": "#ff0000ff" } }
          }
        }
      ]
    }"##;

    const USER_FAMILY: &str = r##"{
      "name": "User",
      "themes": [
        {
          "name": "User Dark",
          "appearance": "dark",
          "style": {
            "background": "#010101ff",
            "border": "#020202ff",
            "text": "#fafafaff",
            "editor.background": "#010101ff",
            "editor.foreground": "#fafafaff",
            "syntax": { "keyword": { "color": "#00ff00ff" } }
          }
        }
      ]
    }"##;

    const SHARED_NAME_EXT: &str = r##"{
      "name": "SharedExt",
      "themes": [
        {
          "name": "Shared Dark",
          "appearance": "dark",
          "style": {
            "background": "#aaaaaaaa",
            "border": "#bbbbbbbb",
            "text": "#cccccccc",
            "editor.background": "#aaaaaaaa",
            "editor.foreground": "#cccccccc",
            "syntax": {}
          }
        }
      ]
    }"##;

    const SHARED_NAME_USER: &str = r##"{
      "name": "SharedUser",
      "themes": [
        {
          "name": "Shared Dark",
          "appearance": "dark",
          "style": {
            "background": "#dddddddd",
            "border": "#eeeeeeee",
            "text": "#ffffffff",
            "editor.background": "#dddddddd",
            "editor.foreground": "#ffffffff",
            "syntax": {}
          }
        }
      ]
    }"##;

    const EXT_INDEX: &str = r#"{
      "themes": {
        "Ext Dark": { "extension": "ext", "path": "themes/ext.json" },
        "Shared Dark": { "extension": "shared", "path": "themes/shared.json" }
      }
    }"#;

    fn inputs_with<'a>(
        settings: &'a str,
        appearance: Appearance,
        discovery: DiscoveryInputs<'a>,
    ) -> SyncInputs<'a> {
        SyncInputs {
            zed_present: true,
            settings_text: Some(settings),
            system_appearance: appearance,
            discovery,
        }
    }

    #[test]
    fn silent_fallback_when_zed_or_settings_absent() {
        let cases = [
            (
                "zed absent",
                SyncInputs {
                    zed_present: false,
                    settings_text: None,
                    system_appearance: Appearance::Dark,
                    discovery: DiscoveryInputs {
                        bundled_families: &[ONE_FAMILY_JSON],
                        ..DiscoveryInputs::default()
                    },
                },
            ),
            (
                "settings absent",
                SyncInputs {
                    zed_present: true,
                    settings_text: None,
                    system_appearance: Appearance::Dark,
                    discovery: DiscoveryInputs {
                        bundled_families: &[ONE_FAMILY_JSON],
                        ..DiscoveryInputs::default()
                    },
                },
            ),
            ("both absent", SyncInputs::absent_zed()),
        ];

        for (name, inputs) in cases {
            let result = sync_theme(inputs);
            assert!(result.used_fallback, "{name}");
            assert_eq!(result.theme_name, FALLBACK_THEME_NAME, "{name}");
            assert_eq!(result.toast, ToastIntent::None, "{name}");
            assert_eq!(result.config.mode, ThemeMode::Dark, "{name}");
            assert_eq!(result.config.name.as_ref(), FALLBACK_THEME_NAME, "{name}");
            assert!(
                result.config.highlight.is_some(),
                "{name}: Fallback must carry syntax highlight"
            );
            assert!(
                result.config.colors.background.is_some(),
                "{name}: Fallback must carry chrome"
            );
        }
    }

    #[test]
    fn resolve_active_theme_name_table() {
        let cases = [
            (
                "static string",
                r#"{"theme":"One Dark"}"#,
                Appearance::Light,
                Some("One Dark"),
            ),
            (
                "mode light",
                r#"{"theme":{"mode":"light","light":"One Light","dark":"One Dark"}}"#,
                Appearance::Dark,
                Some("One Light"),
            ),
            (
                "mode dark",
                r#"{"theme":{"mode":"dark","light":"One Light","dark":"Ext Dark"}}"#,
                Appearance::Light,
                Some("Ext Dark"),
            ),
            (
                "system dark",
                r#"{"theme":{"mode":"system","light":"One Light","dark":"ultraViolet"}}"#,
                Appearance::Dark,
                Some("ultraViolet"),
            ),
            (
                "system light",
                r#"{"theme":{"mode":"system","light":"One Light","dark":"One Dark"}}"#,
                Appearance::Light,
                Some("One Light"),
            ),
            (
                "dynamic defaults",
                r#"{"theme":{"mode":"system"}}"#,
                Appearance::Dark,
                Some("One Dark"),
            ),
            ("invalid json", "{not-json", Appearance::Dark, None),
            (
                "missing theme key uses default",
                r#"{"other":1}"#,
                Appearance::Dark,
                Some("One Dark"),
            ),
            (
                "jsonc comments and trailing commas",
                r#"// Zed settings often look like this
{
  "theme": {
    "mode": "system",
    "light": "One Light",
    "dark": "ultraViolet",
  },
}
"#,
                Appearance::Dark,
                Some("ultraViolet"),
            ),
        ];

        for (label, settings, appearance, expected) in cases {
            let got = resolve_active_theme_name(settings, appearance);
            assert_eq!(got.as_deref(), expected, "{label}");
        }
    }

    #[test]
    fn jsonc_settings_sync_resolves_named_theme_not_silent_fallback() {
        let settings = r#"// Review Desk fixture: valid Zed JSONC
{
  "theme": "One Dark",
}
"#;
        let result = sync_theme(inputs_with(
            settings,
            Appearance::Dark,
            DiscoveryInputs {
                bundled_families: &[ONE_FAMILY_JSON],
                ..DiscoveryInputs::default()
            },
        ));
        assert!(!result.used_fallback);
        assert_eq!(result.theme_name, "One Dark");
        assert_eq!(result.toast, ToastIntent::None);
    }

    #[test]
    fn resolution_order_extension_then_user_then_bundled() {
        let cases = [
            (
                "extension wins over user and bundled",
                r#"{"theme":"Shared Dark"}"#,
                DiscoveryInputs {
                    extension_index: Some(EXT_INDEX),
                    extension_families: &[("shared", "themes/shared.json", SHARED_NAME_EXT)],
                    user_families: &[SHARED_NAME_USER],
                    bundled_families: &[ONE_FAMILY_JSON],
                },
                "Shared Dark",
                "#aaaaaaaa",
                false,
                ToastIntent::None,
            ),
            (
                "user wins when not in extension index",
                r#"{"theme":"User Dark"}"#,
                DiscoveryInputs {
                    extension_index: Some(EXT_INDEX),
                    extension_families: &[("ext", "themes/ext.json", EXT_FAMILY)],
                    user_families: &[USER_FAMILY],
                    bundled_families: &[ONE_FAMILY_JSON],
                },
                "User Dark",
                "#010101ff",
                false,
                ToastIntent::None,
            ),
            (
                "bundled One Dark when not elsewhere",
                r#"{"theme":"One Dark"}"#,
                DiscoveryInputs {
                    extension_index: Some(r#"{"themes":{}}"#),
                    extension_families: &[],
                    user_families: &[],
                    bundled_families: &[ONE_FAMILY_JSON],
                },
                "One Dark",
                "#3b414dff",
                false,
                ToastIntent::None,
            ),
            (
                "extension theme adapts",
                r#"{"theme":"Ext Dark"}"#,
                DiscoveryInputs {
                    extension_index: Some(EXT_INDEX),
                    extension_families: &[("ext", "themes/ext.json", EXT_FAMILY)],
                    user_families: &[],
                    bundled_families: &[ONE_FAMILY_JSON],
                },
                "Ext Dark",
                "#111111ff",
                false,
                ToastIntent::None,
            ),
        ];

        for (label, settings, discovery, expect_name, expect_bg, expect_fallback, expect_toast) in
            cases
        {
            let result = sync_theme(inputs_with(settings, Appearance::Dark, discovery));
            assert_eq!(result.theme_name, expect_name, "{label}");
            assert_eq!(result.used_fallback, expect_fallback, "{label}");
            assert_eq!(result.toast, expect_toast, "{label}");
            assert_eq!(
                result.config.colors.background.as_ref().map(|c| c.as_ref()),
                Some(expect_bg),
                "{label}"
            );
        }
    }

    #[test]
    fn unresolvable_named_theme_falls_back_with_toast() {
        let result = sync_theme(inputs_with(
            r#"{"theme":"Missing Theme"}"#,
            Appearance::Dark,
            DiscoveryInputs {
                extension_index: Some(r#"{"themes":{}}"#),
                extension_families: &[],
                user_families: &[],
                bundled_families: &[ONE_FAMILY_JSON],
            },
        ));
        assert!(result.used_fallback);
        assert_eq!(result.theme_name, FALLBACK_THEME_NAME);
        assert_eq!(
            result.toast,
            ToastIntent::MissingTheme {
                name: "Missing Theme".into()
            }
        );
        assert_eq!(result.config.mode, ThemeMode::Dark);
    }

    #[test]
    fn appearance_mode_selects_light_or_dark_slot() {
        let discovery = DiscoveryInputs {
            extension_index: None,
            extension_families: &[],
            user_families: &[],
            bundled_families: &[ONE_FAMILY_JSON],
        };
        let settings = r#"{"theme":{"mode":"system","light":"One Light","dark":"One Dark"}}"#;

        let dark = sync_theme(inputs_with(settings, Appearance::Dark, discovery.clone()));
        assert!(!dark.used_fallback);
        assert_eq!(dark.theme_name, "One Dark");
        assert_eq!(dark.config.mode, ThemeMode::Dark);
        assert_eq!(dark.toast, ToastIntent::None);

        let light = sync_theme(inputs_with(settings, Appearance::Light, discovery));
        assert!(!light.used_fallback);
        assert_eq!(light.theme_name, "One Light");
        assert_eq!(light.config.mode, ThemeMode::Light);
        assert_eq!(light.toast, ToastIntent::None);
    }

    #[test]
    fn invalid_settings_json_is_silent_fallback() {
        let result = sync_theme(inputs_with(
            "{bad",
            Appearance::Dark,
            DiscoveryInputs {
                bundled_families: &[ONE_FAMILY_JSON],
                ..DiscoveryInputs::default()
            },
        ));
        assert!(result.used_fallback);
        assert_eq!(result.toast, ToastIntent::None);
    }

    #[test]
    fn missing_theme_toast_message_names_theme() {
        assert_eq!(
            missing_theme_toast_message("ultraViolet"),
            "Theme \"ultraViolet\" not found. Using One Dark."
        );
    }

    #[test]
    fn activation_sync_skips_once_after_launch_then_runs() {
        let mut skip_next = true;
        assert!(
            !should_sync_on_activation(&mut skip_next),
            "first activation after launch sync must skip"
        );
        assert!(!skip_next, "skip flag must clear after first activation");
        assert!(
            should_sync_on_activation(&mut skip_next),
            "later focus activations must sync"
        );
        assert!(
            should_sync_on_activation(&mut skip_next),
            "subsequent activations keep syncing"
        );
    }

    #[test]
    fn activation_sync_runs_when_not_skipping() {
        let mut skip_next = false;
        assert!(should_sync_on_activation(&mut skip_next));
        assert!(!skip_next);
    }
}
