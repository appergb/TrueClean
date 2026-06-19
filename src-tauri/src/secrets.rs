//! Secure secret storage wrapper around the `keyring` crate.
//!
//! API Keys (Claude / OpenAI) are stored in the OS-native secret store:
//! - macOS: Keychain
//! - Windows: Credential Manager
//! - Linux: Secret Service (libsecret / D-Bus)
//!
//! On Linux, if `libsecret` is not available (e.g. headless CI), `is_available()`
//! returns `false` and callers fall back to plaintext `settings.json`.

use crate::error::{AppError, AppResult};
use keyring::Entry;

const SERVICE: &str = "com.trueclean.app";
const ACCOUNT_CLAUDE: &str = "claude_api_key";
const ACCOUNT_OPENAI: &str = "openai_api_key";
const ACCOUNT_DEEPSEEK: &str = "deepseek_api_key";

/// 检查 keyring 是否可用（Linux 无 libsecret 时返回 false）
pub fn is_available() -> bool {
    // 尝试创建一个测试 entry 并 get_password，如果返回 NoEntry 或 Ok 则可用，返回其他错误则不可用
    match Entry::new(SERVICE, "__test_availability__") {
        Ok(entry) => match entry.get_password() {
            Ok(_) => true,
            Err(keyring::Error::NoEntry) => true,
            Err(_) => false,
        },
        Err(_) => false,
    }
}

/// 从 keyring 加载 key。不存在时返回 None，keyring 不可用时返回 None
pub fn load_key(account: &str) -> Option<String> {
    let entry = Entry::new(SERVICE, account).ok()?;
    match entry.get_password() {
        Ok(s) if !s.is_empty() => Some(s),
        Ok(_) => None,
        Err(keyring::Error::NoEntry) => None,
        Err(_) => None,
    }
}

/// 存储 key 到 keyring。失败时返回 Err
pub fn store_key(account: &str, key: &str) -> AppResult<()> {
    let entry = Entry::new(SERVICE, account)
        .map_err(|e| AppError::Config(format!("keyring 创建失败: {e}")))?;
    entry
        .set_password(key)
        .map_err(|e| AppError::Config(format!("keyring 写入失败: {e}")))
}

/// 删除 keyring 中的 key。不存在时视为成功
pub fn delete_key(account: &str) -> AppResult<()> {
    let entry = Entry::new(SERVICE, account)
        .map_err(|e| AppError::Config(format!("keyring 创建失败: {e}")))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Config(format!("keyring 删除失败: {e}"))),
    }
}

pub const CLAUDE_ACCOUNT: &str = ACCOUNT_CLAUDE;
pub const OPENAI_ACCOUNT: &str = ACCOUNT_OPENAI;
pub const DEEPSEEK_ACCOUNT: &str = ACCOUNT_DEEPSEEK;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_load_delete_roundtrip() {
        // 仅在 keyring 可用时运行
        if !is_available() {
            eprintln!("跳过 keyring 测试：当前环境不可用");
            return;
        }
        let test_account = "__test_roundtrip__";
        let test_key = "sk-test-key-12345";

        store_key(test_account, test_key).expect("store 应成功");
        let loaded = load_key(test_account);
        assert_eq!(loaded.as_deref(), Some(test_key));

        delete_key(test_account).expect("delete 应成功");
        assert_eq!(load_key(test_account), None);
    }

    #[test]
    fn test_load_nonexistent_returns_none() {
        if !is_available() {
            return;
        }
        assert_eq!(load_key("__nonexistent_account__"), None);
    }

    #[test]
    fn test_delete_nonexistent_is_ok() {
        if !is_available() {
            return;
        }
        assert!(delete_key("__nonexistent_account__").is_ok());
    }
}
