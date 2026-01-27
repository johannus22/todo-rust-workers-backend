use crate::db::{CheckParams, KetoClient, KratosClient};
use crate::utils::context::AppContext;
use crate::middleware::logging;
use worker::*;

const ADMIN_NAMESPACE: &str = "roles";
const ADMIN_OBJECT: &str = "admin";
const ADMIN_RELATION: &str = "member";

/// Reads `X-User-Id` from the request. Returns `None` if missing or empty.
/// In handlers: `let user_id = match middleware::auth::get_user_id(&req) { Some(u) => u, None => return errors::json_error("Missing X-User-Id", 401) };`
pub fn get_user_id(req: &Request) -> Option<String> {
    let s = req.headers().get("X-User-Id").ok().flatten()?;
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    Some(s.to_string())
}

pub async fn is_admin(ctx: &AppContext, user_id: &str) -> Result<bool> {
    if let Ok(kratos) = KratosClient::from_env(ctx) {
        match kratos.get_identity(user_id).await {
            Ok(json) => {
                let role = json
                    .get("metadata_public")
                    .and_then(|m| m.get("role"))
                    .and_then(|r| r.as_str());
                if role == Some("admin") {
                    return Ok(true);
                }
            }
            Err(e) => logging::log_error(&format!("kratos is_admin: {}", e)),
        }
    }

    if let Ok(keto) = KetoClient::from_env(ctx) {
        match keto
            .check(CheckParams {
                namespace: ADMIN_NAMESPACE.to_string(),
                object: ADMIN_OBJECT.to_string(),
                relation: ADMIN_RELATION.to_string(),
                subject_id: Some(format!("user:{}", user_id)),
                subject_set: None,
                max_depth: None,
            })
            .await
        {
            Ok(true) => return Ok(true),
            Ok(false) => {}
            Err(e) => logging::log_error(&format!("keto is_admin: {}", e)),
        }
    }

    Ok(false)
}
