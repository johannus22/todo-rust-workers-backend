use worker::*;

pub fn log_request(req: &Request) {
    console_log!("{} {}", req.method(), req.path());
}

pub fn log_error(msg: &str) {
    console_log!("[ERROR] {}", msg);
}
