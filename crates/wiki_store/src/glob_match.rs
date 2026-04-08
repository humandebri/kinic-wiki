// Where: crates/wiki_store/src/glob_match.rs
// What: Minimal shell-style glob matching for FS-first node paths.
// Why: VFS glob must work without adding a new dependency or changing persistence contracts.
pub(crate) fn matches_path(pattern: &str, path: &str) -> bool {
    let pattern_segments = split_segments(pattern);
    let path_segments = split_segments(path);
    matches_segments(&pattern_segments, &path_segments)
}

fn split_segments(input: &str) -> Vec<&str> {
    if input.is_empty() {
        return Vec::new();
    }
    input
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn matches_segments(patterns: &[&str], segments: &[&str]) -> bool {
    if patterns.is_empty() {
        return segments.is_empty();
    }
    if patterns[0] == "**" {
        return (0..=segments.len())
            .any(|index| matches_segments(&patterns[1..], &segments[index..]));
    }
    if segments.is_empty() {
        return false;
    }
    if !matches_segment(patterns[0], segments[0]) {
        return false;
    }
    matches_segments(&patterns[1..], &segments[1..])
}

fn matches_segment(pattern: &str, segment: &str) -> bool {
    let pattern_chars = pattern.chars().collect::<Vec<_>>();
    let segment_chars = segment.chars().collect::<Vec<_>>();
    matches_chars(&pattern_chars, &segment_chars)
}

fn matches_chars(pattern: &[char], segment: &[char]) -> bool {
    if pattern.is_empty() {
        return segment.is_empty();
    }
    match pattern[0] {
        '*' => (0..=segment.len()).any(|index| matches_chars(&pattern[1..], &segment[index..])),
        '?' => {
            if segment.is_empty() {
                false
            } else {
                matches_chars(&pattern[1..], &segment[1..])
            }
        }
        other => {
            if segment.first().copied() != Some(other) {
                false
            } else {
                matches_chars(&pattern[1..], &segment[1..])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::matches_path;

    #[test]
    fn matches_single_segment_and_nested_patterns() {
        assert!(matches_path("*.md", "foo.md"));
        assert!(!matches_path("*.md", "nested/foo.md"));
        assert!(matches_path("**/*.md", "nested/foo.md"));
        assert!(matches_path("**/*.md", "deep/nested/foo.md"));
        assert!(matches_path("n?sted/*.md", "nested/foo.md"));
        assert!(!matches_path("n?sted/*.md", "nested/deeper/foo.md"));
    }
}
