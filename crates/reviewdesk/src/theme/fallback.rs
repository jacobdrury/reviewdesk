use gpui_kit::component::ThemeConfig;

use super::adapter::{adapt_zed_family, AdaptError};

/// Vendored Zed One theme family (`assets/themes/one/one.json`).
pub const ONE_FAMILY_JSON: &str = include_str!("vendor/one.json");

/// Registry / product name for the dark Fallback theme.
pub const FALLBACK_THEME_NAME: &str = "One Dark";

/// Adapt the vendored One family and return the dark Fallback `ThemeConfig`.
pub fn fallback_one_dark() -> Result<ThemeConfig, AdaptError> {
    let themes = adapt_zed_family(ONE_FAMILY_JSON)?;
    themes
        .into_iter()
        .find(|theme| theme.name.as_ref() == FALLBACK_THEME_NAME)
        .ok_or(AdaptError::ThemeNotFound(FALLBACK_THEME_NAME.to_string()))
}
