//! Unified error type. Serializes to `{ message: string }` for the frontend.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("路径无效: {0}")]
    InvalidPath(String),

    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("网络错误: {0}")]
    Http(String),

    #[error("操作已取消")]
    Cancelled,

    #[error("配置错误: {0}")]
    Config(String),

    #[error("Agent 错误: {0}")]
    Agent(String),

    #[error("权限不足: {0}")]
    PermissionDenied(String),

    #[error("{0}")]
    Other(String),
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Http(e.to_string())
    }
}

impl From<trash::Error> for AppError {
    fn from(e: trash::Error) -> Self {
        AppError::Other(format!("回收站操作失败: {e}"))
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut st = serializer.serialize_struct("AppError", 1)?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// P0: PermissionDenied 变体的 Display 实现应包含 "权限不足" 提示
    /// 和具体路径，便于前端直接展示给用户。
    #[test]
    fn permission_denied_display() {
        let e = AppError::PermissionDenied("test path".into());
        let s = format!("{}", e);
        assert!(s.contains("权限不足"), "应包含 '权限不足': {s}");
        assert!(s.contains("test path"), "应包含路径: {s}");
    }

    /// P0: std::io::Error 应能通过 `From` 自动转换为 AppError::Io，
    /// 这是 `#[from]` 派生的核心契约，扫描器与清理模块都依赖此转换。
    #[test]
    fn io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let app_err: AppError = io_err.into();
        assert!(
            matches!(app_err, AppError::Io(_)),
            "io::Error 应转换为 AppError::Io"
        );
    }

    /// P0: Cancelled 变体的 Display 应包含 "取消" 字样，便于前端区分
    /// 取消与真实错误。
    #[test]
    fn cancelled_display() {
        let e = AppError::Cancelled;
        let s = format!("{}", e);
        assert!(s.contains("取消"), "应包含 '取消': {s}");
    }
}
