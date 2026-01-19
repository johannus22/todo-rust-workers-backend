use crate::models::CreateUser;
use crate::repositories::UserRepo;
use crate::utils::{cors, errors};
use worker::*;

pub async fn list_users(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let users = UserRepo::list(&ctx).await?;
    cors::add_headers(Response::from_json(&users)?)
}

pub async fn create_user(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: CreateUser = match req.json().await {
        Ok(v) => v,
        Err(_) => return errors::json_error("Invalid JSON", 400),
    };

    if body.name.trim().is_empty() {
        return errors::json_error("Name is required", 400);
    }

    let user = UserRepo::create(&ctx, body.name).await?;
    cors::add_headers(Response::from_json(&user)?.with_status(201))
}
