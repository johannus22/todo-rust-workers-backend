use crate::utils::cors;
use worker::*;

pub fn json_error(msg: &str, status: u16) -> Result<Response> {
    let res = Response::from_json(&serde_json::json!({ "error": msg }))?;
    cors::add_headers(res.with_status(status))
}

pub fn forbidden() -> Result<Response> {
    json_error("Forbidden", 403)
}
