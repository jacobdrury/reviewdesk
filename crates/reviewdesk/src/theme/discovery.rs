//! macOS Zed path discovery for theme sync (#23).
//!
//! Reads settings, extension index, user themes, and installed extension
//! family files into owned buffers that feed the injectable [`SyncInputs`] seam.

use std::fs;
use std::path::{Path, PathBuf};

use super::fallback::ONE_FAMILY_JSON;
use super::sync::{sync_theme, Appearance, DiscoveryInputs, SyncInputs, SyncResult};

/// Owned discovery materials loaded from the Zed macOS layout.
#[derive(Debug, Clone)]
pub struct DiscoveredThemes {
    pub zed_present: bool,
    pub settings_text: Option<String>,
    pub extension_index: Option<String>,
    /// `(extension_id, relative_path, family_json)`
    pub extension_families: Vec<(String, String, String)>,
    pub user_families: Vec<String>,
}

impl DiscoveredThemes {
    /// Resolve a theme using these discovery materials and system appearance.
    pub fn sync(&self, system_appearance: Appearance) -> SyncResult {
        let extension_families: Vec<(&str, &str, &str)> = self
            .extension_families
            .iter()
            .map(|(id, path, json)| (id.as_str(), path.as_str(), json.as_str()))
            .collect();
        let user_families: Vec<&str> = self.user_families.iter().map(String::as_str).collect();
        sync_theme(SyncInputs {
            zed_present: self.zed_present,
            settings_text: self.settings_text.as_deref(),
            system_appearance,
            discovery: DiscoveryInputs {
                extension_index: self.extension_index.as_deref(),
                extension_families: &extension_families,
                user_families: &user_families,
                bundled_families: &[ONE_FAMILY_JSON],
            },
        })
    }
}

/// Load Zed theme discovery inputs from the standard macOS paths.
pub fn discover_macos() -> DiscoveredThemes {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    discover_from_roots(
        home.join(".config/zed"),
        home.join("Library/Application Support/Zed"),
    )
}

fn discover_from_roots(config_dir: PathBuf, data_dir: PathBuf) -> DiscoveredThemes {
    let settings_path = config_dir.join("settings.json");
    let themes_dir = config_dir.join("themes");
    let extensions_dir = data_dir.join("extensions");
    let index_path = extensions_dir.join("index.json");
    let installed_dir = extensions_dir.join("installed");

    let zed_present = config_dir.is_dir() || data_dir.is_dir();
    let settings_text = read_to_string_if_exists(&settings_path);
    let extension_index = read_to_string_if_exists(&index_path);

    let extension_families = load_extension_families(extension_index.as_deref(), &installed_dir);
    let user_families = load_user_families(&themes_dir);

    DiscoveredThemes {
        zed_present,
        settings_text,
        extension_index,
        extension_families,
        user_families,
    }
}

fn load_extension_families(
    index_json: Option<&str>,
    installed_dir: &Path,
) -> Vec<(String, String, String)> {
    let Some(index_json) = index_json else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(index_json) else {
        return Vec::new();
    };
    let Some(themes) = value.get("themes").and_then(|v| v.as_object()) else {
        return Vec::new();
    };

    let mut loaded = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for entry in themes.values() {
        let Some(extension_id) = entry.get("extension").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(rel_path) = entry.get("path").and_then(|v| v.as_str()) else {
            continue;
        };
        let key = (extension_id.to_string(), rel_path.to_string());
        if !seen.insert(key.clone()) {
            continue;
        }
        let file_path = installed_dir.join(extension_id).join(rel_path);
        let Some(family_json) = read_to_string_if_exists(&file_path) else {
            continue;
        };
        loaded.push((key.0, key.1, family_json));
    }
    loaded
}

fn load_user_families(themes_dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(themes_dir) else {
        return Vec::new();
    };
    let mut families = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Some(contents) = read_to_string_if_exists(&path) {
            families.push(contents);
        }
    }
    families
}

fn read_to_string_if_exists(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn discover_from_roots_reads_settings_index_and_families() {
        let root = std::env::temp_dir().join(format!(
            "reviewdesk-theme-discovery-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let config = root.join("config");
        let data = root.join("data");
        fs::create_dir_all(config.join("themes")).unwrap();
        fs::create_dir_all(data.join("extensions/installed/ext/themes")).unwrap();

        fs::write(config.join("settings.json"), r#"{"theme":"Ext Dark"}"#).unwrap();
        fs::write(
            data.join("extensions/index.json"),
            r#"{
              "themes": {
                "Ext Dark": {"extension":"ext","path":"themes/ext.json"}
              }
            }"#,
        )
        .unwrap();
        fs::write(
            data.join("extensions/installed/ext/themes/ext.json"),
            r#"{"name":"Ext","themes":[{"name":"Ext Dark","appearance":"dark","style":{}}]}"#,
        )
        .unwrap();
        fs::write(
            config.join("themes/user.json"),
            r#"{"name":"User","themes":[{"name":"User Dark","appearance":"dark","style":{}}]}"#,
        )
        .unwrap();

        let discovered = discover_from_roots(config, data);
        assert!(discovered.zed_present);
        assert!(discovered
            .settings_text
            .as_ref()
            .unwrap()
            .contains("Ext Dark"));
        assert_eq!(discovered.extension_families.len(), 1);
        assert_eq!(discovered.extension_families[0].0, "ext");
        assert_eq!(discovered.user_families.len(), 1);

        let _ = fs::remove_dir_all(&root);
    }
}
