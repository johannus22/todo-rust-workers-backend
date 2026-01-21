use worker::*;

/// Reads `X-User-Id` from the request. Returns `None` if missing or empty.
/// In handlers: `let user_id = match utils::auth::get_user_id(&req) { Some(u) => u, None => return errors::json_error("Missing X-User-Id", 401) };`
pub fn get_user_id(req: &Request) -> Option<String> {
    let s = req.headers().get("X-User-Id").ok().flatten()?;
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    Some(s.to_string())
}
