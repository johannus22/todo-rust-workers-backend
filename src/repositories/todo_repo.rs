use crate::db::{KetoClient, ListParams, SupabaseClient};
use crate::models::{AdminTodo, Todo};
use crate::utils::{context::AppContext, logging};
use std::collections::HashMap;
use worker::*;

const KETO_NAMESPACE: &str = "todos";
const KETO_RELATION_OWNER: &str = "owner";

fn subject_id(user_id: &str) -> String {
    format!("user:{}", user_id)
}

pub struct TodoRepo;

impl TodoRepo {
    /// List todos owned by the user. Uses Keto to resolve owned ids, then fetches from Supabase.
    pub async fn list(ctx: &AppContext, user_id: &str) -> Result<Vec<Todo>> {
        let keto = KetoClient::from_env(ctx)?;
        let sub = subject_id(user_id);

        let list = keto
            .list_relation_tuples(ListParams {
                namespace: KETO_NAMESPACE.to_string(),
                object: None,
                relation: Some(KETO_RELATION_OWNER.to_string()),
                subject_id: Some(sub),
                subject_set: None,
                page_size: Some(500),
                page_token: None,
            })
            .await?;

        let ids: Vec<i64> = list
            .get("relation_tuples")
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|t| t.get("object").and_then(|o| o.as_str()))
                    .filter_map(|s| s.parse::<i64>().ok())
                    .collect()
            })
            .unwrap_or_default();

        if ids.is_empty() {
            return Ok(vec![]);
        }

        let db = SupabaseClient::from_env(ctx)?;
        let id_list = ids
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let query = format!(
            "select=id,title,completed,created_at&id=in.({})&order=created_at.desc",
            id_list
        );
        let json_value = db.get("todos", &query).await?;
        match json_value {
            serde_json::Value::Array(arr) => {
                let todos: Vec<Todo> = serde_json::from_value(serde_json::Value::Array(arr))?;
                Ok(todos)
            }
            _ => Err(Error::RustError(format!(
                "Expected array, got: {}",
                json_value
            ))),
        }
    }

    /// List all todos with owner info (admin-only).
    pub async fn list_all_with_owner(ctx: &AppContext) -> Result<Vec<AdminTodo>> {
        let db = SupabaseClient::from_env(ctx)?;
        let query = "select=id,title,completed,created_at&order=created_at.desc";
        let json_value = db.get("todos", query).await?;
        let todos: Vec<Todo> = match json_value {
            serde_json::Value::Array(arr) => {
                serde_json::from_value(serde_json::Value::Array(arr))?
            }
            _ => {
                return Err(Error::RustError(format!(
                    "Expected array, got: {}",
                    json_value
                )))
            }
        };

        let keto = KetoClient::from_env(ctx)?;
        let tuples = keto
            .list_relation_tuples(ListParams {
                namespace: KETO_NAMESPACE.to_string(),
                object: None,
                relation: Some(KETO_RELATION_OWNER.to_string()),
                subject_id: None,
                subject_set: None,
                page_size: Some(1000),
                page_token: None,
            })
            .await?;

        let mut owners: HashMap<i64, String> = HashMap::new();
        if let Some(arr) = tuples.get("relation_tuples").and_then(|v| v.as_array()) {
            for t in arr {
                let object = t.get("object").and_then(|o| o.as_str());
                let subject_id = t.get("subject_id").and_then(|s| s.as_str());
                if let (Some(obj), Some(sub)) = (object, subject_id) {
                    if let Ok(id) = obj.parse::<i64>() {
                        let owner_id = sub.strip_prefix("user:").unwrap_or(sub).to_string();
                        owners.insert(id, owner_id);
                    }
                }
            }
        }

        let admin_todos = todos
            .into_iter()
            .map(|t| AdminTodo {
                id: t.id,
                title: t.title,
                completed: t.completed,
                created_at: t.created_at,
                owner_id: owners.get(&t.id).cloned(),
                owner_email: None,
            })
            .collect::<Vec<_>>();

        Ok(admin_todos)
    }

    /// Create a todo and set the caller as owner in Keto.
    pub async fn create(ctx: &AppContext, user_id: &str, title: String) -> Result<Todo> {
        let db = SupabaseClient::from_env(ctx)?;
        let body = serde_json::json!({ "title": title });

        let json_value = db.post("todos", body).await?;
        let todo: Todo = match json_value {
            serde_json::Value::Array(arr) => {
                let todos: Vec<Todo> = serde_json::from_value(serde_json::Value::Array(arr))?;
                todos.into_iter().next().ok_or_else(|| Error::RustError("No todo returned".into()))?
            }
            _ => {
                return Err(Error::RustError(format!(
                    "Expected array, got: {}",
                    json_value
                )))
            }
        };

        let keto = KetoClient::from_env(ctx)?;
        keto.create_relation_tuple(
            KETO_NAMESPACE,
            &todo.id.to_string(),
            KETO_RELATION_OWNER,
            &subject_id(user_id),
        )
        .await?;

        Ok(todo)
    }

    /// Update a todo if the user is owner.
    pub async fn update(
        ctx: &AppContext,
        user_id: &str,
        id: i64,
        completed: bool,
    ) -> Result<Todo> {
        let keto = KetoClient::from_env(ctx)?;
        let allowed = keto
            .check(crate::db::CheckParams {
                namespace: KETO_NAMESPACE.to_string(),
                object: id.to_string(),
                relation: KETO_RELATION_OWNER.to_string(),
                subject_id: Some(subject_id(user_id)),
                subject_set: None,
                max_depth: None,
            })
            .await?;

        if !allowed {
            return Err(Error::RustError("Forbidden".into()));
        }

        let db = SupabaseClient::from_env(ctx)?;
        let body = serde_json::json!({ "completed": completed });
        let json_value = db.patch("todos", id, body).await?;
        match json_value {
            serde_json::Value::Array(arr) => {
                let todos: Vec<Todo> = serde_json::from_value(serde_json::Value::Array(arr))?;
                todos.into_iter().next().ok_or_else(|| Error::RustError("Todo not found".into()))
            }
            _ => Err(Error::RustError(format!(
                "Expected array, got: {}",
                json_value
            ))),
        }
    }

    /// Delete a todo if the user is owner. Also removes the ownership tuple in Keto.
    pub async fn delete(ctx: &AppContext, user_id: &str, id: i64) -> Result<()> {
        let keto = KetoClient::from_env(ctx)?;
        let allowed = keto
            .check(crate::db::CheckParams {
                namespace: KETO_NAMESPACE.to_string(),
                object: id.to_string(),
                relation: KETO_RELATION_OWNER.to_string(),
                subject_id: Some(subject_id(user_id)),
                subject_set: None,
                max_depth: None,
            })
            .await?;

        if !allowed {
            return Err(Error::RustError("Forbidden".into()));
        }

        let db = SupabaseClient::from_env(ctx)?;
        db.delete("todos", id).await?;

        // Remove ownership tuple in Keto after successful Supabase delete. Log on failure; do not fail the request.
        if let Err(e) = keto
            .delete_relation_tuple(
                KETO_NAMESPACE,
                &id.to_string(),
                KETO_RELATION_OWNER,
                &subject_id(user_id),
            )
            .await
        {
            logging::log_error(&format!("keto delete relation tuple: {}", e));
        }

        Ok(())
    }

    /// Delete any todo (admin-only). Attempts to remove owner tuples in Keto if available.
    pub async fn delete_any(ctx: &AppContext, id: i64) -> Result<()> {
        let db = SupabaseClient::from_env(ctx)?;
        db.delete("todos", id).await?;

        if let Ok(keto) = KetoClient::from_env(ctx) {
            let tuples = keto
                .list_relation_tuples(ListParams {
                    namespace: KETO_NAMESPACE.to_string(),
                    object: Some(id.to_string()),
                    relation: Some(KETO_RELATION_OWNER.to_string()),
                    subject_id: None,
                    subject_set: None,
                    page_size: Some(100),
                    page_token: None,
                })
                .await;

            match tuples {
                Ok(json) => {
                    if let Some(arr) = json.get("relation_tuples").and_then(|v| v.as_array()) {
                        for t in arr {
                            if let Some(sub) = t.get("subject_id").and_then(|s| s.as_str()) {
                                if let Err(e) = keto
                                    .delete_relation_tuple(
                                        KETO_NAMESPACE,
                                        &id.to_string(),
                                        KETO_RELATION_OWNER,
                                        sub,
                                    )
                                    .await
                                {
                                    logging::log_error(&format!(
                                        "keto delete relation tuple (admin): {}",
                                        e
                                    ));
                                }
                            }
                        }
                    }
                }
                Err(e) => logging::log_error(&format!("keto list relation tuples: {}", e)),
            }
        }

        Ok(())
    }
}
