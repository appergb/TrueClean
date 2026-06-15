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
