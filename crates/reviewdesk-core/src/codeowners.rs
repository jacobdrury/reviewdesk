//! Client-side CODEOWNERS parsing and path matching (GitHub does not expose matches on the PR).

use crate::metadata::CodeownersRule;

/// Parse a CODEOWNERS file into `(pattern, owners)` rules. Comments and blank lines are skipped.
pub fn parse_codeowners(source: &str) -> Vec<(String, Vec<String>)> {
    let mut rules = Vec::new();
    for line in source.lines() {
        let line = line.split('#').next().unwrap_or(line).trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(pattern) = parts.next() else {
            continue;
        };
        let owners: Vec<String> = parts.map(str::to_string).collect();
        if owners.is_empty() {
            continue;
        }
        rules.push((pattern.to_string(), owners));
    }
    rules
}

/// Match `changed_paths` against parsed rules; last matching rule wins per GitHub semantics.
pub fn codeowners_breakdown(
    rules: &[(String, Vec<String>)],
    changed_paths: &[String],
) -> Vec<CodeownersRule> {
    let mut by_pattern: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();

    for path in changed_paths {
        let Some((pattern, owners)) = matching_rule(rules, path) else {
            continue;
        };
        if let Some(entry) = by_pattern
            .iter_mut()
            .find(|(p, o, _)| p == pattern && o == owners)
        {
            if !entry.2.iter().any(|p| p == path) {
                entry.2.push(path.clone());
            }
            continue;
        }
        by_pattern.push((pattern.clone(), owners.clone(), vec![path.clone()]));
    }

    by_pattern
        .into_iter()
        .map(|(pattern, owners, matched_paths)| CodeownersRule {
            pattern,
            owners,
            matched_paths,
        })
        .collect()
}

fn matching_rule<'a>(
    rules: &'a [(String, Vec<String>)],
    path: &str,
) -> Option<&'a (String, Vec<String>)> {
    rules
        .iter()
        .rev()
        .find(|(pattern, _)| pattern_matches(pattern, path))
}

fn pattern_matches(pattern: &str, path: &str) -> bool {
    let path = path.trim_start_matches("./");
    if pattern.contains('*') {
        return glob_match(pattern, path);
    }
    if pattern.ends_with('/') {
        let prefix = pattern.trim_end_matches('/');
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    path == pattern || path.ends_with(&format!("/{pattern}"))
}

fn glob_match(pattern: &str, path: &str) -> bool {
    if let Some(stripped) = pattern.strip_prefix("**/") {
        return path.ends_with(stripped.trim_start_matches('/'))
            || path.contains(stripped.trim_start_matches('/'));
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return path.ends_with(suffix);
    }
    path == pattern
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_skips_comments_and_requires_owners() {
        let rules = parse_codeowners("# comment\n\n/docs/ @docs-team\n*.rs @rust\nempty-pattern\n");
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].0, "/docs/");
        assert_eq!(rules[1].0, "*.rs");
    }

    #[test]
    fn last_rule_wins_for_overlapping_paths() {
        let rules = parse_codeowners("* @everyone\n*.rs @rust\n");
        let breakdown = codeowners_breakdown(&rules, &["src/main.rs".into()]);
        assert_eq!(breakdown.len(), 1);
        assert_eq!(breakdown[0].pattern, "*.rs");
        assert_eq!(breakdown[0].owners, vec!["@rust"]);
        assert_eq!(breakdown[0].matched_paths, vec!["src/main.rs"]);
    }
}
