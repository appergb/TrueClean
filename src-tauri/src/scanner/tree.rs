//! Shape the raw walk output into the IPC model: trim each node's children to
//! the largest N (recording how many were dropped), sort by size descending,
//! and compute the per-category breakdown with percentages.

use crate::model::{CategoryBreakdown, CategoryEntry, DirNode, ScanOptions};
use crate::scanner::walker::{category_from_index, RawNode, CATEGORY_COUNT};

/// Recursively convert a [`RawNode`] into a [`DirNode`], keeping only the
/// `top_children` largest children per node (sorted by size descending) and
/// recording the number omitted in `truncated_children`.
pub(crate) fn build_dir_node(raw: RawNode, options: &ScanOptions) -> DirNode {
    let RawNode {
        name,
        path,
        size_bytes,
        file_count,
        category,
        is_dir,
        mut children,
    } = raw;

    // Sort largest-first so truncation always drops the smallest branches.
    children.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    let top = options.top_children;
    let truncated_children = children.len().saturating_sub(top) as u32;
    if children.len() > top {
        children.truncate(top);
    }

    let kept: Vec<DirNode> = children
        .into_iter()
        .map(|child| build_dir_node(child, options))
        .collect();

    DirNode {
        name,
        path,
        size_bytes,
        file_count,
        category,
        is_dir,
        children: kept,
        truncated_children,
    }
}

/// Build the category breakdown from the accumulated `(size, count)` table.
/// Entries are sorted by size descending and zero-size categories are dropped.
/// `percent` is each category's share of `total_bytes` (0.0–100.0).
pub(crate) fn build_breakdown(totals: &[(u64, u64); CATEGORY_COUNT]) -> CategoryBreakdown {
    let total_bytes: u64 = totals.iter().map(|(size, _)| *size).sum();
    let scanned_files: u64 = totals.iter().map(|(_, count)| *count).sum();

    let mut entries: Vec<CategoryEntry> = totals
        .iter()
        .enumerate()
        .filter(|(_, (size, count))| *size > 0 || *count > 0)
        .map(|(i, (size, count))| CategoryEntry {
            category: category_from_index(i),
            size_bytes: *size,
            file_count: *count,
            percent: if total_bytes > 0 {
                (*size as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            },
        })
        .collect();

    entries.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    CategoryBreakdown {
        entries,
        total_bytes,
        scanned_files,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Category;
    use crate::scanner::walker::category_index;

    fn raw(name: &str, size: u64, children: Vec<RawNode>) -> RawNode {
        RawNode {
            name: name.into(),
            path: format!("/{name}"),
            size_bytes: size,
            file_count: 1,
            category: Category::Other,
            is_dir: true,
            children,
        }
    }

    #[test]
    fn truncates_and_sorts_children() {
        let opts = ScanOptions {
            top_children: 2,
            ..ScanOptions::default()
        };
        let root = raw(
            "root",
            60,
            vec![
                raw("a", 10, vec![]),
                raw("b", 30, vec![]),
                raw("c", 20, vec![]),
            ],
        );
        let node = build_dir_node(root, &opts);
        assert_eq!(node.children.len(), 2);
        assert_eq!(node.truncated_children, 1);
        // Sorted descending: b(30), c(20); a(10) dropped.
        assert_eq!(node.children[0].name, "b");
        assert_eq!(node.children[1].name, "c");
    }

    #[test]
    fn breakdown_percentages_sum_to_total() {
        let mut totals = [(0u64, 0u64); CATEGORY_COUNT];
        totals[category_index(Category::Media)] = (75, 3);
        totals[category_index(Category::Caches)] = (25, 1);

        let bd = build_breakdown(&totals);
        assert_eq!(bd.total_bytes, 100);
        assert_eq!(bd.scanned_files, 4);
        assert_eq!(bd.entries.len(), 2);
        // Largest first.
        assert_eq!(bd.entries[0].category, Category::Media);
        assert!((bd.entries[0].percent - 75.0).abs() < 1e-9);
        assert!((bd.entries[1].percent - 25.0).abs() < 1e-9);
    }
}
