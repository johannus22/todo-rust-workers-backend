use worker::*;

pub fn log_request(req: &Request) {
    console_log!("{} {}", req.method(), req.path());
}
