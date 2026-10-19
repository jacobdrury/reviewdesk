//! Zed default file-icon theme mapping (subset), rendered via vendored SVGs
//! tinted with language brand colors (Seti / VS Code–style) so types read at a glance.

use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::{Hsla, rgb};

macro_rules! zed_icon {
    ($file:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/ui/vendor/file_icons/",
            $file
        ))
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IconKind {
    Rust,
    JavaScript,
    TypeScript,
    React,
    Python,
    Go,
    Json,
    Yaml,
    Toml,
    Markdown,
    Html,
    Css,
    Sass,
    Image,
    Database,
    Shell,
    Lock,
    Diff,
    Java,
    Kotlin,
    Ruby,
    Lua,
    Swift,
    Settings,
    Docker,
    Git,
    Default,
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

/// Folder / chevron tint (cool blue-gray, readable on dark chrome).
pub fn dir_icon_color() -> Hsla {
    rgb(0x82_aa_ff).into()
}

/// File icon by basename / extension (Zed default associations, vendored subset).
pub fn file_icon(file_name: &str) -> Icon {
    Icon::default()
        .data(icon_svg(icon_kind(file_name)))
        .xsmall()
}

/// Brand tint for [`file_icon`], so `.rs` / `.toml` / etc. are distinguishable.
pub fn file_icon_color(file_name: &str) -> Hsla {
    kind_color(icon_kind(file_name))
}

fn icon_kind(file_name: &str) -> IconKind {
    let lower = file_name.to_ascii_lowercase();
    if let Some(kind) = stem_kind(&lower) {
        return kind;
    }
    let suffix = lower
        .rsplit_once('.')
        .map(|(_, ext)| ext)
        .unwrap_or(lower.as_str());
    suffix_kind(suffix)
}

fn stem_kind(name: &str) -> Option<IconKind> {
    match name {
        "dockerfile" | "containerfile" | ".dockerignore" => Some(IconKind::Docker),
        _ if name.starts_with("dockerfile.") => Some(IconKind::Docker),
        "docker-compose.yml" | "docker-compose.yaml" | "compose.yml" | "compose.yaml" => {
            Some(IconKind::Docker)
        }
        "gemfile" | "podfile" => Some(IconKind::Ruby),
        "cargo.toml" | "cargo.lock" => Some(IconKind::Toml),
        ".gitignore" | ".gitattributes" | ".gitmodules" | ".gitkeep" => Some(IconKind::Git),
        "makefile" => Some(IconKind::Settings),
        _ => None,
    }
}

fn suffix_kind(suffix: &str) -> IconKind {
    match suffix {
        "rs" => IconKind::Rust,
        "js" | "cjs" | "mjs" => IconKind::JavaScript,
        "ts" | "cts" | "mts" => IconKind::TypeScript,
        "jsx" | "tsx" | "mjsx" | "mtsx" => IconKind::React,
        "py" => IconKind::Python,
        "go" | "mod" | "work" => IconKind::Go,
        "json" | "jsonc" => IconKind::Json,
        "yaml" | "yml" => IconKind::Yaml,
        "toml" => IconKind::Toml,
        "md" | "markdown" | "mdx" | "txt" => IconKind::Markdown,
        "html" | "htm" | "xml" | "hbs" => IconKind::Html,
        "css" | "pcss" | "postcss" => IconKind::Css,
        "scss" | "sass" => IconKind::Sass,
        "svg" | "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "bmp" => IconKind::Image,
        "sql" | "sqlite" | "db" | "csv" => IconKind::Database,
        "sh" | "bash" | "zsh" | "fish" | "ps1" => IconKind::Shell,
        "lock" => IconKind::Lock,
        "diff" | "patch" => IconKind::Diff,
        "java" => IconKind::Java,
        "kt" => IconKind::Kotlin,
        "rb" => IconKind::Ruby,
        "lua" => IconKind::Lua,
        "swift" => IconKind::Swift,
        "conf" | "ini" | "env" => IconKind::Settings,
        _ => IconKind::Default,
    }
}

fn icon_svg(kind: IconKind) -> &'static [u8] {
    match kind {
        IconKind::Rust => zed_icon!("rust.svg"),
        IconKind::JavaScript => zed_icon!("javascript.svg"),
        IconKind::TypeScript => zed_icon!("typescript.svg"),
        IconKind::React => zed_icon!("react.svg"),
        IconKind::Python => zed_icon!("python.svg"),
        IconKind::Go => zed_icon!("go.svg"),
        IconKind::Json => zed_icon!("code.svg"),
        IconKind::Yaml => zed_icon!("yaml.svg"),
        IconKind::Toml => zed_icon!("toml.svg"),
        IconKind::Markdown => zed_icon!("book.svg"),
        IconKind::Html => zed_icon!("html.svg"),
        IconKind::Css => zed_icon!("css.svg"),
        IconKind::Sass => zed_icon!("sass.svg"),
        IconKind::Image => zed_icon!("image.svg"),
        IconKind::Database => zed_icon!("database.svg"),
        IconKind::Shell => zed_icon!("terminal.svg"),
        IconKind::Lock => zed_icon!("lock.svg"),
        IconKind::Diff => zed_icon!("diff.svg"),
        IconKind::Java => zed_icon!("java.svg"),
        IconKind::Kotlin => zed_icon!("kotlin.svg"),
        IconKind::Ruby => zed_icon!("ruby.svg"),
        IconKind::Lua => zed_icon!("lua.svg"),
        IconKind::Swift => zed_icon!("swift.svg"),
        IconKind::Settings => zed_icon!("settings.svg"),
        IconKind::Docker => zed_icon!("docker.svg"),
        IconKind::Git => zed_icon!("git.svg"),
        IconKind::Default => zed_icon!("file.svg"),
    }
}

/// Brand colors tuned for dark UI (Seti / Material-adjacent).
fn kind_color(kind: IconKind) -> Hsla {
    let hex = match kind {
        IconKind::Rust => 0xde_a5_84,
        IconKind::JavaScript => 0xcb_cb_41,
        IconKind::TypeScript => 0x51_9a_ba,
        IconKind::React => 0x61_da_fb,
        IconKind::Python => 0x51_9a_ba,
        IconKind::Go => 0x51_9a_ba,
        IconKind::Json => 0xcb_cb_41,
        IconKind::Yaml => 0xcb_17_1e,
        IconKind::Toml => 0x9c_42_21,
        IconKind::Markdown => 0x51_9a_ba,
        IconKind::Html => 0xe3_79_33,
        IconKind::Css => 0x51_9a_ba,
        IconKind::Sass => 0xf5_53_85,
        IconKind::Image => 0xa0_74_c4,
        IconKind::Database => 0xf5_53_85,
        IconKind::Shell => 0x89_e0_51,
        IconKind::Lock => 0xcc_b8_8a,
        IconKind::Diff => 0xe6_b4_50,
        IconKind::Java => 0xb0_72_19,
        IconKind::Kotlin => 0xa9_7b_ff,
        IconKind::Ruby => 0xcc_34_2d,
        IconKind::Lua => 0x51_9a_ba,
        IconKind::Swift => 0xf0_51_38,
        IconKind::Settings => 0x6d_80_86,
        IconKind::Docker => 0x24_96_ed,
        IconKind::Git => 0xf0_50_32,
        IconKind::Default => 0x6d_80_86,
    };
    rgb(hex).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_and_default_map() {
        assert_eq!(icon_kind("main.rs"), IconKind::Rust);
        assert_eq!(icon_kind("App.tsx"), IconKind::React);
        assert_eq!(icon_kind("weird.xyz"), IconKind::Default);
        assert_eq!(icon_kind("Dockerfile"), IconKind::Docker);
        assert_eq!(icon_kind(".gitignore"), IconKind::Git);
        assert_eq!(icon_svg(IconKind::Rust), zed_icon!("rust.svg"));
    }

    #[test]
    fn brand_colors_differ_for_common_types() {
        assert_ne!(file_icon_color("a.rs"), file_icon_color("a.toml"));
        assert_ne!(file_icon_color("a.rs"), file_icon_color("a.md"));
        assert_ne!(file_icon_color("a.rs"), dir_icon_color());
    }
}
