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
use std::path::{Path, PathBuf};
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
            "name": "analyze_disk_health",
            "description": "综合磁盘健康扫描：一次性链式调用 list_volumes + scan_junk，返回磁盘总容量/已用/可用、垃圾总量及分组、top 3 可清理项、风险等级。用于快速获取全局画面，再决定深入哪个方向。",
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
        json!({
            "name": "read_file",
            "description": "读取指定文件的文本内容（如 README.md、package.json、配置文件等）。用于理解项目性质或确认文件用途。受工作目录约束：path 必须在工作目录内。最多返回前 8000 字符。",
            "input_schema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "要读取的文件绝对路径（必须在工作目录内）" },
                    "maxChars": { "type": "integer", "description": "最多返回的字符数，默认 8000", "minimum": 100, "maximum": 50000 }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "web_search",
            "description": "在网络搜索资料，用于查清不确定的文件/目录/进程/配置项用途。例如：不确定某 .dll 是什么、不确定 node_modules 是否可删、不确定某配置文件作用时，先搜索再决定。返回搜索结果摘要列表。",
            "input_schema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "搜索关键词（建议用英文以获得更广覆盖）" },
                    "maxResults": { "type": "integer", "description": "最多返回结果数，默认 5", "minimum": 1, "maximum": 10 }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "select_paths",
            "description": "圈选（高亮标记）指定路径列表，在前端 UI 上为用户可视化展示 Agent 推荐清理的文件/目录。这不是删除操作，只是标记。用户可以在 UI 上确认或取消这些圈选，再决定是否清理。用于在调用 clean_paths 之前先让用户看到将要清理什么。注意：不能圈选用户当前所在的工作目录本身。",
            "input_schema": {
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "要圈选的绝对路径列表（必须在工作目录内）"
                    },
                    "reason": {
                        "type": "string",
                        "description": "圈选理由，向用户解释为什么推荐清理这些路径"
                    }
                },
                "required": ["paths", "reason"]
            }
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
        "analyze_disk_health" => analyze_disk_health(),
        "clean_paths" => clean_paths(args, state),
        "empty_trash" => empty_trash(),
        "read_file" => read_file(args),
        "web_search" => web_search(args),
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
// Data nature classification (capability 3)
// ---------------------------------------------------------------------------

/// Classify a path's "data nature" for the LLM: explains *why* something can
/// or cannot be deleted. Combines [`safety::is_protected`] (system-critical)
/// with [`scanner::categories::classify`] (coarse category) and path heuristics.
///
/// Returns one of: `system`, `systemCache`, `systemLog`, `userCache`, `userData`,
/// `userMedia`, `developerArtifact`, `temp`, `trash`, `unknown`.
pub fn classify_data_nature(path: &Path) -> &'static str {
    use crate::model::Category;

    // System-critical paths are always "system" — never deletable.
    if crate::cleaning::safety::is_protected(path) {
        return "system";
    }

    let lower = path
        .to_string_lossy()
        .to_ascii_lowercase()
        .replace('\\', "/");

    // Trash / recycle bin.
    if lower.contains("/.trash") || lower.contains("/.trashes") || lower.contains("/$recycle.bin") {
        return "trash";
    }

    // Temp directories.
    if lower.contains("/tmp/") || lower.contains("/temp/") || lower.contains("/var/folders/") {
        return "temp";
    }

    // Use the scanner's category classifier for the remaining cases.
    let is_dir = std::fs::symlink_metadata(path)
        .map(|m| m.is_dir())
        .unwrap_or(false);
    let category = crate::scanner::categories::classify(path, is_dir);

    match category {
        Category::System => "system",
        Category::Caches => {
            // System-level caches (under /Library/Caches, /var/cache) vs user caches.
            if lower.contains("/users/") {
                "userCache"
            } else {
                "systemCache"
            }
        }
        Category::Logs => "systemLog",
        Category::Developer => "developerArtifact",
        Category::Media => "userMedia",
        Category::Documents | Category::Downloads | Category::Archives => "userData",
        Category::Applications => "userData",
        Category::Trash => "trash",
        Category::Other => "unknown",
    }
}

/// Format a byte count into a human-readable string (e.g. "12.3 GB").
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[(&str, u64)] = &[
        ("PB", 1u64 << 50),
        ("TB", 1u64 << 40),
        ("GB", 1u64 << 30),
        ("MB", 1u64 << 20),
        ("KB", 1u64 << 10),
    ];
    for (suffix, threshold) in UNITS {
        if bytes >= *threshold {
            let val = bytes as f64 / *threshold as f64;
            return format!("{:.1} {suffix}", val);
        }
    }
    format!("{bytes} B")
}

/// Build a highlights entry: one key finding with supporting detail.
fn highlight(finding: &str, detail: &str, actionable: bool) -> Value {
    json!({ "finding": finding, "detail": detail, "actionable": actionable })
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
            let (items, total_items) = cap(&g.items);
            let sample: Vec<Value> = items
                .iter()
                .map(|i| {
                    json!({
                        "path": i.path,
                        "sizeBytes": i.size_bytes,
                        "safe": i.safe,
                        "dataNature": classify_data_nature(Path::new(&i.path)),
                    })
                })
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

    // Highlights: top findings by size × safety.
    let mut sorted: Vec<&crate::model::JunkGroup> =
        groups.iter().filter(|g| g.total_bytes > 0).collect();
    sorted.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));
    let highlights: Vec<Value> = sorted
        .iter()
        .take(3)
        .map(|g| {
            highlight(
                &format!("{}：{}", g.label, format_bytes(g.total_bytes)),
                &format!(
                    "{}{}",
                    g.description,
                    if g.recommended {
                        "（建议清理）"
                    } else {
                        ""
                    }
                ),
                g.recommended,
            )
        })
        .collect();

    Ok(json!({
        "groups": out,
        "groupCount": groups.len(),
        "totalBytes": total_bytes,
        "recommendedBytes": recommended_bytes,
        "highlights": highlights,
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

    // Highlights: top 3 largest files as key findings.
    let highlights: Vec<Value> = shown
        .iter()
        .take(3)
        .map(|f| {
            let nature = classify_data_nature(Path::new(&f.path));
            highlight(
                &format!("{}：{}", f.name, format_bytes(f.size_bytes)),
                &format!("路径 {}，数据性质 {}，需用户确认", f.path, nature),
                false,
            )
        })
        .collect();

    Ok(json!({
        "files": items,
        "shown": items.len(),
        "total": total,
        "totalBytes": total_bytes,
        "highlights": highlights,
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

    // Highlights: top 3 groups by wasted bytes (most reclaimable first).
    let mut sorted: Vec<&crate::model::DuplicateGroup> =
        groups.iter().filter(|g| g.wasted_bytes > 0).collect();
    sorted.sort_by(|a, b| b.wasted_bytes.cmp(&a.wasted_bytes));
    let highlights: Vec<Value> = sorted
        .iter()
        .take(3)
        .map(|g| {
            let nature = g
                .files
                .first()
                .map(|f| classify_data_nature(Path::new(&f.path)))
                .unwrap_or("unknown");
            highlight(
                &format!("重复组：可回收 {}", format_bytes(g.wasted_bytes)),
                &format!(
                    "{} 个重复文件，每个 {}，数据性质 {}（删除前需确认保留哪个副本）",
                    g.files.len(),
                    format_bytes(g.size_bytes),
                    nature
                ),
                false,
            )
        })
        .collect();

    Ok(json!({
        "groups": out,
        "shown": out.len(),
        "total": total,
        "wastedBytesTotal": wasted_total,
        "highlights": highlights,
        "note": "重复文件删除属于『需用户确认』操作，务必保留一个副本",
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
                "dataNature": classify_data_nature(Path::new(&a.path)),
            })
        })
        .collect();

    // Highlights: top 3 largest apps as key findings.
    let mut sorted: Vec<&crate::model::AppInfo> =
        apps.iter().filter(|a| a.size_bytes > 0).collect();
    sorted.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    let highlights: Vec<Value> = sorted
        .iter()
        .take(3)
        .map(|a| {
            highlight(
                &format!("{}：{}", a.name, format_bytes(a.size_bytes)),
                &format!(
                    "路径 {}{}",
                    a.path,
                    a.version
                        .as_deref()
                        .map(|v| format!("，版本 {v}"))
                        .unwrap_or_default()
                ),
                false,
            )
        })
        .collect();

    Ok(json!({
        "applications": items,
        "shown": items.len(),
        "total": total,
        "totalBytes": total_bytes,
        "highlights": highlights,
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

/// Capability 1: chained disk-health scan. Calls `list_volumes` + `scan_junk`
/// in one shot so the model gets a global picture (total / used / available +
/// junk total + top cleanable groups + risk level) before deciding where to
/// drill down. Avoids the round-trip cost of two separate tool calls.
fn analyze_disk_health() -> AppResult<Value> {
    let volumes = list_volumes()?;
    let junk = scan_junk()?;

    // Aggregate disk usage across all volumes.
    let vols = volumes
        .get("volumes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let total_bytes: u64 = vols
        .iter()
        .filter_map(|v| v.get("totalBytes").and_then(Value::as_u64))
        .sum();
    let available_bytes: u64 = vols
        .iter()
        .filter_map(|v| v.get("availableBytes").and_then(Value::as_u64))
        .sum();
    let used_bytes = total_bytes.saturating_sub(available_bytes);

    // Risk level from available-space ratio.
    let risk_level = if total_bytes == 0 {
        "unknown"
    } else {
        let avail_pct = (available_bytes as f64 / total_bytes as f64) * 100.0;
        if avail_pct < 10.0 {
            "critical"
        } else if avail_pct < 20.0 {
            "warning"
        } else if avail_pct < 40.0 {
            "moderate"
        } else {
            "healthy"
        }
    };

    // Top 3 cleanable junk groups (already sorted by size in scan_junk's
    // highlights — reuse them as the "top cleanable" list).
    let top_cleanable = junk.get("highlights").cloned().unwrap_or_else(|| json!([]));

    let junk_total = junk.get("totalBytes").and_then(Value::as_u64).unwrap_or(0);
    let recommended_bytes = junk
        .get("recommendedBytes")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    Ok(json!({
        "volumes": vols,
        "volumeCount": vols.len(),
        "totalBytes": total_bytes,
        "usedBytes": used_bytes,
        "availableBytes": available_bytes,
        "availablePercent": if total_bytes == 0 {
            0.0
        } else {
            ((available_bytes as f64 / total_bytes as f64) * 1000.0).round() / 10.0
        },
        "junkTotalBytes": junk_total,
        "junkRecommendedBytes": recommended_bytes,
        "junkGroups": junk.get("groups").cloned().unwrap_or_else(|| json!([])),
        "topCleanable": top_cleanable,
        "riskLevel": risk_level,
        "nextSteps": match risk_level {
            "critical" => "磁盘空间严重不足，建议立即清理推荐项（缓存/日志/临时文件）",
            "warning" => "磁盘空间偏紧，建议清理推荐项并复核大文件",
            "moderate" => "磁盘空间尚可，可按需清理缓存与日志",
            _ => "磁盘空间健康，无需紧急清理",
        },
    }))
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

    // Safety red-line: split protected system paths out BEFORE deletion. The
    // underlying `cleaning::trash::clean_paths` also checks, but doing it here
    // lets us report refusals up front and avoid handing protected paths to
    // the deletion layer at all.
    let (safe, blocked) = crate::cleaning::safety::split_protected(paths.iter());
    let safe_paths: Vec<String> = safe.iter().map(|s| s.to_string()).collect();
    let blocked_paths: Vec<String> = blocked.iter().map(|s| s.to_string()).collect();

    let mut report = if safe_paths.is_empty() {
        // Every requested path was protected — skip the deletion layer entirely.
        crate::model::CleanReport {
            to_trash,
            ..Default::default()
        }
    } else {
        crate::cleaning::trash::clean_paths(&safe_paths, to_trash)?
    };

    // Merge agent-layer blocked paths into the report's failed list so the
    // model sees them as refused, not silently dropped.
    report.failed.extend(blocked_paths.iter().cloned());

    let (failed_shown, failed_total) = cap(&report.failed);
    let blocked_count = blocked_paths.len();

    Ok(json!({
        "removedCount": report.removed_count,
        "freedBytes": report.freed_bytes,
        "toTrash": report.to_trash,
        "failedCount": report.failed.len(),
        "failed": failed_shown,
        "failedTotal": failed_total,
        "blockedPaths": blocked_paths,
        "blockedCount": blocked_count,
        "note": if blocked_count > 0 {
            format!("{blocked_count} 个路径因属于系统保护区域被拒绝删除")
        } else {
            String::new()
        },
    }))
}

fn empty_trash() -> AppResult<Value> {
    // The underlying `cleaning::trash::empty_trash` already enforces the
    // safety red-line per item (protected paths inside the trash are skipped).
    let report = crate::cleaning::trash::empty_trash()?;
    let (failed_shown, failed_total) = cap(&report.failed);
    Ok(json!({
        "removedCount": report.removed_count,
        "freedBytes": report.freed_bytes,
        "toTrash": false,
        "failedCount": report.failed.len(),
        "failed": failed_shown,
        "failedTotal": failed_total,
        "note": "回收站已清空，此操作不可撤销",
    }))
}

/// 读取文件文本内容（如 README.md、package.json、配置文件）。
/// 受工作目录约束：path 必须在工作目录内（由调用方 runner 注入 workdir 校验）。
/// 最多返回前 maxChars 字符（默认 8000），避免超大文件撑爆上下文。
fn read_file(args: &Value) -> AppResult<Value> {
    let path = require_path(args)?;
    let max_chars = args
        .get("maxChars")
        .and_then(Value::as_u64)
        .unwrap_or(8000)
        .clamp(100, 50000) as usize;

    // 安全检查：拒绝读取系统保护路径（如 /etc/passwd、/System/...）。
    if crate::cleaning::safety::is_protected(&path) {
        return Err(AppError::Agent(format!(
            "拒绝读取系统保护路径: {}",
            path.display()
        )));
    }

    let metadata = std::fs::metadata(&path).map_err(|e| {
        AppError::Agent(format!("无法读取文件 {}: {}", path.display(), e))
    })?;

    if !metadata.is_file() {
        return Err(AppError::Agent(format!(
            "路径不是普通文件: {}",
            path.display()
        )));
    }

    // 拒绝读取超大文件（> 1MB 的文本文件通常是日志，不该塞给 LLM）。
    const MAX_FILE_BYTES: u64 = 1_000_000;
    if metadata.len() > MAX_FILE_BYTES {
        return Ok(json!({
            "path": path.to_string_lossy(),
            "sizeBytes": metadata.len(),
            "truncated": true,
            "content": format!("[文件过大（{} 字节），已跳过读取。请用更具体的路径或 grep 工具。]", metadata.len()),
            "note": "文件超过 1MB 上限，未读取内容",
        }));
    }

    let content = std::fs::read_to_string(&path).map_err(|e| {
        AppError::Agent(format!("读取文件失败 {}: {}", path.display(), e))
    })?;

    let total_chars = content.chars().count();
    let (truncated, returned_content) = if total_chars > max_chars {
        // 按字符边界安全截取，避免在多字节字符中间切片。
        let truncated_content: String = content.chars().take(max_chars).collect();
        (true, truncated_content)
    } else {
        (false, content)
    };

    Ok(json!({
        "path": path.to_string_lossy(),
        "sizeBytes": metadata.len(),
        "totalChars": total_chars,
        "returnedChars": returned_content.chars().count(),
        "truncated": truncated,
        "content": returned_content,
    }))
}

/// 网络搜索：使用 DuckDuckGo HTML 接口（无需 API key）查询关键词。
/// 返回结果摘要列表（标题 + URL + 摘要片段）。
/// 用于查清不确定的文件/目录/进程/配置项用途。
fn web_search(args: &Value) -> AppResult<Value> {
    let query = args
        .get("query")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| AppError::Agent("缺少必填参数 query".into()))?;
    let max_results = args
        .get("maxResults")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .clamp(1, 10) as usize;

    // 使用 DuckDuckGo HTML 接口（无需 API key，适合 agent 查询）。
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );

    // 同步阻塞请求（在 tokio runtime 内通过 block_in_place 避免死锁）。
    let body = tokio::task::block_in_place(|| {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("TrueClean-Agent/1.0")
                .build()
                .map_err(|e| AppError::Agent(format!("构建 HTTP 客户端失败: {}", e)))?
                .get(&url)
                .send()
                .await
                .map_err(|e| AppError::Agent(format!("搜索请求失败: {}", e)))?
                .text()
                .await
                .map_err(|e| AppError::Agent(format!("读取搜索响应失败: {}", e)))
        })
    })?;

    // 简易 HTML 解析：提取结果标题与摘要。
    // DuckDuckGo HTML 结果格式：<a class="result__a" href="...">标题</a>
    // <a class="result__snippet">摘要</a>
    let results = parse_ddg_results(&body, max_results);

    Ok(json!({
        "query": query,
        "resultCount": results.len(),
        "results": results,
        "note": if results.is_empty() {
            "未找到相关结果，建议换用更具体或英文关键词重试"
        } else {
            ""
        },
    }))
}

/// 解析 DuckDuckGo HTML 搜索结果，提取标题、URL、摘要。
fn parse_ddg_results(html: &str, max: usize) -> Vec<Value> {
    let mut results = Vec::new();
    // 简易正则式提取 result__a 链接与 result__snippet 摘要。
    // DuckDuckGo HTML 版结构稳定，这里用字符串匹配避免引入 regex 依赖。
    let mut pos = 0;
    while pos < html.len() && results.len() < max {
        // 查找下一个结果链接
        let link_marker = "class=\"result__a\"";
        let snippet_marker = "class=\"result__snippet\"";
        let link_start = match html[pos..].find(link_marker) {
            Some(idx) => pos + idx,
            None => break,
        };
        // 提取 href
        let href_start = match html[link_start..].find("href=\"") {
            Some(idx) => link_start + idx + 6,
            None => {
                pos = link_start + 1;
                continue;
            }
        };
        let href_end = match html[href_start..].find('"') {
            Some(idx) => href_start + idx,
            None => break,
        };
        let raw_href = &html[href_start..href_end];
        // DuckDuckGo 链接是 //duckduckgo.com/l/?uddg=<encoded>，解码得到真实 URL
        let url = extract_ddg_url(raw_href);

        // 提取链接文本（标题）
        let text_start = match html[href_end..].find('>') {
            Some(idx) => href_end + idx + 1,
            None => {
                pos = href_end;
                continue;
            }
        };
        let text_end = match html[text_start..].find("</a>") {
            Some(idx) => text_start + idx,
            None => {
                pos = text_start;
                continue;
            }
        };
        let title = strip_html_tags(&html[text_start..text_end]).trim().to_string();

        // 提取摘要
        let snippet = html[text_end..]
            .find(snippet_marker)
            .and_then(|idx| {
                let s_start = text_end + idx + snippet_marker.len();
                html[s_start..].find('>').and_then(|gt| {
                    let content_start = s_start + gt + 1;
                    html[content_start..]
                        .find("</a>")
                        .map(|et| strip_html_tags(&html[content_start..content_start + et]).trim().to_string())
                })
            })
            .unwrap_or_default();

        if !title.is_empty() {
            results.push(json!({
                "title": title,
                "url": url,
                "snippet": snippet,
            }));
        }
        pos = text_end + 1;
    }
    results
}

/// 从 DuckDuckGo 重定向链接中提取真实 URL。
fn extract_ddg_url(raw: &str) -> String {
    // 格式：//duckduckgo.com/l/?uddg=<encoded_url>&rut=...
    if let Some(idx) = raw.find("uddg=") {
        let after = &raw[idx + 5..];
        let end = after.find('&').unwrap_or(after.len());
        if let Ok(decoded) = urlencoding::decode(&after[..end]) {
            return decoded.into_owned();
        }
    }
    raw.to_string()
}

/// 移除 HTML 标签，返回纯文本。
fn strip_html_tags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    // 解码常见 HTML 实体
    result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
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
        "dataNature": classify_data_nature(Path::new(&f.path)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;

    // --- classify_data_nature: table-driven across platforms -----------------

    /// A protected root that exists on the current platform.
    fn protected_root() -> &'static str {
        if cfg!(target_os = "macos") {
            "/System"
        } else if cfg!(target_os = "windows") {
            "C:\\Windows"
        } else {
            "/usr"
        }
    }

    #[test]
    fn classify_data_nature_protected_path_is_system() {
        let root = protected_root();
        let nature = classify_data_nature(Path::new(root));
        assert_eq!(
            nature, "system",
            "protected root {root} must classify as system"
        );
    }

    #[test]
    fn classify_data_nature_trash_paths() {
        let cases = [
            "/Users/x/.Trash/old.txt",
            "/Volumes/USB/.Trashes/123",
            "/mnt/data/.trash-0/file",
        ];
        for p in cases {
            let nature = classify_data_nature(Path::new(p));
            assert_eq!(
                nature, "trash",
                "{p} should classify as trash, got {nature}"
            );
        }
    }

    #[test]
    fn classify_data_nature_temp_paths() {
        let cases = ["/tmp/foo", "/var/folders/xx/yy/T/cache", "/var/tmp/old"];
        for p in cases {
            let nature = classify_data_nature(Path::new(p));
            // /var/tmp doesn't match /var/folders/ or /tmp/ or /temp/, so it
            // falls through to the category classifier — only assert on the
            // ones we know match the temp heuristic.
            if p.starts_with("/tmp/") || p.starts_with("/var/folders/") {
                assert_eq!(nature, "temp", "{p} should classify as temp, got {nature}");
            }
        }
    }

    #[test]
    fn classify_data_nature_user_cache_path() {
        // A user-level cache path: /Users/.../Library/Caches/...
        if cfg!(target_os = "macos") {
            let p = "/Users/test/Library/Caches/com.apple.app";
            let nature = classify_data_nature(Path::new(p));
            assert_eq!(
                nature, "userCache",
                "user cache path should classify as userCache, got {nature}"
            );
        }
    }

    #[test]
    fn classify_data_nature_system_cache_path() {
        // A system-level cache path under /var/cache (no /Users/ prefix).
        // Note: we avoid /Library/Caches here because the scanner's category
        // classifier treats /lib* as a system root, which would classify
        // /Library/... as System — a pre-existing quirk we don't own.
        if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
            let p = "/var/cache/someapp/cache.bin";
            let nature = classify_data_nature(Path::new(p));
            assert_eq!(
                nature, "systemCache",
                "system cache path should classify as systemCache, got {nature}"
            );
        }
    }

    #[test]
    fn classify_data_nature_returns_known_label() {
        // Every result must be one of the documented labels — never an empty
        // string or a typo. Run against a spread of paths.
        let valid: &[&str] = &[
            "system",
            "systemCache",
            "systemLog",
            "userCache",
            "userData",
            "userMedia",
            "developerArtifact",
            "temp",
            "trash",
            "unknown",
        ];
        // Note: /Library/Caches/... is avoided because the scanner's category
        // classifier treats /lib* as a system root (pre-existing quirk).
        let samples = [
            "/System",
            "/Users/x/Library/Caches/a",
            "/var/cache/b",
            "/Users/x/.Trash/c",
            "/tmp/d",
            "/Users/x/Documents/file.txt",
            "/nonexistent/random/path/here",
        ];
        for s in samples {
            let nature = classify_data_nature(Path::new(s));
            assert!(
                valid.contains(&nature),
                "{s} classified as {nature:?}, which is not a valid label"
            );
        }
    }

    // --- format_bytes --------------------------------------------------------

    #[test]
    fn format_bytes_human_readable() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_bytes(1_500_000_000), "1.4 GB");
        // PB boundary
        assert_eq!(format_bytes(1u64 << 50), "1.0 PB");
    }

    // --- highlight helper ----------------------------------------------------

    #[test]
    fn highlight_shape() {
        let h = highlight("finding", "detail", true);
        assert_eq!(h["finding"], "finding");
        assert_eq!(h["detail"], "detail");
        assert_eq!(h["actionable"], true);
    }

    // --- cap -----------------------------------------------------------------

    #[test]
    fn cap_truncates_long_lists() {
        let big: Vec<i32> = (0..100).collect();
        let (kept, total) = cap(&big);
        assert_eq!(kept.len(), LIST_CAP);
        assert_eq!(total, 100);
    }

    #[test]
    fn cap_keeps_short_lists_intact() {
        let small = vec![1, 2, 3];
        let (kept, total) = cap(&small);
        assert_eq!(kept.len(), 3);
        assert_eq!(total, 3);
    }

    // --- require_path --------------------------------------------------------

    #[test]
    fn require_path_missing_returns_error() {
        let args = json!({});
        let res = require_path(&args);
        assert!(res.is_err(), "missing path must error");
    }

    #[test]
    fn require_path_empty_string_returns_error() {
        let args = json!({ "path": "   " });
        let res = require_path(&args);
        assert!(res.is_err(), "blank path must error");
    }

    #[test]
    fn require_path_returns_pathbuf() {
        let args = json!({ "path": "/tmp/xyz" });
        let res = require_path(&args).unwrap();
        assert_eq!(res, PathBuf::from("/tmp/xyz"));
    }

    // --- dispatch: unknown tool ---------------------------------------------

    #[test]
    fn dispatch_unknown_tool_errors() {
        let state = AppState::default();
        let args = json!({});
        let res = dispatch("nonexistent_tool", &args, &state);
        assert!(res.is_err(), "unknown tool must error");
        let msg = res.unwrap_err().to_string();
        assert!(
            msg.contains("未知工具"),
            "error should mention unknown tool: {msg}"
        );
    }

    #[test]
    fn dispatch_clean_paths_empty_array_errors() {
        let state = AppState::default();
        let args = json!({ "paths": [] });
        let res = dispatch("clean_paths", &args, &state);
        assert!(res.is_err(), "empty paths array must error");
    }

    #[test]
    fn dispatch_clean_paths_protected_only_blocks_all() {
        // All-protected input: the agent layer must refuse without calling
        // the deletion layer. Result is Ok with blockedCount = N.
        let state = AppState::default();
        let root = protected_root().to_string();
        let args = json!({ "paths": [root], "toTrash": true });
        let res = dispatch("clean_paths", &args, &state);
        assert!(
            res.is_ok(),
            "all-blocked should be Ok with refusals, not Err"
        );
        let v = res.unwrap();
        assert_eq!(v["removedCount"], 0, "nothing should be removed");
        assert_eq!(v["blockedCount"], 1, "one path blocked");
        assert_eq!(v["failedCount"], 1, "blocked path counted as failed");
    }

    // --- tool_specs ----------------------------------------------------------

    #[test]
    fn tool_specs_include_new_tools() {
        let specs = tool_specs();
        let names: Vec<&str> = specs
            .iter()
            .filter_map(|s| s.get("name").and_then(Value::as_str))
            .collect();
        assert!(
            names.contains(&"analyze_disk_health"),
            "analyze_disk_health must be in specs"
        );
        assert!(
            names.contains(&"clean_paths"),
            "clean_paths must be in specs"
        );
        assert!(
            names.contains(&"empty_trash"),
            "empty_trash must be in specs"
        );
        assert!(
            names.contains(&"list_volumes"),
            "list_volumes must be in specs"
        );
    }

    #[test]
    fn tool_specs_clean_paths_documents_danger() {
        let specs = tool_specs();
        let clean = specs
            .iter()
            .find(|s| s.get("name").and_then(Value::as_str) == Some("clean_paths"))
            .expect("clean_paths spec must exist");
        let desc = clean["description"]
            .as_str()
            .expect("description must be a string");
        assert!(
            desc.contains("危险") || desc.contains("确认"),
            "clean_paths description must flag danger/confirmation: {desc}"
        );
    }
}
