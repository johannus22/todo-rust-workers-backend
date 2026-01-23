//! Minimal Ory Kratos Admin API client for Cloudflare Workers.

use crate::utils::context::AppContext;
use worker::*;

pub struct KratosClient {
    pub admin_url: String,
}

impl KratosClient {
    /// Build client from env. Expects `KRATOS_ADMIN_URL` (or `KRATOS_PUBLIC_URL` as fallback).
    pub fn from_env(ctx: &AppContext) -> Result<Self> {
        let admin_url = ctx
            .env
            .var("KRATOS_ADMIN_URL")
            .or_else(|_| ctx.env.secret("KRATOS_ADMIN_URL"))
            .or_else(|_| ctx.env.var("KRATOS_PUBLIC_URL"))
            .or_else(|_| ctx.env.secret("KRATOS_PUBLIC_URL"))
            .map(|v| v.to_string())?;
        Ok(Self { admin_url })
    }

    fn headers() -> Result<Headers> {
        let h = Headers::new();
        h.set("Content-Type", "application/json")?;
        Ok(h)
    }

    /// Fetch identity JSON by id, trying admin and public routes.
    pub async fn get_identity(&self, id: &str) -> Result<serde_json::Value> {
        let candidates = [
            format!("{}/admin/identities/{}", self.admin_url, id),
            format!("{}/identities/{}", self.admin_url, id),
        ];

        let mut last_error: Option<String> = None;
        for url in candidates {
            let req = Request::new_with_init(
                &url,
                RequestInit::new()
                    .with_method(Method::Get)
                    .with_headers(Self::headers()?),
            )?;

            let mut resp = Fetch::Request(req).send().await?;
            let code = resp.status_code();
            let text = resp.text().await?;

            if code == 200 {
                return serde_json::from_str(&text)
                    .map_err(|e| Error::RustError(format!("Kratos identity json: {}", e)));
            }

            if code != 404 {
                return Err(Error::RustError(format!(
                    "Kratos identity error ({}): {}",
                    code, text
                )));
            }

            last_error = Some(format!("{} -> {}", url, text));
        }

        Err(Error::RustError(format!(
            "Kratos identity error (404): no identity endpoint found; last: {}",
            last_error.unwrap_or_else(|| "none".to_string())
        )))
    }
}
