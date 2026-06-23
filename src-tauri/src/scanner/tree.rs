//! Shape the raw walk output into the IPC model.
//!
//! 注意：自增量裁剪重构后，`RawNode` 的 children 已在 `walker::visit_dir`
//! 阶段排序+裁剪到 `top_children` 个最大子节点。`build_dir_node` 只需做
//! 简单的结构转换（RawNode → DirNode），无需再次排序裁剪，避免了全量树
//! 的二次遍历和双倍内存峰值。

use crate::model::{CategoryBreakdown, CategoryEntry, DirNode, ScanOptions};
use crate::scanner::walker::{category_from_index, RawNode, CATEGORY_COUNT};

/// 输出树的全局节点上限。约 270 B/节点 → ~8 MB JSON，IPC 传输与前端
/// `JSON.parse` 都在亚秒级。
const MAX_TREE_NODES: usize = 30_000;
/// 输出树的最大深度。系统盘真实深度可达 31 层，对可视化无意义且会让
/// 「top_children/节点 × 无限深度」的树膨胀到上百 MB JSON → webview 卡死。
const MAX_TREE_DEPTH: usize = 14;

/// Convert the raw walk tree into a **bounded** [`DirNode`] tree for IPC.
///
/// walker 已在 `visit_dir` 阶段把每个节点的子节点增量裁剪到 `top_children`
/// 个（按 size 降序），但**深度无限**——系统盘里 node_modules / Library 这类
/// 又深又宽的结构会让输出树达到数十万节点 / 上百 MB JSON，序列化经 IPC 后
/// webview `JSON.parse` 直接卡死/OOM（“扫完却不出结果”）。此处再做一层
/// 深度 + 全局节点数的二次有界化：节点预算按子节点 size 占比分配，让有限的
/// 节点花在“占空间”的分支上。被裁分支的体积已计入父 `size_bytes`，仅结构
/// 不展开，并累加到 `truncated_children` 告知前端“还有 N 项”。
pub(crate) fn build_dir_node(raw: RawNode, _options: &ScanOptions) -> DirNode {
    build_bounded(raw, 0, MAX_TREE_NODES)
}

/// 递归构建有界子树。`budget` = 本子树（含自身及后代）允许保留的最大节点数。
fn build_bounded(raw: RawNode, depth: usize, budget: usize) -> DirNode {
    let RawNode {
        name,
        path,
        size_bytes,
        file_count,
        category,
        is_dir,
        mut children,
        mut truncated_children,
    } = raw;

    // 本节点占 1 个名额；剩余给子树。
    let sub_budget = budget.saturating_sub(1);

    let kept: Vec<DirNode> = if depth >= MAX_TREE_DEPTH || sub_budget == 0 || children.is_empty() {
        // 深度上限 / 预算耗尽：不展开子节点（体积已计入 size_bytes）。
        truncated_children = truncated_children.saturating_add(children.len() as u32);
        Vec::new()
    } else {
        // 预算装不下全部子节点时，保留最大的 sub_budget 个（每个至少 1 名额）。
        if children.len() > sub_budget {
            children.sort_by_key(|c| std::cmp::Reverse(c.size_bytes));
            truncated_children =
                truncated_children.saturating_add((children.len() - sub_budget) as u32);
            children.truncate(sub_budget);
        }
        // 剩余可分配名额按子节点 size 占比下发。
        let total: u128 = children.iter().map(|c| c.size_bytes as u128).sum::<u128>().max(1);
        let extra = (sub_budget - children.len()) as u128;
        children
            .into_iter()
            .map(|child| {
                let share = (extra * child.size_bytes as u128 / total) as usize;
                build_bounded(child, depth + 1, 1 + share)
            })
            .collect()
    };

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

    fn count_nodes(n: &DirNode) -> usize {
        1 + n.children.iter().map(count_nodes).sum::<usize>()
    }
    fn tree_depth(n: &DirNode) -> usize {
        1 + n.children.iter().map(tree_depth).max().unwrap_or(0)
    }

    /// 深链（40 层）→ 输出深度被截到 MAX_TREE_DEPTH(+1)，否则系统盘 31 层
    /// 深树会撑爆序列化。
    #[test]
    fn bounds_output_depth() {
        let mut node = raw("leaf", 1, vec![]);
        for i in 0..40 {
            node = raw(&format!("d{i}"), 100, vec![node]);
        }
        let out = build_dir_node(node, &ScanOptions::default());
        // tree_depth 以 root=1 计层；深度检查用 0 起的 depth，故最多 MAX_TREE_DEPTH+1 层。
        let d = tree_depth(&out);
        assert!(d <= MAX_TREE_DEPTH + 1, "输出深度 {d} 应 ≤ {}", MAX_TREE_DEPTH + 1);
    }

    /// 又宽又深的树（8^6 ≈ 26 万节点）→ 输出节点数被全局预算压到
    /// MAX_TREE_NODES 以内（系统盘 54 万节点 / 146 MB JSON 的修复）。
    #[test]
    fn bounds_output_node_count() {
        fn build(depth: usize, breadth: usize, sz: u64) -> RawNode {
            if depth == 0 {
                return raw("f", sz, vec![]);
            }
            let kids = (0..breadth)
                .map(|i| build(depth - 1, breadth, sz + i as u64 + 1))
                .collect();
            raw("d", sz * breadth as u64, kids)
        }
        let big = build(6, 8, 1);
        let out = build_dir_node(big, &ScanOptions::default());
        let n = count_nodes(&out);
        assert!(n <= MAX_TREE_NODES, "输出节点数 {n} 应 ≤ {MAX_TREE_NODES}");
        assert!(tree_depth(&out) <= MAX_TREE_DEPTH + 1);
        assert!(n > 1000, "有界树过于稀疏：仅 {n} 节点");
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
