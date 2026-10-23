//! The company's master data.

use crate::{Answer, Platform};
use serde_json::Value;

pub fn call(cmd: &str, _args: &Value, _p: &dyn Platform) -> Answer {
    Err(format!("unknown command: {cmd}"))
}

pub fn call_bytes(cmd: &str, _args: &Value, _p: &dyn Platform) -> Result<Vec<u8>, String> {
    Err(format!("unknown command: {cmd}"))
}
