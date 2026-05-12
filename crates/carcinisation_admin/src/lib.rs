//! Shared admin protocol types for the carcinisation multiplayer server.
//!
//! JSON-lines over a Unix domain socket. One request per connection,
//! one response back, then the connection closes.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Default directory for admin sockets (matches systemd `RuntimeDirectory`).
pub const DEFAULT_SOCKET_DIR: &str = "/run/carcinisation";

/// Derive the admin socket path for a named instance.
pub fn socket_path_for(instance: &str) -> PathBuf {
    Path::new(DEFAULT_SOCKET_DIR).join(format!("{instance}.admin.sock"))
}

// ---------------------------------------------------------------------------
// Request
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum AdminRequest {
    Help,
    Status,
    Players,
    Say { message: String },
    Restart,
    ResetMap,
    Shutdown,
}

// ---------------------------------------------------------------------------
// Response
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl AdminResponse {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            ok: true,
            message: Some(message.into()),
            error: None,
            data: None,
        }
    }

    pub fn success_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            ok: true,
            message: Some(message.into()),
            error: None,
            data: Some(data),
        }
    }

    pub fn error(error: impl Into<String>) -> Self {
        Self {
            ok: false,
            message: None,
            error: Some(error.into()),
            data: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_help() {
        let json = r#"{"command":"help"}"#;
        let req: AdminRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req, AdminRequest::Help));
    }

    #[test]
    fn parse_status() {
        let json = r#"{"command":"status"}"#;
        let req: AdminRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req, AdminRequest::Status));
    }

    #[test]
    fn parse_say() {
        let json = r#"{"command":"say","message":"hello world"}"#;
        let req: AdminRequest = serde_json::from_str(json).unwrap();
        match req {
            AdminRequest::Say { message } => assert_eq!(message, "hello world"),
            other => panic!("expected Say, got {other:?}"),
        }
    }

    #[test]
    fn parse_shutdown() {
        let json = r#"{"command":"shutdown"}"#;
        let req: AdminRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req, AdminRequest::Shutdown));
    }

    #[test]
    fn parse_restart() {
        let json = r#"{"command":"restart"}"#;
        let req: AdminRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req, AdminRequest::Restart));
    }

    #[test]
    fn parse_reset_map() {
        let json = r#"{"command":"reset_map"}"#;
        let req: AdminRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req, AdminRequest::ResetMap));
    }

    #[test]
    fn parse_unknown_command_fails() {
        let json = r#"{"command":"explode"}"#;
        assert!(serde_json::from_str::<AdminRequest>(json).is_err());
    }

    #[test]
    fn response_success_roundtrip() {
        let resp = AdminResponse::success("ok");
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: AdminResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.ok);
        assert_eq!(parsed.message.as_deref(), Some("ok"));
        assert!(parsed.error.is_none());
        assert!(parsed.data.is_none());
    }

    #[test]
    fn response_error_roundtrip() {
        let resp = AdminResponse::error("bad thing");
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: AdminResponse = serde_json::from_str(&json).unwrap();
        assert!(!parsed.ok);
        assert!(parsed.message.is_none());
        assert_eq!(parsed.error.as_deref(), Some("bad thing"));
    }

    #[test]
    fn response_with_data_roundtrip() {
        let data = serde_json::json!({"players": 3});
        let resp = AdminResponse::success_with_data("status", data.clone());
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: AdminResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.ok);
        assert_eq!(parsed.data.unwrap(), data);
    }

    #[test]
    fn socket_path_derivation() {
        let path = socket_path_for("deathmatch");
        assert_eq!(
            path,
            PathBuf::from("/run/carcinisation/deathmatch.admin.sock")
        );
    }
}
