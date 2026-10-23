//! The program and its companies.

use crate::{Answer, Platform};
use serde_json::Value;

pub fn call(cmd: &str, _args: &Value, _p: &dyn Platform) -> Answer {
    Err(format!("unknown command: {cmd}"))
}
