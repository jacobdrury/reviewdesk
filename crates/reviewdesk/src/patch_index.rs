use std::ops::Range;

use gpui_kit::SharedString;
use gpui_kit::component::diff::DiffFile;
use reviewdesk_core::{Patch, ProviderError};

/// Patch bytes plus per-file spans for lazy Diff materialization.
#[derive(Clone, Debug)]
pub struct PatchIndex {
    patch: Patch,
    entries: Vec<PatchFileEntry>,
}

#[derive(Clone, Debug)]
struct PatchFileEntry {
    path: SharedString,
    range: Range<usize>,
}

impl PatchIndex {
    pub fn paths(&self) -> impl ExactSizeIterator<Item = &SharedString> {
        self.entries.iter().map(|entry| &entry.path)
    }

    pub fn materialize(&self, path: &str) -> Result<DiffFile, ProviderError> {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.path.as_str() == path)
            .ok_or_else(|| ProviderError::other(format!("unknown patch file: {path}")))?;
        let slice = &self.patch.as_str()[entry.range.clone()];
        let files = DiffFile::parse(slice).map_err(parse_error)?;
        files
            .into_iter()
            .find(|file| file.path().as_str() == path)
            .ok_or_else(|| ProviderError::other(format!("patch slice did not contain {path}")))
    }
}

/// Builds a path index from Patch bytes without materializing every file's Diff rows.
pub fn index_patch(patch: Patch) -> Result<PatchIndex, ProviderError> {
    let text = patch.as_str();
    if text.trim().is_empty() {
        return Ok(PatchIndex {
            patch,
            entries: Vec::new(),
        });
    }

    let mut entries = Vec::new();
    let mut file_start: Option<usize> = None;
    let mut pending_path: Option<SharedString> = None;
    let mut bytes = 0usize;

    for line in text.split_inclusive('\n') {
        let line_start = bytes;
        bytes += line.len();
        let trimmed = line.strip_suffix('\n').unwrap_or(line);
        let content = trimmed.strip_suffix('\r').unwrap_or(trimmed);

        if content.starts_with("diff --git ") {
            if let Some(start) = file_start.take() {
                push_entry(&mut entries, pending_path.take(), start..line_start);
            }
            file_start = Some(line_start);
            pending_path = git_display_path(content.strip_prefix("diff --git ").unwrap_or(content))
                .map(SharedString::from);
            continue;
        }

        if content == "-- " || content == "--" {
            if let Some(start) = file_start.take() {
                push_entry(&mut entries, pending_path.take(), start..line_start);
            }
            continue;
        }

        if let Some(raw) = content.strip_prefix("--- ") {
            if file_start.is_none() {
                file_start = Some(line_start);
            }
            if pending_path.is_none() {
                pending_path = header_display_path(raw).map(SharedString::from);
            }
            continue;
        }

        if let Some(raw) = content.strip_prefix("+++ ") {
            if let Some(path) = header_display_path(raw) {
                pending_path = Some(SharedString::from(path));
            }
            continue;
        }

        if let Some(raw) = content.strip_prefix("rename to ") {
            pending_path = Some(SharedString::from(decode_path_token(raw)));
            continue;
        }

        if content.starts_with("deleted file mode ") {
            // Keep the original-side path already captured from ---/+++ or diff --git.
        }
    }

    if let Some(start) = file_start {
        push_entry(&mut entries, pending_path.take(), start..bytes);
    }

    if entries.is_empty() {
        return Err(ProviderError::other("expected a unified or Git diff"));
    }

    Ok(PatchIndex { patch, entries })
}

fn push_entry(entries: &mut Vec<PatchFileEntry>, path: Option<SharedString>, range: Range<usize>) {
    let Some(path) = path else {
        return;
    };
    if range.is_empty() {
        return;
    }
    entries.push(PatchFileEntry { path, range });
}

fn parse_error(error: gpui_kit::component::diff::DiffParseError) -> ProviderError {
    ProviderError::other(format!(
        "failed to parse patch at line {}: {}",
        error.line(),
        error.message()
    ))
}

fn git_display_path(header: &str) -> Option<String> {
    let (old, new) = split_git_paths(header)?;
    Some(preferred_display_path(&old, &new))
}

fn split_git_paths(value: &str) -> Option<(String, String)> {
    if value.starts_with('"') {
        let end = quoted_end(value)?;
        let old = decode_path_token(&value[..end]);
        let new = decode_path_token(value[end..].trim_start());
        return Some((old, new));
    }
    if value.ends_with('"') {
        let start = value
            .match_indices(" \"")
            .map(|(ix, _)| ix + 1)
            .find(|ix| quoted_end(&value[*ix..]) == Some(value.len() - ix))?;
        let old = value[..start - 1].to_owned();
        let new = decode_path_token(&value[start..]);
        return Some((old, new));
    }
    for (old_prefix, new_prefix) in GIT_PREFIXES {
        if let Some((old, new)) = value.match_indices(' ').find_map(|(ix, _)| {
            let (left, right) = value.split_at(ix);
            let right = right.strip_prefix(' ')?;
            let path = left.strip_prefix(old_prefix)?;
            if path.is_empty() {
                return None;
            }
            right.strip_prefix(new_prefix).filter(|p| *p == path)?;
            Some((left.to_owned(), right.to_owned()))
        }) {
            return Some((old, new));
        }
        if let Some(ix) = value.rfind(&format!(" {new_prefix}"))
            && value.starts_with(old_prefix)
        {
            let old = value[..ix].to_owned();
            let new = value[ix + 1..].to_owned();
            return Some((old, new));
        }
    }
    let (old, new) = value.split_once(' ')?;
    Some((old.to_owned(), new.to_owned()))
}

const GIT_PREFIXES: [(&str, &str); 7] = [
    ("a/", "b/"),
    ("i/", "w/"),
    ("c/", "w/"),
    ("c/", "i/"),
    ("o/", "w/"),
    ("1/", "2/"),
    ("", ""),
];

fn preferred_display_path(old: &str, new: &str) -> String {
    let old_path = strip_git_prefix(old);
    let new_path = strip_git_prefix(new);
    if !new_path.is_empty() {
        new_path
    } else {
        old_path
    }
}

fn strip_git_prefix(path: &str) -> String {
    for (old_prefix, new_prefix) in GIT_PREFIXES {
        if let Some(stripped) = path.strip_prefix(old_prefix)
            && !old_prefix.is_empty()
        {
            return stripped.to_owned();
        }
        if let Some(stripped) = path.strip_prefix(new_prefix)
            && !new_prefix.is_empty()
        {
            return stripped.to_owned();
        }
    }
    path.to_owned()
}

fn header_display_path(raw: &str) -> Option<String> {
    let token = raw.split('\t').next()?.trim();
    if token == "/dev/null" {
        return None;
    }
    Some(strip_git_prefix(&decode_path_token(token)))
}

fn decode_path_token(value: &str) -> String {
    let value = value.trim_matches('"');
    let mut out = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let Some(next) = chars.next() else {
                out.push('\\');
                break;
            };
            if next.is_ascii_digit() {
                let mut octal = next.to_string();
                for _ in 0..2 {
                    if chars.peek().is_some_and(|c| c.is_ascii_digit()) {
                        octal.push(chars.next().unwrap());
                    }
                }
                if let Ok(byte) = u8::from_str_radix(&octal, 8) {
                    out.push(byte as char);
                    continue;
                }
            }
            out.push(next);
        } else {
            out.push(ch);
        }
    }
    out
}

fn quoted_end(value: &str) -> Option<usize> {
    if !value.starts_with('"') {
        return None;
    }
    let mut escaped = false;
    for (ix, ch) in value.char_indices().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            return Some(ix + ch.len_utf8());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const MULTI_FILE: &str = "diff --git a/new b/new\nnew file mode 100644\n--- /dev/null\n+++ b/new\n@@ -0,0 +1 @@\n+added\ndiff --git a/old b/old\ndeleted file mode 100644\n--- a/old\n+++ /dev/null\n@@ -1 +0,0 @@\n-deleted\ndiff --git a/from b/to\nsimilarity index 100%\nrename from from\nrename to to\ndiff --git a/pic b/pic\nBinary files a/pic and b/pic differ\n";

    #[test]
    fn index_lists_every_path_without_eager_rows() {
        let index = index_patch(Patch::new(MULTI_FILE)).unwrap();
        let paths: Vec<_> = index.paths().map(|p| p.as_str()).collect();
        assert_eq!(paths, ["new", "old", "to", "pic"]);
    }

    #[test]
    fn materialize_matches_full_parse_per_file() {
        let patch = Patch::new(MULTI_FILE);
        let eager = DiffFile::parse(MULTI_FILE).unwrap();
        let index = index_patch(patch).unwrap();

        for file in &eager {
            let path = file.path().as_str();
            let lazy = index.materialize(path).unwrap();
            assert_eq!(lazy.path().as_str(), path);
            assert_eq!(lazy.additions(), file.additions());
            assert_eq!(lazy.deletions(), file.deletions());
            assert_eq!(lazy.status(), file.status());
        }
    }

    #[test]
    fn first_file_materializes_before_other_files_parse() {
        let mut broken = MULTI_FILE.to_owned();
        broken.push_str("diff --git a/bad b/bad\n--- a/bad\n+++ b/bad\n@@ not a hunk @@\n");
        let index = index_patch(Patch::new(&broken)).unwrap();
        assert_eq!(index.paths().count(), 5);
        assert!(index.materialize("new").is_ok());
        assert!(index.materialize("bad").is_err());
    }
}
