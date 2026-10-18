use std::rc::Rc;

use gpui_kit::component::{Theme, ThemeConfig, ThemeMode};
use gpui_kit::App;

use super::fallback::{fallback_one_dark, FALLBACK_THEME_NAME};

/// System appearance stub for #23 dynamic `{mode, light, dark}` settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Light used when #23 wires dynamic settings.
pub enum Appearance {
    Light,
    Dark,
}

/// Toast policy for a sync attempt. #22 only exercises the silent path.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // MissingTheme reserved for #23.
pub enum ToastIntent {
    None,
    /// Reserved for #23 (named theme missing from discovery).
    MissingTheme { name: String },
}

/// Injectable inputs for the Zed theme sync seam.
#[derive(Debug, Clone)]
pub struct SyncInputs<'a> {
    /// False when Zed is not installed / not discoverable.
    pub zed_present: bool,
    /// Zed `settings.json` text when readable.
    pub settings_text: Option<&'a str>,
    /// Theme family JSON when a resolver (#23) already loaded it.
    pub family_json: Option<&'a str>,
    /// System appearance for dynamic theme name selection (#23).
    pub system_appearance: Appearance,
}

impl SyncInputs<'static> {
    /// Inputs that always take the silent Fallback path (no Zed / no settings).
    pub fn absent_zed() -> Self {
        Self {
            zed_present: false,
            settings_text: None,
            family_json: None,
            system_appearance: Appearance::Dark,
        }
    }
}

/// Result of one sync attempt, ready for the shell to apply.
#[derive(Debug, Clone)]
#[allow(dead_code)] // theme_name / toast / used_fallback read by #23 + tests.
pub struct SyncResult {
    pub theme_name: String,
    pub config: ThemeConfig,
    pub used_fallback: bool,
    pub toast: ToastIntent,
}

/// Resolve a theme from injectable discovery inputs.
///
/// #22: Zed missing or settings absent → silent Fallback `One Dark` (dark only).
/// #23 will extend name resolution, family lookup, and missing-name toast.
pub fn sync_theme(inputs: SyncInputs<'_>) -> SyncResult {
    if !inputs.zed_present || inputs.settings_text.is_none() {
        return silent_fallback();
    }

    // Minimal seam for #23: when a family is injected, adapt its dark theme
    // matching the Fallback name, or the first dark entry. Full settings parse
    // and discovery order land in #23.
    if let Some(family_json) = inputs.family_json {
        if let Ok(themes) = super::adapt_zed_family(family_json) {
            if let Some(config) = pick_dark_theme(themes, inputs.system_appearance) {
                return SyncResult {
                    theme_name: config.name.to_string(),
                    config,
                    used_fallback: false,
                    toast: ToastIntent::None,
                };
            }
        }
    }

    silent_fallback()
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

fn pick_dark_theme(themes: Vec<ThemeConfig>, appearance: Appearance) -> Option<ThemeConfig> {
    let want_dark = matches!(appearance, Appearance::Dark);
    themes.into_iter().find(|theme| {
        if want_dark {
            theme.mode == ThemeMode::Dark
        } else {
            theme.mode == ThemeMode::Light
        }
    })
}

/// Apply a sync result to the live gpui-kit theme (after `gpui_kit::init`).
pub fn apply_sync_result(cx: &mut App, result: &SyncResult) {
    let config = Rc::new(result.config.clone());
    Theme::update(cx, |theme| {
        theme.apply_config(&config);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::fallback::ONE_FAMILY_JSON;
    use gpui_kit::component::ThemeMode;

    #[test]
    fn silent_fallback_when_zed_or_settings_absent() {
        let cases = [
            (
                "zed absent",
                SyncInputs {
                    zed_present: false,
                    settings_text: None,
                    family_json: None,
                    system_appearance: Appearance::Dark,
                },
            ),
            (
                "settings absent",
                SyncInputs {
                    zed_present: true,
                    settings_text: None,
                    family_json: Some(ONE_FAMILY_JSON),
                    system_appearance: Appearance::Dark,
                },
            ),
            (
                "both absent",
                SyncInputs::absent_zed(),
            ),
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
    fn without_zed_only_dark_fallback_activates() {
        let result = sync_theme(SyncInputs::absent_zed());
        assert!(result.used_fallback);
        assert_eq!(result.config.mode, ThemeMode::Dark);
        assert_ne!(result.config.mode, ThemeMode::Light);
    }

    #[test]
    fn injected_family_with_settings_adapts_without_toast() {
        let result = sync_theme(SyncInputs {
            zed_present: true,
            settings_text: Some(r#"{"theme":"One Dark"}"#),
            family_json: Some(ONE_FAMILY_JSON),
            system_appearance: Appearance::Dark,
        });
        assert!(!result.used_fallback);
        assert_eq!(result.toast, ToastIntent::None);
        assert_eq!(result.config.mode, ThemeMode::Dark);
        assert_eq!(result.theme_name, FALLBACK_THEME_NAME);
    }
}
