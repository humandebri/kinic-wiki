// Where: crates/wiki_store/src/glob_match.rs
// What: Minimal shell-style glob matching for FS-first node paths.
// Why: VFS glob must work without adding a new dependency or changing persistence contracts.
const MAX_GLOB_PATTERN_LEN: usize = 512;
const MAX_GLOB_STEPS: usize = 50_000;

pub(crate) fn validate_pattern(pattern: &str) -> Result<(), String> {
    if pattern.len() > MAX_GLOB_PATTERN_LEN {
        return Err(format!(
            "pattern is too long: {} > {MAX_GLOB_PATTERN_LEN}",
            pattern.len()
        ));
    }
    Ok(())
}

pub(crate) fn matches_path(pattern: &str, path: &str) -> Result<bool, String> {
    validate_pattern(pattern)?;
    let pattern_segments = split_segments(pattern);
    let path_segments = split_segments(path);
    let mut steps = 0usize;
    matches_segments(&pattern_segments, &path_segments, &mut steps)
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

fn matches_segments(
    patterns: &[&str],
    segments: &[&str],
    steps: &mut usize,
) -> Result<bool, String> {
    *steps += 1;
    if *steps > MAX_GLOB_STEPS {
        return Err("glob pattern is too complex".to_string());
    }
    if patterns.is_empty() {
        return Ok(segments.is_empty());
    }
    if patterns[0] == "**" {
        for index in 0..=segments.len() {
            if matches_segments(&patterns[1..], &segments[index..], steps)? {
                return Ok(true);
            }
        }
        return Ok(false);
    }
    if segments.is_empty() {
        return Ok(false);
    }
    if !matches_segment(patterns[0], segments[0]) {
        return Ok(false);
    }
    matches_segments(&patterns[1..], &segments[1..], steps)
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
        assert!(matches_path("*.md", "foo.md").expect("match should succeed"));
        assert!(!matches_path("*.md", "nested/foo.md").expect("match should succeed"));
        assert!(matches_path("**/*.md", "nested/foo.md").expect("match should succeed"));
        assert!(matches_path("**/*.md", "deep/nested/foo.md").expect("match should succeed"));
        assert!(matches_path("n?sted/*.md", "nested/foo.md").expect("match should succeed"));
        assert!(
            !matches_path("n?sted/*.md", "nested/deeper/foo.md").expect("match should succeed")
        );
    }

    #[test]
    fn rejects_inputs_that_exceed_limits() {
        let long_pattern = "*".repeat(513);
        assert!(matches_path(&long_pattern, "foo.md").is_err());
    }

    #[test]
    fn allows_long_paths_to_match_without_failing_the_whole_query() {
        let long_path = format!("{}/note.md", "nested".repeat(1024));
        assert!(matches_path("**/*.md", &long_path).expect("long path should still match"));
    }
}
