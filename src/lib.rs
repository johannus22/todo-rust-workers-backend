mod models;
mod db;
mod repositories;
mod handlers;
mod utils;

use utils::cors;
use worker::*;

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

    Router::new()
        .get("/health", handlers::health::health_check)
        .get_async("/users", handlers::user_handler::list_users)
        .post_async("/users", handlers::user_handler::create_user)
        .get_async("/api/todos", handlers::todo_handler::list_todos)
        .post_async("/api/todos", handlers::todo_handler::create_todo)
        .patch_async("/api/todos/:id", handlers::todo_handler::update_todo)
        .delete_async("/api/todos/:id", handlers::todo_handler::delete_todo)
        .run(req, env)
        .await
}
