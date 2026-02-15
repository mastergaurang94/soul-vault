//! Shared export page state enums.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Context,
    Json,
    Bundle,
}

impl ExportFormat {
    pub fn next(self) -> Self {
        match self {
            Self::Context => Self::Json,
            Self::Json => Self::Bundle,
            Self::Bundle => Self::Context,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Context => "Markdown Context",
            Self::Json => "JSON",
            Self::Bundle => "Bundle (folder)",
        }
    }

    pub fn arg(self) -> &'static str {
        match self {
            Self::Context => "context",
            Self::Json => "json",
            Self::Bundle => "bundle",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportField {
    Format,
    Identity,
    Preferences,
    Topics,
    People,
    Memories,
    Execute,
}

impl ExportField {
    pub const ALL: [Self; 7] = [
        Self::Format,
        Self::Identity,
        Self::Preferences,
        Self::Topics,
        Self::People,
        Self::Memories,
        Self::Execute,
    ];

    pub fn next(self) -> Self {
        let idx = Self::ALL
            .iter()
            .position(|field| *field == self)
            .unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL
            .iter()
            .position(|field| *field == self)
            .unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}
