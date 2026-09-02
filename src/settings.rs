//! Action settings as persisted by Stream Deck.
//!
//! This struct is only a serde view of `payload.settings` on each event.
//! Do not store it in a HashMap keyed by context.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ActionSettings {
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub composition_op: String,
    #[serde(default)]
    pub trans_op: String,
    #[serde(default)]
    pub tenths: String,
    #[serde(default)]
    pub trans_type: String,
    #[serde(default)]
    pub pinp_key: String,
    #[serde(default)]
    pub pos_h: String,
    #[serde(default)]
    pub pos_v: String,
    #[serde(default)]
    pub dsk_op: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub output: String,
    #[serde(default)]
    pub output_assign: String,
    #[serde(default)]
    pub channel6: String,
    #[serde(default)]
    pub slot: String,
    #[serde(default)]
    pub audio_op: String,
    #[serde(default)]
    pub audio_input: String,
    #[serde(default)]
    pub analog_input: String,
    #[serde(default)]
    pub system_op: String,
    #[serde(default)]
    pub hdcp: String,
    #[serde(default)]
    pub test_pattern: String,
    #[serde(default)]
    pub test_tone: String,
    #[serde(default)]
    pub tally_check: String,
    /// `manual` shows Host / Test connection. `saved` uses an existing endpoint.
    #[serde(default)]
    pub connection_mode: String,
}

impl ActionSettings {
    pub fn host_trimmed(&self) -> &str {
        self.host.trim()
    }
}

#[derive(Debug, Deserialize)]
pub struct PiMessage {
    #[serde(default)]
    pub property_inspector: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EndpointInfo {
    pub host: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct PiOut {
    pub status: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub endpoints: Vec<EndpointInfo>,
}

impl PiOut {
    pub fn state(status: impl Into<String>, endpoints: Vec<EndpointInfo>) -> Self {
        Self {
            status: status.into(),
            endpoints,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pi_message_reads_test_connection() {
        let msg: PiMessage =
            serde_json::from_str(r#"{"command":"test_connection","host":"192.168.3.39"}"#).unwrap();
        assert_eq!(msg.command.as_deref(), Some("test_connection"));
        assert_eq!(msg.host.as_deref(), Some("192.168.3.39"));
    }

    #[test]
    fn pi_out_omits_empty_endpoints() {
        let json = serde_json::to_value(PiOut::state("Connected", Vec::new())).unwrap();
        assert_eq!(json["status"], "Connected");
        assert!(json.get("endpoints").is_none());
    }
}
