use crate::middleware::cors;
use worker::*;

pub fn health_check(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let app_name = ctx.env.var("APP_NAME")?.to_string();
    let mut res = Response::ok(format!("OK from {}", app_name))?;
    res.headers_mut().set("x-backend", "workers-rust")?;
    cors::add_headers(res)
}
