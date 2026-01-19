use crate::db::SupabaseClient;
use crate::models::Todo;
use worker::*;

pub struct TodoRepo;

impl TodoRepo {
    pub async fn list(ctx: &RouteContext<()>) -> Result<Vec<Todo>> {
        let client = SupabaseClient::from_env(ctx)?;
        let query = "select=id,title,completed,created_at&order=created_at.desc";
        
        let json_value = client.get("todos", query).await?;
        match json_value {
            serde_json::Value::Array(arr) => {
                let todos: Vec<Todo> = serde_json::from_value(serde_json::Value::Array(arr))?;
                Ok(todos)
            }
            _ => Err(Error::RustError(format!("Expected array, got: {}", json_value)))
        }
    }

    pub async fn create(ctx: &RouteContext<()>, title: String) -> Result<Todo> {
        let client = SupabaseClient::from_env(ctx)?;
        let body = serde_json::json!({ "title": title });
        
        let json_value = client.post("todos", body).await?;
        match json_value {
            serde_json::Value::Array(arr) => {
                let todos: Vec<Todo> = serde_json::from_value(serde_json::Value::Array(arr))?;
                todos.into_iter().next().ok_or_else(|| Error::RustError("No todo returned".into()))
            }
            _ => Err(Error::RustError(format!("Expected array, got: {}", json_value)))
        }
    }

    pub async fn update(ctx: &RouteContext<()>, id: i64, completed: bool) -> Result<Todo> {
        let client = SupabaseClient::from_env(ctx)?;
        let body = serde_json::json!({ "completed": completed });
        
        let json_value = client.patch("todos", id, body).await?;
        match json_value {
            serde_json::Value::Array(arr) => {
                let todos: Vec<Todo> = serde_json::from_value(serde_json::Value::Array(arr))?;
                todos.into_iter().next().ok_or_else(|| Error::RustError("Todo not found".into()))
            }
            _ => Err(Error::RustError(format!("Expected array, got: {}", json_value)))
        }
    }

    pub async fn delete(ctx: &RouteContext<()>, id: i64) -> Result<()> {
        let client = SupabaseClient::from_env(ctx)?;
        client.delete("todos", id).await
    }
}
