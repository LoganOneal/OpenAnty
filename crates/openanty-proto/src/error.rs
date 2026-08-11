use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Machine-readable error codes for agents and REST clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    ProfileNotFound,
    SessionNotFound,
    SessionAlreadyRunning,
    SessionExpired,
    PortConflict,
    ProxyDead,
    ProxyAuthFailed,
    FingerprintInconsistent,
    BinaryMissing,
    BinaryMajorMismatch,
    ResourceLimit,
    StorageLocked,
    CorruptProfile,
    CookiesPartial,
    CookiesApplyFailed,
    Unauthorized,
    UnauthorizedBind,
    AlreadyRunning,
    InvalidRequest,
    NotInitialized,
    Internal,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProfileNotFound => "PROFILE_NOT_FOUND",
            Self::SessionNotFound => "SESSION_NOT_FOUND",
            Self::SessionAlreadyRunning => "SESSION_ALREADY_RUNNING",
            Self::SessionExpired => "SESSION_EXPIRED",
            Self::PortConflict => "PORT_CONFLICT",
            Self::ProxyDead => "PROXY_DEAD",
            Self::ProxyAuthFailed => "PROXY_AUTH_FAILED",
            Self::FingerprintInconsistent => "FINGERPRINT_INCONSISTENT",
            Self::BinaryMissing => "BINARY_MISSING",
            Self::BinaryMajorMismatch => "BINARY_MAJOR_MISMATCH",
            Self::ResourceLimit => "RESOURCE_LIMIT",
            Self::StorageLocked => "STORAGE_LOCKED",
            Self::CorruptProfile => "CORRUPT_PROFILE",
            Self::CookiesPartial => "COOKIES_PARTIAL",
            Self::CookiesApplyFailed => "COOKIES_APPLY_FAILED",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::UnauthorizedBind => "UNAUTHORIZED_BIND",
            Self::AlreadyRunning => "ALREADY_RUNNING",
            Self::InvalidRequest => "INVALID_REQUEST",
            Self::NotInitialized => "NOT_INITIALIZED",
            Self::Internal => "INTERNAL",
        }
    }

    pub fn retryable(self) -> bool {
        matches!(
            self,
            Self::PortConflict
                | Self::ProxyDead
                | Self::ResourceLimit
                | Self::SessionExpired
                | Self::CookiesApplyFailed
                | Self::Internal
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl ErrorBody {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code.as_str().to_string(),
            message: message.into(),
            retryable: code.retryable(),
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

#[derive(Debug, Error)]
pub enum OpenAntyError {
    #[error("{message}")]
    App {
        code: ErrorCode,
        message: String,
        hint: Option<String>,
    },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl OpenAntyError {
    pub fn app(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::App {
            code,
            message: message.into(),
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        if let Self::App { hint: h, .. } = &mut self {
            *h = Some(hint.into());
        }
        self
    }

    pub fn code(&self) -> ErrorCode {
        match self {
            Self::App { code, .. } => *code,
            Self::Other(_) => ErrorCode::Internal,
        }
    }

    pub fn body(&self) -> ErrorBody {
        match self {
            Self::App {
                code,
                message,
                hint,
            } => {
                let mut b = ErrorBody::new(*code, message.clone());
                if let Some(h) = hint {
                    b.hint = Some(h.clone());
                }
                b
            }
            Self::Other(e) => ErrorBody::new(ErrorCode::Internal, e.to_string()),
        }
    }
}

// anyhow is only used via Other — keep dependency light through thiserror paths in core.
// Provide From for common cases without forcing anyhow on all call sites.
impl From<std::io::Error> for OpenAntyError {
    fn from(value: std::io::Error) -> Self {
        Self::app(ErrorCode::Internal, value.to_string())
    }
}

impl From<serde_json::Error> for OpenAntyError {
    fn from(value: serde_json::Error) -> Self {
        Self::app(ErrorCode::InvalidRequest, value.to_string())
    }
}
