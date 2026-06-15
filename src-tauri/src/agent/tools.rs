//! Agent tool definitions (neutral JSON schema) and the dispatch layer that
//! runs each tool against the cleaning / scanner subsystems.
//!
//! Results are trimmed into compact, LLM-friendly JSON: sizes are plain byte
//! counts, lists are truncated to [`LIST_CAP`] items with the full count noted,
//! and `scan_directory` returns a category breakdown plus a summary of the
//! largest top-level children rather than the whole tree.

use crate::error::{AppError, AppResult};
use crate::model::ScanOptions;
use crate::state::AppState;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

/// Maximum number of items returned to the LLM per list. Keeps prompts compact.
const LIST_CAP: usize = 30;

/// Neutral tool schemas. Each provider adapter rewrites these into its own
/// request shape (see `providers::*`).
pub fn tool_specs() -> Vec<Value> {
    vec![
        json!({
            "name": "list_volumes",
            "description": "列出所有磁盘卷及其总容量、已用、可用空间。用于了解整体磁盘健康状况。",
            "input_schema": { "type": "object", "properties": {}, "required": [] }
        }),
        json!({
            "name": "scan_directory",
            "description": "扫描指定目录，返回按类别(缓存/日志/媒体/文档等)的占比分解，以及顶层最大的若干子项摘要。不返回整棵目录树。用于定位空间都被什么占用。",
            "input_schema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "要扫描的绝对路径目录" },
                    "topN": { "type": "integer", "description": "返回的顶层最大子项数量，默认 15", "minimum": 1, "maximum": 50 }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "scan_junk",
            "description": "扫描系统中可清理的垃圾：用户/系统/应用缓存、日志、临时文件、浏览器缓存、开发缓存、回收站等，按组返回每组体积与是否推荐清理。",
            "input_schema": { "type": "object", "properties": {}, "required": [] }
        }),
        json!({
            "name": "find_large_old_files",
            "description": "在指定目录下查找体积大且长期未修改的文件，用于发现可能可以归档或删除的大文件(属于需用户确认类别)。",
            "input_schema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "要搜索的绝对路径目录" },
                    "minSizeMb": { "type": "number", "description": "最小文件大小(MB)，默认 100" },
                    "olderThanDays": { "type": "integer", "description": "至少多少天未修改，默认 180" }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "find_duplicates",
            "description": "在指定目录下按内容哈希查找重复文件组，返回每组的重复文件与可回收的浪费空间。",
            "input_schema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "要搜索的绝对路径目录" },
                    "minSizeMb": { "type": "number", "description": "参与去重的最小文件大小(MB)，默认 1" }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "list_applications",
            "description": "列出已安装的应用程序及其体积、版本、最近使用时间，用于卸载建议。卸载属于需用户确认操作。",
            "input_schema": { "type": "object", "properties": {}, "required": [] }
        }),
        json!({
            "name": "list_startup_items",
            "description": "列出开机自启动项(登录项/启动代理/服务等)及其启用状态，用于优化开机速度。",
            "input_schema": { "type": "object", "properties": {}, "required": [] }
        }),
        json!({
            "name": "clean_paths",
            "description": "【危险操作】删除指定路径列表。默认移入回收站(toTrash=true)。执行前必须已获得用户明确确认。返回清理报告(删除数量、释放空间、失败项)。",
            "input_schema": {
                "type": "object",
                "properties": {
                    "paths": { "type": "array", "items": { "type": "string" }, "description": "要删除的绝对路径列表" },
                    "toTrash": { "type": "boolean", "description": "true=移入回收站(可恢复，推荐)；false=永久删除(不可恢复)。默认 true" }
                },
                "required": ["paths"]
            }
        }),
        json!({
            "name": "empty_trash",
            "description": "【危险操作】清空系统回收站，永久删除其中内容。执行前必须已获得用户明确确认。返回清理报告。",
            "input_schema": { "type": "object", "properties": {}, "required": [] }
        }),
    ]
}

const MB: u64 = 1024 * 1024;

/// Execute a tool by name and return compact JSON for the model.
pub fn dispatch(name: &str, args: &Value, state: &AppState) -> AppResult<Value> {
    match name {
        "list_volumes" => list_volumes(),
        "scan_directory" => scan_directory(args),
        "scan_junk" => scan_junk(),
        "find_large_old_files" => find_large_old_files(args),
        "find_duplicates" => find_duplicates(args),
        "list_applications" => list_applications(),
        "list_startup_items" => list_startup_items(),
        "clean_paths" => clean_paths(args, state),
        "empty_trash" => empty_trash(),
        other => Err(AppError::Agent(format!("未知工具: {other}"))),
    }
}

// ---------------------------------------------------------------------------
// Argument helpers
// ---------------------------------------------------------------------------

fn require_path(args: &Value) -> AppResult<PathBuf> {
    let p = args
        .get("path")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| AppError::Agent("缺少必填参数 path".into()))?;
    Ok(PathBuf::from(p))
}

fn opt_f64(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(Value::as_f64)
}

fn opt_u64(args: &Value, key: &str) -> Option<u64> {
    args.get(key).and_then(Value::as_u64)
}

/// Truncate a list, returning the kept slice plus the total count so the model
/// knows how much was omitted.
fn cap<T>(items: &[T]) -> (&[T], usize) {
    let total = items.len();
    let kept = items.len().min(LIST_CAP);
    (&items[..kept], total)
}

// ---------------------------------------------------------------------------
// Tool implementations
// ---------------------------------------------------------------------------

/// Lightweight volume listing via sysinfo (independent of the scan subsystem).
fn list_volumes() -> AppResult<Value> {
    use sysinfo::Disks;
    let disks = Disks::new_with_refreshed_list();
    let volumes: Vec<Value> = disks
        .iter()
        .map(|d| {
            let total = d.total_space();
            let available = d.available_space();
            json!({
                "name": d.name().to_string_lossy(),
                "mountPoint": d.mount_point().to_string_lossy(),
                "totalBytes": total,
                "availableBytes": available,
                "usedBytes": total.saturating_sub(available),
                "fileSystem": d.file_system().to_string_lossy(),
                "isRemovable": d.is_removable(),
            })
        })
        .collect();
    Ok(json!({ "volumes": volumes, "count": volumes.len() }))
}

fn scan_directory(args: &Value) -> AppResult<Value> {
    let root = require_path(args)?;
    if !root.exists() {
        return Err(AppError::InvalidPath(root.display().to_string()));
    }
    let top_n = opt_u64(args, "topN").unwrap_or(15).clamp(1, 50) as usize;

    let options = ScanOptions {
        top_children: top_n,
        ..ScanOptions::default()
    };
    let cancel = AtomicBool::new(false);
    let result = crate::scanner::engine::scan_tree(&root, &options, &cancel, &|_p| {})?;

    // Category breakdown — compact form.
    let breakdown: Vec<Value> = result
        .breakdown
        .entries
        .iter()
        .map(|e| {
            json!({
                "category": e.category.label(),
                "sizeBytes": e.size_bytes,
                "fileCount": e.file_count,
                "percent": (e.percent * 10.0).round() / 10.0,
            })
        })
        .collect();

    // Largest top-level children (the tree's immediate children) as a summary.
    let (children, total_children) = cap(&result.tree.children);
    let top_items: Vec<Value> = children
        .iter()
        .map(|c| {
            json!({
                "name": c.name,
                "path": c.path,
                "sizeBytes": c.size_bytes,
                "isDir": c.is_dir,
                "category": c.category.label(),
            })
        })
        .collect();

    Ok(json!({
        "root": result.root,
        "totalBytes": result.breakdown.total_bytes,
        "scannedFiles": result.breakdown.scanned_files,
        "breakdown": breakdown,
        "topItems": top_items,
        "topItemsShown": top_items.len(),
        "topItemsTotal": total_children,
    }))
}

fn scan_junk() -> AppResult<Value> {
    let cancel = AtomicBool::new(false);
    let groups = crate::cleaning::junk::scan_junk(&cancel)?;

    let total_bytes: u64 = groups.iter().map(|g| g.total_bytes).sum();
    let recommended_bytes: u64 = groups
        .iter()
        .filter(|g| g.recommended)
        .map(|g| g.total_bytes)
        .sum();

    let out: Vec<Value> = groups
        .iter()
        .map(|g| {
            // Surface a few representative paths per group, not every item.
            let (items, total_items) = cap(&g.items);
            let sample: Vec<Value> = items
                .iter()
                .map(|i| json!({ "path": i.path, "sizeBytes": i.size_bytes, "safe": i.safe }))
                .collect();
            json!({
                "id": g.id,
                "label": g.label,
                "kind": format!("{:?}", g.kind),
                "description": g.description,
                "totalBytes": g.total_bytes,
                "itemCount": total_items,
                "recommended": g.recommended,
                "sampleItems": sample,
            })
        })
        .collect();

    Ok(json!({
        "groups": out,
        "groupCount": groups.len(),
        "totalBytes": total_bytes,
        "recommendedBytes": recommended_bytes,
    }))
}

fn find_large_old_files(args: &Value) -> AppResult<Value> {
    let root = require_path(args)?;
    let min_size = (opt_f64(args, "minSizeMb").unwrap_or(100.0).max(0.0) * MB as f64) as u64;
    let older_than_days = opt_u64(args, "olderThanDays").unwrap_or(180);

    let files = crate::cleaning::large_old::find_large_old(&root, min_size, older_than_days)?;
    let total_bytes: u64 = files.iter().map(|f| f.size_bytes).sum();
    let (shown, total) = cap(&files);
    let items: Vec<Value> = shown.iter().map(file_entry_json).collect();

    Ok(json!({
        "files": items,
        "shown": items.len(),
        "total": total,
        "totalBytes": total_bytes,
        "note": "大文件属于『需用户确认』类别，删除前务必逐项确认",
    }))
}

fn find_duplicates(args: &Value) -> AppResult<Value> {
    let root = require_path(args)?;
    let min_size = (opt_f64(args, "minSizeMb").unwrap_or(1.0).max(0.0) * MB as f64) as u64;

    let groups = crate::cleaning::duplicates::find_duplicates(&root, min_size)?;
    let wasted_total: u64 = groups.iter().map(|g| g.wasted_bytes).sum();
    let (shown, total) = cap(&groups);
    let out: Vec<Value> = shown
        .iter()
        .map(|g| {
            let files: Vec<Value> = g.files.iter().map(file_entry_json).collect();
            json!({
                "hash": g.hash,
                "sizeBytes": g.size_bytes,
                "wastedBytes": g.wasted_bytes,
                "files": files,
            })
        })
        .collect();

    Ok(json!({
        "groups": out,
        "shown": out.len(),
        "total": total,
        "wastedBytesTotal": wasted_total,
    }))
}

fn list_applications() -> AppResult<Value> {
    let apps = crate::cleaning::uninstaller::list_applications()?;
    let total_bytes: u64 = apps.iter().map(|a| a.size_bytes).sum();
    let (shown, total) = cap(&apps);
    let items: Vec<Value> = shown
        .iter()
        .map(|a| {
            json!({
                "id": a.id,
                "name": a.name,
                "path": a.path,
                "version": a.version,
                "sizeBytes": a.size_bytes,
                "lastUsed": a.last_used,
            })
        })
        .collect();

    Ok(json!({
        "applications": items,
        "shown": items.len(),
        "total": total,
        "totalBytes": total_bytes,
        "note": "卸载应用属于『需用户确认』操作",
    }))
}

fn list_startup_items() -> AppResult<Value> {
    let items = crate::cleaning::startup::list_startup_items()?;
    let (shown, total) = cap(&items);
    let out: Vec<Value> = shown
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "name": s.name,
                "path": s.path,
                "enabled": s.enabled,
                "kind": s.kind,
            })
        })
        .collect();

    Ok(json!({ "items": out, "shown": out.len(), "total": total }))
}

fn clean_paths(args: &Value, _state: &AppState) -> AppResult<Value> {
    let paths: Vec<String> = args
        .get("paths")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if paths.is_empty() {
        return Err(AppError::Agent("clean_paths 需要至少一个路径".into()));
    }
    // Default to the recoverable path (trash) when the model omits the flag.
    let to_trash = args.get("toTrash").and_then(Value::as_bool).unwrap_or(true);

    let report = crate::cleaning::trash::clean_paths(&paths, to_trash)?;
    Ok(clean_report_json(&report))
}

fn empty_trash() -> AppResult<Value> {
    let report = crate::cleaning::trash::empty_trash()?;
    Ok(clean_report_json(&report))
}

// ---------------------------------------------------------------------------
// JSON shaping helpers
// ---------------------------------------------------------------------------

fn file_entry_json(f: &crate::model::FileEntry) -> Value {
    json!({
        "path": f.path,
        "name": f.name,
        "sizeBytes": f.size_bytes,
        "modified": f.modified,
        "category": f.category.label(),
    })
}

fn clean_report_json(r: &crate::model::CleanReport) -> Value {
    json!({
        "removedCount": r.removed_count,
        "freedBytes": r.freed_bytes,
        "toTrash": r.to_trash,
        "failedCount": r.failed.len(),
        "failed": cap(&r.failed).0,
    })
}
