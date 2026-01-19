use worker::*;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppContext {
    pub env: Env,
    pub request_id: String,
    pub start_time: u64,
}
#[warn(unused)]
impl AppContext {
    pub fn new(env: Env) -> Self {
        Self {
            env,
            request_id: Uuid::new_v4().to_string(),
            start_time: Date::now().as_millis() as u64,
        }
    }
}
