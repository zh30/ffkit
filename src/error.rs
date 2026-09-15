use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    Input,
    Ffmpeg,
    Output,
    MissingTool,
    Timeout,
    Verification,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{message}")]
    Kind { kind: ErrorKind, message: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl Error {
    pub fn kind(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self::Kind {
            kind,
            message: message.into(),
        }
    }

    pub fn input(message: impl Into<String>) -> Self {
        Self::kind(ErrorKind::Input, message)
    }

    pub fn ffmpeg(message: impl Into<String>) -> Self {
        Self::kind(ErrorKind::Ffmpeg, message)
    }

    pub fn output(message: impl Into<String>) -> Self {
        Self::kind(ErrorKind::Output, message)
    }

    pub fn missing_tool(message: impl Into<String>) -> Self {
        Self::kind(ErrorKind::MissingTool, message)
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::kind(ErrorKind::Timeout, message)
    }

    pub fn verification(message: impl Into<String>) -> Self {
        Self::kind(ErrorKind::Verification, message)
    }

    pub fn kind_of(&self) -> ErrorKind {
        match self {
            Self::Kind { kind, .. } => *kind,
            Self::Io(_) => ErrorKind::Output,
            Self::Json(_) => ErrorKind::Input,
        }
    }

    pub fn message(&self) -> String {
        self.to_string()
    }
}
