use crate::db::SupabaseClient;
use crate::models::User;
use crate::utils::context::AppContext;
use worker::*;

pub struct UserRepo;

impl UserRepo {
    pub async fn list(ctx: &AppContext) -> Result<Vec<User>> {
        let client = SupabaseClient::from_env(ctx)?;
        let query = "select=id,name&order=id.desc";
        
        let json_value = client.get("users", query).await?;
        match json_value {
            serde_json::Value::Array(arr) => {
                let users: Vec<User> = serde_json::from_value(serde_json::Value::Array(arr))?;
                Ok(users)
            }
            _ => Err(Error::RustError(format!("Expected array, got: {}", json_value)))
        }
    }

    pub async fn create(ctx: &AppContext, name: String) -> Result<User> {
        let client = SupabaseClient::from_env(ctx)?;
        let body = serde_json::json!({ "name": name });
        
        let json_value = client.post("users", body).await?;
        match json_value {
            serde_json::Value::Array(arr) => {
                let users: Vec<User> = serde_json::from_value(serde_json::Value::Array(arr))?;
                users.into_iter().next().ok_or_else(|| Error::RustError("No user returned".into()))
            }
            _ => Err(Error::RustError(format!("Expected array, got: {}", json_value)))
        }
    }
}
