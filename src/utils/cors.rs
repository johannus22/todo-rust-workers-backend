use worker::*;

pub fn get_headers() -> Result<Headers> {
    let headers = Headers::new();
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set("Access-Control-Allow-Methods", "GET, POST, PATCH, DELETE, OPTIONS")?;
    headers.set("Access-Control-Allow-Headers", "Content-Type, Authorization")?;
    headers.set("Access-Control-Max-Age", "86400")?; // 24 hours
    Ok(headers)
}

pub fn add_headers(mut res: Response) -> Result<Response> {
    let cors_headers = get_headers()?;
    for (key, value) in cors_headers.entries() {
        res.headers_mut().set(&key, &value)?;
    }
    Ok(res)
}

pub fn handle_preflight() -> Result<Response> {
    let mut res = Response::ok("")?;
    let cors_headers = get_headers()?;
    for (key, value) in cors_headers.entries() {
        res.headers_mut().set(&key, &value)?;
    }
    Ok(res)
}
