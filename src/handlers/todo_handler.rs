use crate::models::{CreateTodo, UpdateTodo};
use crate::repositories::TodoRepo;
use crate::utils::{auth, context::AppContext, cors, errors, logging};
use worker::*;

pub async fn list_todos(req: Request, app: AppContext) -> Result<Response> {
    let user_id = match auth::get_user_id(&req) {
        Some(u) => u,
        None => return errors::json_error("Missing X-User-Id", 401),
    };
    let todos = match TodoRepo::list(&app, &user_id).await {
        Ok(t) => t,
        Err(e) => {
            logging::log_error(&format!("list_todos: {}", e));
            return errors::json_server_error("Internal server error");
        }
    };
    cors::add_headers(Response::from_json(&todos)?)
}

pub async fn create_todo(mut req: Request, app: AppContext) -> Result<Response> {
    let user_id = match auth::get_user_id(&req) {
        Some(u) => u,
        None => return errors::json_error("Missing X-User-Id", 401),
    };
    let body: CreateTodo = req
        .json()
        .await
        .map_err(|_| Error::RustError("Invalid JSON".into()))?;

    if body.title.trim().is_empty() {
        return errors::json_error("Title is required", 400);
    }

    let todo = match TodoRepo::create(&app, &user_id, body.title).await {
        Ok(t) => t,
        Err(e) => {
            logging::log_error(&format!("create_todo: {}", e));
            return errors::json_server_error("Internal server error");
        }
    };
    cors::add_headers(Response::from_json(&todo)?.with_status(201))
}

pub async fn update_todo(
    mut req: Request,
    ctx: RouteContext<()>,
    app: AppContext,
) -> Result<Response> {
    let user_id = match auth::get_user_id(&req) {
        Some(u) => u,
        None => return errors::json_error("Missing X-User-Id", 401),
    };
    let id: i64 = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing id parameter".into()))?
        .parse()
        .map_err(|_| Error::RustError("Invalid id parameter".into()))?;

    let body: UpdateTodo = req.json().await?;
    match TodoRepo::update(&app, &user_id, id, body.completed).await {
        Ok(todo) => cors::add_headers(Response::from_json(&todo)?),
        Err(e) => {
            let msg = format!("{}", e);
            if msg.contains("Forbidden") {
                errors::forbidden()
            } else if msg.contains("Todo not found") {
                errors::json_error("Todo not found", 404)
            } else {
                logging::log_error(&format!("update_todo: {}", e));
                errors::json_server_error("Internal server error")
            }
        }
    }
}

pub async fn delete_todo(req: Request, ctx: RouteContext<()>, app: AppContext) -> Result<Response> {
    let user_id = match auth::get_user_id(&req) {
        Some(u) => u,
        None => return errors::json_error("Missing X-User-Id", 401),
    };
    let id: i64 = match ctx.param("id") {
        Some(id_str) => match id_str.parse() {
            Ok(i) => i,
            Err(_) => {
                logging::log_error("delete_todo invalid id parameter");
                return errors::json_server_error("Internal server error");
            }
        },
        None => {
            logging::log_error("delete_todo missing id parameter");
            return errors::json_server_error("Internal server error");
        }
    };

    match TodoRepo::delete(&app, &user_id, id).await {
        Ok(()) => cors::add_headers(Response::ok("deleted")?),
        Err(e) => {
            let msg = format!("{}", e);
            if msg.contains("Forbidden") {
                errors::forbidden()
            } else if msg.contains("Todo not found") {
                errors::json_error("Todo not found", 404)
            } else {
                logging::log_error(&format!("delete_todo: {}", e));
                errors::json_server_error("Internal server error")
            }
        }
    }
}
