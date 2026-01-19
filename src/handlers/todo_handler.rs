use crate::models::{CreateTodo, UpdateTodo};
use crate::repositories::TodoRepo;
use crate::utils::{cors, errors, context::AppContext};
use worker::*;

pub async fn list_todos(_req: Request, app: AppContext) -> Result<Response> {
    let todos = TodoRepo::list(&app).await?;
    cors::add_headers(Response::from_json(&todos)?)
}

pub async fn create_todo(mut req: Request, app: AppContext) -> Result<Response> {
    let body: CreateTodo = req.json().await
        .map_err(|_| Error::RustError("Invalid JSON".into()))?;

    if body.title.trim().is_empty() {
        return errors::json_error("Title is required", 400);
    }

    let todo = TodoRepo::create(&app, body.title).await?;
    cors::add_headers(Response::from_json(&todo)?.with_status(201))
}

pub async fn update_todo(mut req: Request, ctx: RouteContext<()>, app: AppContext) -> Result<Response> {
    let id: i64 = ctx.param("id")
        .ok_or_else(|| Error::RustError("Missing id parameter".into()))?
        .parse()
        .map_err(|_| Error::RustError("Invalid id parameter".into()))?;
    
    let body: UpdateTodo = req.json().await?;
    let todo = TodoRepo::update(&app, id, body.completed).await?;
    cors::add_headers(Response::from_json(&todo)?)
}

pub async fn delete_todo(_req: Request, ctx: RouteContext<()>, app: AppContext) -> Result<Response> {
    let id: i64 = ctx.param("id")
        .ok_or_else(|| Error::RustError("Missing id parameter".into()))?
        .parse()
        .map_err(|_| Error::RustError("Invalid id parameter".into()))?;
    
    TodoRepo::delete(&app, id).await?;
    cors::add_headers(Response::ok("deleted")?)
}
