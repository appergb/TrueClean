//! Shape the raw walk output into the IPC model.
//!
//! 注意：自增量裁剪重构后，`RawNode` 的 children 已在 `walker::visit_dir`
//! 阶段排序+裁剪到 `top_children` 个最大子节点。`build_dir_node` 只需做
//! 简单的结构转换（RawNode → DirNode），无需再次排序裁剪，避免了全量树
//! 的二次遍历和双倍内存峰值。

use crate::model::{CategoryBreakdown, CategoryEntry, DirNode, ScanOptions};
use crate::scanner::walker::{category_from_index, RawNode, CATEGORY_COUNT};

/// Recursively convert a [`RawNode`] into a [`DirNode`].
///
/// 裁剪（排序 + truncate 到 top_children）已在 `walker::visit_dir` 阶段
/// 增量完成，此处只需结构转换。`truncated_children` 直接透传。
pub(crate) fn build_dir_node(raw: RawNode, _options: &ScanOptions) -> DirNode {
    let RawNode {
        name,
        path,
        size_bytes,
        file_count,
        category,
        is_dir,
        children,
        truncated_children,
    } = raw;

    let kept: Vec<DirNode> = children
        .into_iter()
        .map(|child| build_dir_node(child, _options))
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

    entries.sort_by_key(|a| std::cmp::Reverse(a.size_bytes));

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
            truncated_children: 0,
        }
    }

    #[test]
    fn truncates_and_sorts_children() {
        // 注意：增量裁剪重构后，build_dir_node 不再排序裁剪——裁剪已在
        // walker::visit_dir 阶段完成。此测试验证 build_dir_node 正确透传
        // truncated_children 字段（由 walk 阶段设置）。
        let opts = ScanOptions {
            top_children: 2,
            ..ScanOptions::default()
        };
        // 模拟 walk 阶段已裁剪：保留 b(30) 和 c(20)，丢弃 a(10)。
        let mut root = raw("root", 60, vec![raw("b", 30, vec![]), raw("c", 20, vec![])]);
        // 手动设置 truncated_children（walk 阶段会做）。
        root.truncated_children = 1;
        let node = build_dir_node(root, &opts);
        assert_eq!(node.children.len(), 2);
        assert_eq!(node.truncated_children, 1);
        // children 顺序由 walk 阶段排序决定，build_dir_node 保持原序。
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
