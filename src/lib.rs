mod models;
mod db;
mod repositories;
mod handlers;
mod utils;

use utils::cors;
use worker::*;
use utils::context::AppContext;

#[event(fetch)]
async fn fetch(
    req: Request,
    env: Env,
    _ctx: Context,
) -> Result<Response> {
    utils::logging::log_request(&req);

    // Handle CORS preflight (OPTIONS) requests
    if req.method() == Method::Options {
        return cors::handle_preflight();
    }

    let app_ctx = AppContext::new(env.clone());
    let env_for_router = env;

    Router::new()
        .get("/health", handlers::health::health_check)
        .get_async("/users", {
            let app_ctx = app_ctx.clone();
            move |req, _| {
                let app_ctx = app_ctx.clone();
                async move { handlers::user_handler::list_users(req, app_ctx).await }
            }
        })
        .post_async("/users", {
            let app_ctx = app_ctx.clone();
            move |req, _| {
                let app_ctx = app_ctx.clone();
                async move { handlers::user_handler::create_user(req, app_ctx).await }
            }
        })
        .get_async("/api/todos", {
            let app_ctx = app_ctx.clone();
            move |req, _| {
                let app_ctx = app_ctx.clone();
                async move { handlers::todo_handler::list_todos(req, app_ctx).await }
            }
        })
        .post_async("/api/todos", {
            let app_ctx = app_ctx.clone();
            move |req, _| {
                let app_ctx = app_ctx.clone();
                async move { handlers::todo_handler::create_todo(req, app_ctx).await }
            }
        })
        .patch_async("/api/todos/:id", {
            let app_ctx = app_ctx.clone();
            move |req, ctx| {
                let app_ctx = app_ctx.clone();
                async move { handlers::todo_handler::update_todo(req, ctx, app_ctx).await }
            }
        })
        .delete_async("/api/todos/:id", {
            let app_ctx = app_ctx.clone();
            move |req, ctx| {
                let app_ctx = app_ctx.clone();
                async move { handlers::todo_handler::delete_todo(req, ctx, app_ctx).await }
            }
        })
        .get_async("/api/admin/todos", {
            let app_ctx = app_ctx.clone();
            move |req, _| {
                let app_ctx = app_ctx.clone();
                async move { handlers::todo_handler::admin_list_todos(req, app_ctx).await }
            }
        })
        .delete_async("/api/admin/todos/:id", {
            let app_ctx = app_ctx.clone();
            move |req, ctx| {
                let app_ctx = app_ctx.clone();
                async move { handlers::todo_handler::admin_delete_todo(req, ctx, app_ctx).await }
            }
        })
        .run(req, env_for_router)
        .await
}
