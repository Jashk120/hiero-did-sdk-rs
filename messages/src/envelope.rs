use serde::{
    Deserialize,
    Serialize,
};

/// The outer envelope submitted to HCS
/// { message: {...}, signature: "<base64>" }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HcsEnvelope {
    pub message: HcsMessage,
    pub signature: String,
}

/// The inner message object — this is what gets signed
/// JSON.stringify(this) is the bytes that are signed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HcsMessage {
    pub timestamp: String,
    pub operation: String,
    pub did: String,
    /// base64-encoded JSON event payload
    pub event: Option<String>,
}
