use crate::utils::cors;
use worker::*;

pub fn json_error(msg: &str, status: u16) -> Result<Response> {
    let res = Response::from_json(&serde_json::json!({ "error": msg }))?;
    cors::add_headers(res.with_status(status))
}

/// Returns 500 with `{ "message": msg }` for Keto/Supabase and other internal errors.
pub fn json_server_error(msg: &str) -> Result<Response> {
    let res = Response::from_json(&serde_json::json!({ "message": msg }))?;
    cors::add_headers(res.with_status(500))
}

pub fn forbidden() -> Result<Response> {
    json_error("Forbidden", 403)
}
