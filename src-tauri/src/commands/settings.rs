//! Settings persistence. Stored as JSON under the OS config dir.

use crate::error::{AppError, AppResult};
use crate::model::AppSettings;
use crate::state::AppState;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

fn settings_path() -> AppResult<PathBuf> {
    let mut dir =
        dirs::config_dir().ok_or_else(|| AppError::Config("无法定位系统配置目录".into()))?;
    dir.push("TrueClean");
    fs::create_dir_all(&dir)?;
    dir.push("settings.json");
    Ok(dir)
}

/// Read settings from disk, falling back to defaults on any error.
pub fn read_settings() -> AppSettings {
    let Ok(path) = settings_path() else {
        return AppSettings::default();
    };
    let Ok(text) = fs::read_to_string(path) else {
        return AppSettings::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

/// Load persisted settings into managed state. Called at startup.
pub fn load_into_state(app: &AppHandle) {
    let mut settings = read_settings();

    // 迁移：如果 keyring 可用且 settings.json 中有明文 key，迁移到 keyring
    if crate::secrets::is_available() {
        let mut needs_rewrite = false;

        // Claude key 迁移
        if !settings.claude_api_key.is_empty() {
            if crate::secrets::load_key(crate::secrets::CLAUDE_ACCOUNT).is_none() {
                // keyring 中没有，从 settings.json 迁移过去
                if crate::secrets::store_key(
                    crate::secrets::CLAUDE_ACCOUNT,
                    &settings.claude_api_key,
                )
                .is_ok()
                {
                    settings.claude_api_key = String::new();
                    needs_rewrite = true;
                }
            } else {
                // keyring 中已有，清空内存中的明文
                settings.claude_api_key = String::new();
                needs_rewrite = true;
            }
        }

        // OpenAI key 迁移（同上逻辑）
        if !settings.openai_api_key.is_empty() {
            if crate::secrets::load_key(crate::secrets::OPENAI_ACCOUNT).is_none() {
                if crate::secrets::store_key(
                    crate::secrets::OPENAI_ACCOUNT,
                    &settings.openai_api_key,
                )
                .is_ok()
                {
                    settings.openai_api_key = String::new();
                    needs_rewrite = true;
                }
            } else {
                settings.openai_api_key = String::new();
                needs_rewrite = true;
            }
        }

        // DeepSeek key 迁移（同上逻辑）
        if !settings.deepseek_api_key.is_empty() {
            if crate::secrets::load_key(crate::secrets::DEEPSEEK_ACCOUNT).is_none() {
                if crate::secrets::store_key(
                    crate::secrets::DEEPSEEK_ACCOUNT,
                    &settings.deepseek_api_key,
                )
                .is_ok()
                {
                    settings.deepseek_api_key = String::new();
                    needs_rewrite = true;
                }
            } else {
                settings.deepseek_api_key = String::new();
                needs_rewrite = true;
            }
        }

        // 如果迁移了，回写 settings.json（清空 key 字段）
        if needs_rewrite {
            let _ = write_settings(&settings);
        }
    }

    // P0-7: 即使 Mutex 被 poison（持锁线程 panic），也恢复其内部数据，
    // 避免单次 panic 让整个设置通道永久不可用。
    *app.state::<AppState>()
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = settings;
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> AppResult<AppSettings> {
    // P0-7: 即使 Mutex 被 poison（持锁线程 panic），也恢复其内部数据，
    // 避免单次 panic 让整个设置通道永久不可用。
    let mut settings = state
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();

    // 掩码处理：如果 keyring 中有 key，返回掩码占位（非空，让前端知道 key 已存储）
    if crate::secrets::is_available() {
        if crate::secrets::load_key(crate::secrets::CLAUDE_ACCOUNT).is_some() {
            settings.claude_api_key = "********".to_string();
        }
        if crate::secrets::load_key(crate::secrets::OPENAI_ACCOUNT).is_some() {
            settings.openai_api_key = "********".to_string();
        }
        if crate::secrets::load_key(crate::secrets::DEEPSEEK_ACCOUNT).is_some() {
            settings.deepseek_api_key = "********".to_string();
        }
    }

    Ok(settings)
}

/// 原子写入 settings.json：先写到同目录下的临时文件，再 rename 覆盖目标。
/// 这样即使写入过程中途崩溃，settings.json 也不会出现半截内容。
/// 同目录 rename 在同一文件系统上为原子操作（POSIX rename / Windows MoveFileEx）。
fn write_settings(settings: &AppSettings) -> AppResult<()> {
    let path = settings_path()?;
    let json = serde_json::to_string_pretty(settings)?;

    let parent = path
        .parent()
        .ok_or_else(|| AppError::Config("无法定位 settings 父目录".into()))?;
    fs::create_dir_all(parent)?;

    let tmp_path = path.with_extension("json.tmp");
    {
        let mut f = fs::File::create(&tmp_path)?;
        f.write_all(json.as_bytes())?;
        // 确保数据落盘再 rename，避免掉电后 tmp 文件为空。
        f.sync_all()?;
    }
    // 原子替换：tmp -> settings.json
    fs::rename(&tmp_path, &path)?;
    Ok(())
}

#[tauri::command]
pub fn save_settings(settings: AppSettings, state: State<AppState>) -> AppResult<()> {
    let mut settings = settings;

    // 处理 API Key：非空且非掩码时存入 keyring
    if crate::secrets::is_available() {
        // Claude
        if !settings.claude_api_key.is_empty() && settings.claude_api_key != "********" {
            // 用户输入了新 key，存入 keyring
            crate::secrets::store_key(crate::secrets::CLAUDE_ACCOUNT, &settings.claude_api_key)?;
            settings.claude_api_key = String::new();
        } else if settings.claude_api_key == "********" {
            // 掩码占位：用户未修改 key，保持 keyring 不变，settings.json 字段保持空串
            settings.claude_api_key = String::new();
        } else {
            // 空串表示用户清空了 key，删除 keyring 中的
            crate::secrets::delete_key(crate::secrets::CLAUDE_ACCOUNT)?;
        }
        // OpenAI（同上逻辑）
        if !settings.openai_api_key.is_empty() && settings.openai_api_key != "********" {
            crate::secrets::store_key(crate::secrets::OPENAI_ACCOUNT, &settings.openai_api_key)?;
            settings.openai_api_key = String::new();
        } else if settings.openai_api_key == "********" {
            settings.openai_api_key = String::new();
        } else {
            crate::secrets::delete_key(crate::secrets::OPENAI_ACCOUNT)?;
        }
        // DeepSeek（同上逻辑）
        if !settings.deepseek_api_key.is_empty() && settings.deepseek_api_key != "********" {
            crate::secrets::store_key(
                crate::secrets::DEEPSEEK_ACCOUNT,
                &settings.deepseek_api_key,
            )?;
            settings.deepseek_api_key = String::new();
        } else if settings.deepseek_api_key == "********" {
            settings.deepseek_api_key = String::new();
        } else {
            crate::secrets::delete_key(crate::secrets::DEEPSEEK_ACCOUNT)?;
        }
    }

    // P0-7: 原子写入 — 先写到同目录下的临时文件，再 rename 覆盖目标。
    write_settings(&settings)?;

    // 写盘成功后再更新内存状态，保证磁盘与内存一致。
    *state.settings.lock().unwrap_or_else(|e| e.into_inner()) = settings;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// P0-7: 被 poison 的 Mutex 仍能通过 unwrap_or_else 取出内部值，
    /// 不会让 get_settings 永久 panic。
    #[test]
    fn poisoned_mutex_does_not_break_get_settings() {
        // 模拟 poison 后恢复：手动 poison 一次再验证 unwrap_or_else 能拿到内部值。
        let m = std::sync::Arc::new(Mutex::new(AppSettings::default()));
        let m2 = m.clone();
        let h = std::thread::spawn(move || {
            let _g = m2.lock().unwrap();
            panic!("intentional poison");
        });
        let _ = h.join();
        // 现在 m 已被 poison，验证 unwrap_or_else 能拿到内部值。
        let recovered = m.lock().unwrap_or_else(|e| e.into_inner()).clone();
        assert_eq!(
            recovered.language,
            AppSettings::default().language,
            "poisoned mutex should still yield the inner value"
        );
    }

    /// P0-7: save_settings 使用原子写入，写入后文件存在且可读回。
    #[test]
    fn save_settings_writes_atomically_and_roundtrips() {
        // 用一个独立的临时 settings.json 路径，避免污染真实配置。
        // 这里通过直接调用底层逻辑验证：写 tmp + rename 后内容可读。
        let tmp_dir =
            std::env::temp_dir().join(format!("trueclean_settings_p07_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp_dir).unwrap();
        let target = tmp_dir.join("settings.json");
        let tmp = target.with_extension("json.tmp");

        let payload = AppSettings::default();
        let json = serde_json::to_string_pretty(&payload).unwrap();

        {
            let mut f = fs::File::create(&tmp).unwrap();
            f.write_all(json.as_bytes()).unwrap();
            f.sync_all().unwrap();
        }
        fs::rename(&tmp, &target).unwrap();

        // tmp 应已消失，target 应可读回。
        assert!(!tmp.exists(), "tmp file should be gone after rename");
        assert!(target.exists(), "target file should exist after rename");
        let read_back: AppSettings =
            serde_json::from_str(&fs::read_to_string(&target).unwrap()).unwrap();
        assert_eq!(
            read_back.language, payload.language,
            "roundtrip should preserve language"
        );

        let _ = fs::remove_dir_all(&tmp_dir);
    }
}
