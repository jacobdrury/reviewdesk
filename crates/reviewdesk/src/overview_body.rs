//! Split a PR description into markdown runs and HTML `<details>` blocks,
//! then compose a single Overview markdown document for virtualized scrolling.

pub const DETAILS_FENCE_LANG: &str = "reviewdesk-details";

/// Bodies at or above this nested character count are not inlined on first
/// paint when the Provider marks `<details open>`, so Renovate-sized release
/// notes stay out of the composed markdown until the reviewer toggles open.
pub const HEAVY_DETAILS_BODY_MIN_CHARS: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodySegment {
    Markdown(String),
    Details(DetailsBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailsBlock {
    pub id: usize,
    pub summary: String,
    pub default_open: bool,
    pub segments: Vec<BodySegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailsFence {
    pub id: usize,
    pub open: bool,
    pub default_open: bool,
    pub summary: String,
}

/// Parse `source` into markdown segments and nested `<details>` blocks.
///
/// Ids are assigned in document order so the UI can keep toggle state stable
/// across re-renders of the same description.
pub fn parse_body_segments(source: &str) -> Vec<BodySegment> {
    let mut next_id = 0;
    parse_segments(source, &mut next_id)
}

/// Build the full Overview markdown, including title/meta and details fences.
pub fn compose_overview_markdown(
    title: &str,
    author: &str,
    is_draft: bool,
    web_url: &str,
    labels: &str,
    reviewers: &str,
    body: &str,
    segments: &[BodySegment],
    is_open: &dyn Fn(&DetailsBlock) -> bool,
) -> String {
    let mut out = String::new();
    out.push_str("# ");
    out.push_str(title.trim());
    out.push_str("\n\n");
    out.push_str(author.trim());
    out.push_str(" · ");
    out.push_str(if is_draft { "Draft" } else { "Ready" });
    out.push_str(" · ");
    out.push_str(web_url.trim());
    out.push_str("\n\n");
    out.push_str("Labels: ");
    out.push_str(labels);
    out.push_str("\n\n");
    out.push_str("Reviewers: ");
    out.push_str(reviewers);
    out.push_str("\n\n");
    if body.trim().is_empty() {
        out.push_str("No description.");
        return out;
    }
    out.push_str(&compose_body_segments(segments, is_open));
    out
}

/// Collect ids of details blocks that compose as open, in ascending order.
pub fn open_details_ids(
    segments: &[BodySegment],
    is_open: &dyn Fn(&DetailsBlock) -> bool,
) -> Vec<usize> {
    let mut ids = Vec::new();
    collect_open_details_ids(segments, is_open, &mut ids);
    ids.sort_unstable();
    ids
}

fn collect_open_details_ids(
    segments: &[BodySegment],
    is_open: &dyn Fn(&DetailsBlock) -> bool,
    ids: &mut Vec<usize>,
) {
    for segment in segments {
        let BodySegment::Details(block) = segment else {
            continue;
        };
        if is_open(block) {
            ids.push(block.id);
        }
        collect_open_details_ids(&block.segments, is_open, ids);
    }
}

/// Whether a details block should compose as open (inline body after the fence).
///
/// `user_override` is `Some` when the reviewer has toggled this block in the UI.
/// Provider `<details open>` with a heavy nested body stays closed until toggled.
pub fn details_effective_open(block: &DetailsBlock, user_override: Option<bool>) -> bool {
    if let Some(open) = user_override {
        return open;
    }
    if block.default_open && details_body_char_count(block) >= HEAVY_DETAILS_BODY_MIN_CHARS {
        return false;
    }
    block.default_open
}

fn details_body_char_count(block: &DetailsBlock) -> usize {
    block
        .segments
        .iter()
        .map(|segment| match segment {
            BodySegment::Markdown(text) => text.len(),
            BodySegment::Details(nested) => details_body_char_count(nested),
        })
        .sum()
}

/// Compose body segments. Open details emit a toggle fence, then their body as
/// normal markdown so TextView can virtualize those blocks while scrolling.
pub fn compose_body_segments(
    segments: &[BodySegment],
    is_open: &dyn Fn(&DetailsBlock) -> bool,
) -> String {
    let mut out = String::new();
    for segment in segments {
        match segment {
            BodySegment::Markdown(markdown) => {
                let trimmed = markdown.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if !out.is_empty() {
                    out.push_str("\n\n");
                }
                out.push_str(trimmed);
            }
            BodySegment::Details(block) => {
                let open = is_open(block);
                if !out.is_empty() {
                    out.push_str("\n\n");
                }
                out.push_str(&emit_details_fence(block, open));
                if !open {
                    continue;
                }
                let nested = compose_body_segments(&block.segments, is_open);
                if nested.is_empty() {
                    continue;
                }
                out.push_str("\n\n");
                out.push_str(&nested);
            }
        }
    }
    out
}

pub fn parse_details_fence(
    lang: Option<&str>,
    meta: Option<&str>,
    value: &str,
) -> Option<DetailsFence> {
    if lang != Some(DETAILS_FENCE_LANG) {
        return None;
    }
    let meta = meta?.trim();
    let mut parts = meta.split_whitespace();
    let id = parts.next()?.parse().ok()?;
    let open = match parts.next()? {
        "open" => true,
        "closed" => false,
        _ => return None,
    };
    let default_open = match parts.next()? {
        "1" | "true" | "open" => true,
        "0" | "false" | "closed" => false,
        _ => return None,
    };
    let summary = value.lines().next().unwrap_or("").trim();
    let summary = if summary.is_empty() {
        "Details".to_string()
    } else {
        summary.to_string()
    };
    Some(DetailsFence {
        id,
        open,
        default_open,
        summary,
    })
}

fn emit_details_fence(block: &DetailsBlock, open: bool) -> String {
    let summary = if block.summary.is_empty() {
        "Details"
    } else {
        block.summary.as_str()
    };
    let summary = summary.replace('\n', " ");
    let state = if open { "open" } else { "closed" };
    let default_open = if block.default_open { "1" } else { "0" };
    // 4-backtick fence so summaries with triple backticks cannot break out.
    format!("````{DETAILS_FENCE_LANG} {} {state} {default_open}\n{summary}\n````", block.id)
}

fn parse_segments(source: &str, next_id: &mut usize) -> Vec<BodySegment> {
    let mut segments = Vec::new();
    let mut rest = source;

    while let Some(open_rel) = find_open_tag(rest, "details") {
        let before = &rest[..open_rel];
        push_markdown(&mut segments, before);

        let after_open = &rest[open_rel..];
        let Some(tag_end_rel) = after_open.find('>') else {
            push_markdown(&mut segments, rest);
            return segments;
        };
        let open_tag = &after_open[..=tag_end_rel];
        let default_open = open_tag_has_flag(open_tag, "open");
        let after_tag = &after_open[tag_end_rel + 1..];

        let Some(close_rel) = find_matching_close(after_tag, "details") else {
            push_markdown(&mut segments, rest);
            return segments;
        };

        let inner = &after_tag[..close_rel];
        let (summary, body) = split_summary(inner);
        let id = *next_id;
        *next_id += 1;
        segments.push(BodySegment::Details(DetailsBlock {
            id,
            summary: plain_text(&summary),
            default_open,
            segments: parse_segments(body.trim(), next_id),
        }));

        let Some(end) = find_close_tag_end(after_tag, close_rel, "details") else {
            return segments;
        };
        rest = &after_tag[end..];
    }

    push_markdown(&mut segments, rest);
    segments
}

fn push_markdown(segments: &mut Vec<BodySegment>, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    segments.push(BodySegment::Markdown(text.to_string()));
}

fn find_open_tag(haystack: &str, tag: &str) -> Option<usize> {
    let bytes = haystack.as_bytes();
    let tag_bytes = tag.as_bytes();
    let mut i = 0;
    while i + tag_bytes.len() + 1 < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let name_start = i + 1;
        let name_end = name_start + tag_bytes.len();
        if !eq_ignore_ascii_case(&bytes[name_start..name_end], tag_bytes) {
            i += 1;
            continue;
        }
        let boundary = bytes[name_end];
        if boundary == b'>' || boundary.is_ascii_whitespace() || boundary == b'/' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_matching_close(haystack: &str, tag: &str) -> Option<usize> {
    let bytes = haystack.as_bytes();
    let open_pat = format!("<{tag}");
    let close_pat = format!("</{tag}");
    let open_bytes = open_pat.as_bytes();
    let close_bytes = close_pat.as_bytes();
    let mut depth = 1usize;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        if i + close_bytes.len() <= bytes.len()
            && eq_ignore_ascii_case(&bytes[i..i + close_bytes.len()], close_bytes)
        {
            let after = i + close_bytes.len();
            if after < bytes.len() {
                let boundary = bytes[after];
                if boundary != b'>' && !boundary.is_ascii_whitespace() {
                    i += 1;
                    continue;
                }
            }
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
            i += close_bytes.len();
            continue;
        }
        if i + open_bytes.len() <= bytes.len()
            && eq_ignore_ascii_case(&bytes[i..i + open_bytes.len()], open_bytes)
        {
            let after = i + open_bytes.len();
            if after < bytes.len() {
                let boundary = bytes[after];
                if boundary == b'>' || boundary.is_ascii_whitespace() || boundary == b'/' {
                    depth += 1;
                }
            }
        }
        i += 1;
    }
    None
}

fn find_close_tag_end(haystack: &str, close_at: usize, tag: &str) -> Option<usize> {
    let after = &haystack[close_at..];
    let close_prefix = format!("</{tag}");
    if after.len() < close_prefix.len() {
        return None;
    }
    if !eq_ignore_ascii_case(
        &after.as_bytes()[..close_prefix.len()],
        close_prefix.as_bytes(),
    ) {
        return None;
    }
    let gt = after.find('>')?;
    Some(close_at + gt + 1)
}

fn split_summary(inner: &str) -> (String, &str) {
    let Some(summary_at) = find_open_tag(inner, "summary") else {
        return (String::new(), inner);
    };
    let after_open = &inner[summary_at..];
    let Some(tag_end) = after_open.find('>') else {
        return (String::new(), inner);
    };
    let after_tag = &after_open[tag_end + 1..];
    let Some(close_at) = find_matching_close(after_tag, "summary") else {
        return (String::new(), inner);
    };
    let summary = &after_tag[..close_at];
    let Some(close_end) = find_close_tag_end(after_tag, close_at, "summary") else {
        return (summary.to_string(), "");
    };
    let body = &after_tag[close_end..];
    let body = body.strip_prefix('\n').unwrap_or(body);
    let body = body.strip_prefix('\r').unwrap_or(body);
    let body = body.strip_prefix('\n').unwrap_or(body);
    (summary.to_string(), body)
}

fn open_tag_has_flag(open_tag: &str, flag: &str) -> bool {
    let lower = open_tag.to_ascii_lowercase();
    let flag = flag.to_ascii_lowercase();
    for part in lower.split(|c: char| c.is_ascii_whitespace() || c == '/' || c == '>') {
        if part == flag || part.starts_with(&format!("{flag}=")) {
            return true;
        }
    }
    false
}

fn plain_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    decode_basic_entities(out.trim())
}

fn decode_basic_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

fn eq_ignore_ascii_case(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b.iter())
            .all(|(l, r)| l.to_ascii_lowercase() == r.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_markdown_is_one_segment() {
        let segments = parse_body_segments("## Hello\n\nWorld");
        assert_eq!(
            segments,
            vec![BodySegment::Markdown("## Hello\n\nWorld".into())]
        );
    }

    #[test]
    fn extracts_details_with_summary_and_body() {
        let source = "### Release Notes\n\n<details>\n<summary>pkg (image)</summary>\n\n### v1\n\n- fix\n</details>\n";
        let segments = parse_body_segments(source);
        assert_eq!(segments.len(), 2);
        assert!(matches!(&segments[0], BodySegment::Markdown(text) if text.contains("Release Notes")));
        let BodySegment::Details(details) = &segments[1] else {
            panic!("expected details");
        };
        assert_eq!(details.id, 0);
        assert_eq!(details.summary, "pkg (image)");
        assert!(!details.default_open);
        assert_eq!(
            details.segments,
            vec![BodySegment::Markdown("### v1\n\n- fix".into())]
        );
    }

    #[test]
    fn respects_open_attribute_and_nesting() {
        let source = r#"<details open>
<summary>outer</summary>
before
<details>
<summary>inner</summary>
nested
</details>
after
</details>"#;
        let segments = parse_body_segments(source);
        let BodySegment::Details(outer) = &segments[0] else {
            panic!("expected outer details");
        };
        assert!(outer.default_open);
        assert_eq!(outer.summary, "outer");
        assert_eq!(outer.segments.len(), 3);
        assert!(matches!(&outer.segments[0], BodySegment::Markdown(_)));
        let BodySegment::Details(inner) = &outer.segments[1] else {
            panic!("expected inner details");
        };
        assert_eq!(inner.id, 1);
        assert_eq!(inner.summary, "inner");
        assert!(!inner.default_open);
        assert!(matches!(&outer.segments[2], BodySegment::Markdown(_)));
    }

    #[test]
    fn strips_tags_from_summary() {
        let source = "<details><summary><code>pkg</code> name</summary>body</details>";
        let segments = parse_body_segments(source);
        let BodySegment::Details(details) = &segments[0] else {
            panic!("expected details");
        };
        assert_eq!(details.summary, "pkg name");
    }

    #[test]
    fn compose_emits_fence_and_inlines_open_body() {
        let segments = parse_body_segments(
            "intro\n\n<details>\n<summary>closed</summary>\nhidden\n</details>\n\n<details open>\n<summary>open one</summary>\nvisible\n</details>\n",
        );
        let open = |block: &DetailsBlock| block.default_open;
        let composed = compose_body_segments(&segments, &open);
        assert!(composed.contains("intro"));
        assert!(composed.contains("````reviewdesk-details 0 closed 0\nclosed\n````"));
        assert!(composed.contains("````reviewdesk-details 1 open 1\nopen one\n````"));
        assert!(composed.contains("visible"));
        assert!(!composed.contains("hidden"));

        let fence = parse_details_fence(
            Some(DETAILS_FENCE_LANG),
            Some("1 open 1"),
            "open one",
        )
        .unwrap();
        assert_eq!(
            fence,
            DetailsFence {
                id: 1,
                open: true,
                default_open: true,
                summary: "open one".into(),
            }
        );
    }

    #[test]
    fn compose_caps_provider_open_when_body_is_heavy() {
        let heavy_body = "x".repeat(super::HEAVY_DETAILS_BODY_MIN_CHARS + 1);
        let source = format!(
            "<details open>\n<summary>Renovate release notes</summary>\n\n{heavy_body}\n</details>"
        );
        let segments = parse_body_segments(&source);
        let composed = compose_body_segments(&segments, &|block| {
            super::details_effective_open(block, None)
        });
        assert!(composed.contains("Renovate release notes"));
        assert!(composed.contains("````reviewdesk-details 0 closed 1"));
        assert!(!composed.contains(&heavy_body));
    }

    #[test]
    fn compose_inlines_provider_open_when_body_is_light() {
        let source = "<details open>\n<summary>small</summary>\n\nshort body\n</details>";
        let segments = parse_body_segments(source);
        let composed = compose_body_segments(&segments, &|block| {
            super::details_effective_open(block, None)
        });
        assert!(composed.contains("````reviewdesk-details 0 open 1\nsmall\n````"));
        assert!(composed.contains("short body"));
    }

    #[test]
    fn open_details_ids_sorted_and_nested() {
        let segments = parse_body_segments(
            "<details open>\n<summary>outer</summary>\n<details>\n<summary>inner closed</summary>\nx\n</details>\n</details>\n\n<details open>\n<summary>second</summary>\ny\n</details>",
        );
        let ids = super::open_details_ids(&segments, &|block| block.default_open);
        assert_eq!(ids, vec![0, 2]);
    }

    #[test]
    fn compose_inlines_heavy_body_after_reviewer_opens() {
        let heavy_body = "y".repeat(super::HEAVY_DETAILS_BODY_MIN_CHARS + 1);
        let source = format!(
            "<details open>\n<summary>notes</summary>\n\n{heavy_body}\n</details>"
        );
        let segments = parse_body_segments(&source);
        let composed = compose_body_segments(&segments, &|block| {
            super::details_effective_open(block, Some(true))
        });
        assert!(composed.contains("````reviewdesk-details 0 open 1"));
        assert!(composed.contains(&heavy_body));
    }
}
