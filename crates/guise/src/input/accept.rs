//! Extension filtering shared by [`FileInput`](super::FileInput) and
//! [`Dropzone`](super::Dropzone). Pure logic.

use std::path::{Path, PathBuf};

/// Normalize an accept entry: strip a leading dot, lowercase ("`.PNG`" → "png").
pub(crate) fn normalize_ext(entry: &str) -> String {
    entry.trim().trim_start_matches('.').to_ascii_lowercase()
}

/// Whether `path` passes the accept list. An empty list accepts everything;
/// extensions compare case-insensitively; extension-less paths only pass an
/// empty list.
pub(crate) fn path_accepted(path: &Path, accept: &[String]) -> bool {
    if accept.is_empty() {
        return true;
    }
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| {
            let ext = ext.to_ascii_lowercase();
            accept.iter().any(|a| *a == ext)
        })
}

/// Keep only the paths passing the accept list.
pub(crate) fn filter_paths(paths: Vec<PathBuf>, accept: &[String]) -> Vec<PathBuf> {
    paths
        .into_iter()
        .filter(|p| path_accepted(p, accept))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accept(entries: &[&str]) -> Vec<String> {
        entries.iter().map(|e| normalize_ext(e)).collect()
    }

    #[test]
    fn normalizes_entries() {
        assert_eq!(normalize_ext(".PNG"), "png");
        assert_eq!(normalize_ext("jpg"), "jpg");
        assert_eq!(normalize_ext(" .Tar "), "tar");
    }

    #[test]
    fn empty_list_accepts_everything() {
        assert!(path_accepted(Path::new("a.png"), &[]));
        assert!(path_accepted(Path::new("no_extension"), &[]));
    }

    #[test]
    fn filters_by_extension_case_insensitively() {
        let list = accept(&[".png", "JPG"]);
        assert!(path_accepted(Path::new("photo.PNG"), &list));
        assert!(path_accepted(Path::new("photo.jpg"), &list));
        assert!(!path_accepted(Path::new("notes.txt"), &list));
        assert!(!path_accepted(Path::new("archive"), &list));
    }

    #[test]
    fn filter_paths_keeps_matches_in_order() {
        let list = accept(&["rs"]);
        let paths = vec![
            PathBuf::from("a.rs"),
            PathBuf::from("b.txt"),
            PathBuf::from("c.rs"),
        ];
        assert_eq!(
            filter_paths(paths, &list),
            vec![PathBuf::from("a.rs"), PathBuf::from("c.rs")]
        );
    }
}
