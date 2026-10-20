//! Clean-up suggestions: every cache, editor folder and ignored entry in a scan, largest first.

use std::path::{Path, PathBuf};

use crate::scan::{Kind, Node};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub path: PathBuf,
    /// Top-level folder under the scan root it belongs to (the root's name for loose entries).
    pub project: String,
    pub kind: Kind,
    pub label: &'static str,
    /// How to get it back; only caches have one.
    pub restore: Option<&'static str>,
    pub bytes: u64,
    pub files: u64,
}

/// Everything in `root` that isn't a project file or git history, largest first.
pub fn suggestions(root: &Node) -> Vec<Suggestion> {
    let mut out = Vec::new();
    collect(root, &root.path, &root.name, &mut out);
    out.sort_by_key(|s| std::cmp::Reverse(s.bytes));
    out
}

fn collect(node: &Node, root: &Path, root_name: &str, out: &mut Vec<Suggestion>) {
    for child in &node.children {
        let (label, restore) = match child.kind {
            Kind::Cache => (
                child.detector.map_or("Cache", |d| d.label),
                child.detector.map(|d| d.restore),
            ),
            Kind::Editor => ("Editor / OS metadata", None),
            Kind::Ignored => ("Ignored by .gitignore", None),
            Kind::Source => {
                collect(child, root, root_name, out);
                continue;
            }
            Kind::Git => continue,
        };
        out.push(Suggestion {
            path: child.path.clone(),
            project: project(&child.path, root, root_name),
            kind: child.kind,
            label,
            restore,
            bytes: child.bytes,
            files: child.files,
        });
    }
}

fn project(path: &Path, root: &Path, root_name: &str) -> String {
    let mut rel = path.strip_prefix(root).unwrap_or(path).components();
    match (rel.next(), rel.next()) {
        (Some(first), Some(_)) => first.as_os_str().to_string_lossy().into_owned(),
        _ => root_name.to_owned(),
    }
}

/// Caches are checked by default; the rest waits for review.
pub fn default_selected(s: &Suggestion) -> bool {
    s.kind == Kind::Cache
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::DETECTORS;

    fn node(path: &str, kind: Kind, bytes: u64, children: Vec<Node>) -> Node {
        let mut by_kind = [0; 5];
        by_kind[kind.index()] = bytes;
        Node {
            name: path.rsplit('/').next().unwrap().into(),
            path: PathBuf::from(path),
            kind,
            is_dir: true,
            bytes,
            files: 1,
            by_kind,
            children,
            detector: (kind == Kind::Cache).then(|| &DETECTORS[0]),
        }
    }

    fn tree() -> Node {
        node(
            "/ws",
            Kind::Source,
            6000,
            vec![
                node(
                    "/ws/A",
                    Kind::Source,
                    5900,
                    vec![
                        node("/ws/A/node_modules", Kind::Cache, 5000, vec![]),
                        node("/ws/A/notes.log", Kind::Ignored, 100, vec![]),
                        node("/ws/A/src", Kind::Source, 800, vec![]),
                    ],
                ),
                node(
                    "/ws/B",
                    Kind::Source,
                    10,
                    vec![node("/ws/B/.idea", Kind::Editor, 10, vec![])],
                ),
                node("/ws/.DS_Store", Kind::Editor, 4, vec![]),
            ],
        )
    }

    #[test]
    fn lists_non_source_nodes_sorted_by_size() {
        let s = suggestions(&tree());
        let sizes: Vec<u64> = s.iter().map(|s| s.bytes).collect();
        assert_eq!(sizes, [5000, 100, 10, 4]);
        let projects: Vec<&str> = s.iter().map(|s| s.project.as_str()).collect();
        assert_eq!(projects, ["A", "A", "B", "ws"]);
        assert_eq!(s[0].label, "Node packages");
        assert_eq!(s[0].restore, Some("npm/pnpm/yarn install"));
        assert_eq!(s[1].label, "Ignored by .gitignore");
        assert_eq!(s[1].restore, None);
        assert_eq!(s[2].label, "Editor / OS metadata");
    }

    #[test]
    fn only_caches_are_preselected() {
        let s = suggestions(&tree());
        let picked: Vec<bool> = s.iter().map(default_selected).collect();
        assert_eq!(picked, [true, false, false, false]);
    }
}
