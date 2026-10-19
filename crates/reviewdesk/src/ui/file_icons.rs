//! Zed default file-icon theme mapping (subset), rendered via vendored SVGs.

use gpui_kit::component::{Icon, Sizable as _};

macro_rules! zed_icon {
    ($file:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/ui/vendor/file_icons/",
            $file
        ))
    };
}

/// Folder chevron + icon for a directory row.
pub fn dir_icons(expanded: bool) -> (Icon, Icon) {
    let chevron = if expanded {
        Icon::default().data(zed_icon!("chevron_down.svg"))
    } else {
        Icon::default().data(zed_icon!("chevron_right.svg"))
    };
    let folder = if expanded {
        Icon::default().data(zed_icon!("folder_open.svg"))
    } else {
        Icon::default().data(zed_icon!("folder.svg"))
    };
    (chevron.xsmall(), folder.xsmall())
}

/// File icon by basename / extension (Zed default associations, vendored subset).
pub fn file_icon(file_name: &str) -> Icon {
    Icon::default().data(icon_svg_for_name(file_name)).xsmall()
}

fn icon_svg_for_name(file_name: &str) -> &'static [u8] {
    let lower = file_name.to_ascii_lowercase();
    if let Some(svg) = stem_icon(&lower) {
        return svg;
    }
    let suffix = lower
        .rsplit_once('.')
        .map(|(_, ext)| ext)
        .unwrap_or(lower.as_str());
    suffix_icon(suffix)
}

fn stem_icon(name: &str) -> Option<&'static [u8]> {
    match name {
        "dockerfile" | "containerfile" | ".dockerignore" => Some(zed_icon!("docker.svg")),
        _ if name.starts_with("dockerfile.") => Some(zed_icon!("docker.svg")),
        "docker-compose.yml" | "docker-compose.yaml" | "compose.yml" | "compose.yaml" => {
            Some(zed_icon!("docker.svg"))
        }
        "gemfile" | "podfile" => Some(zed_icon!("ruby.svg")),
        "cargo.toml" | "cargo.lock" => Some(zed_icon!("toml.svg")),
        ".gitignore" | ".gitattributes" | ".gitmodules" | ".gitkeep" => Some(zed_icon!("git.svg")),
        "makefile" => Some(zed_icon!("settings.svg")),
        _ => None,
    }
}

fn suffix_icon(suffix: &str) -> &'static [u8] {
    match suffix {
        "rs" => zed_icon!("rust.svg"),
        "js" | "cjs" | "mjs" => zed_icon!("javascript.svg"),
        "ts" | "cts" | "mts" => zed_icon!("typescript.svg"),
        "jsx" | "tsx" | "mjsx" | "mtsx" => zed_icon!("react.svg"),
        "py" => zed_icon!("python.svg"),
        "go" | "mod" | "work" => zed_icon!("go.svg"),
        "json" | "jsonc" => zed_icon!("code.svg"),
        "yaml" | "yml" => zed_icon!("yaml.svg"),
        "toml" => zed_icon!("toml.svg"),
        "md" | "markdown" | "mdx" | "txt" => zed_icon!("book.svg"),
        "html" | "htm" | "xml" | "hbs" => zed_icon!("html.svg"),
        "css" | "pcss" | "postcss" => zed_icon!("css.svg"),
        "scss" | "sass" => zed_icon!("sass.svg"),
        "svg" | "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "bmp" => zed_icon!("image.svg"),
        "sql" | "sqlite" | "db" | "csv" => zed_icon!("database.svg"),
        "sh" | "bash" | "zsh" | "fish" | "ps1" => zed_icon!("terminal.svg"),
        "lock" => zed_icon!("lock.svg"),
        "diff" | "patch" => zed_icon!("diff.svg"),
        "java" => zed_icon!("java.svg"),
        "kt" => zed_icon!("kotlin.svg"),
        "rb" => zed_icon!("ruby.svg"),
        "lua" => zed_icon!("lua.svg"),
        "swift" => zed_icon!("swift.svg"),
        "conf" | "ini" | "env" => zed_icon!("settings.svg"),
        _ => zed_icon!("file.svg"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_and_default_map() {
        assert_eq!(icon_svg_for_name("main.rs"), zed_icon!("rust.svg"));
        assert_eq!(icon_svg_for_name("App.tsx"), zed_icon!("react.svg"));
        assert_eq!(icon_svg_for_name("weird.xyz"), zed_icon!("file.svg"));
        assert_eq!(icon_svg_for_name("Dockerfile"), zed_icon!("docker.svg"));
        assert_eq!(icon_svg_for_name(".gitignore"), zed_icon!("git.svg"));
    }
}
