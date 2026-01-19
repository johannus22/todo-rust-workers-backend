use crate::utils::context::AppContext;
use worker::*;

pub struct SupabaseClient {
    pub base_url: String,
    pub api_key: String,
}

impl SupabaseClient {
    pub fn from_env(ctx: &AppContext) -> Result<Self> {
        let base_url = ctx.env.var("DB_API_URL")?.to_string();
        let api_key = ctx.env
            .var("DB_API_KEY")
            .or_else(|_| ctx.env.secret("DB_API_KEY"))
            .map(|v| v.to_string())?;
        
        Ok(Self { base_url, api_key })
    }

    pub fn get_headers(&self) -> Result<Headers> {
        let headers = Headers::new();
        headers.set("apikey", &self.api_key)?;
        headers.set("Authorization", &format!("Bearer {}", self.api_key))?;
        headers.set("Content-Type", "application/json")?;
        Ok(headers)
    }

    pub async fn get(&self, table: &str, query: &str) -> Result<serde_json::Value> {
        let url = format!("{}/rest/v1/{}?{}", self.base_url, table, query);
        let headers = self.get_headers()?;
        
        let req = Request::new_with_init(
            &url,
            RequestInit::new()
                .with_method(Method::Get)
                .with_headers(headers),
        )?;
        
        let mut resp = Fetch::Request(req).send().await?;
        if resp.status_code() != 200 {
            let error_text = resp.text().await?;
            return Err(Error::RustError(format!("Supabase error ({}): {}", resp.status_code(), error_text)));
        }
        resp.json().await
    }

    pub async fn post(&self, table: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}/rest/v1/{}", self.base_url, table);
        let headers = self.get_headers()?;
        headers.set("Prefer", "return=representation")?;
        
        let req = Request::new_with_init(
            &url,
            RequestInit::new()
                .with_method(Method::Post)
                .with_headers(headers)
                .with_body(Some(body.to_string().into())),
        )?;
        
        let mut resp = Fetch::Request(req).send().await?;
        if resp.status_code() != 201 && resp.status_code() != 200 {
            let error_text = resp.text().await?;
            return Err(Error::RustError(format!("Supabase error ({}): {}", resp.status_code(), error_text)));
        }
        resp.json().await
    }

    pub async fn patch(&self, table: &str, id: i64, body: serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}/rest/v1/{}?id=eq.{}", self.base_url, table, id);
        let headers = self.get_headers()?;
        headers.set("Prefer", "return=representation")?;
        
        let req = Request::new_with_init(
            &url,
            RequestInit::new()
                .with_method(Method::Patch)
                .with_headers(headers)
                .with_body(Some(body.to_string().into())),
        )?;
        
        let mut resp = Fetch::Request(req).send().await?;
        if resp.status_code() != 200 {
            let error_text = resp.text().await?;
            return Err(Error::RustError(format!("Supabase error ({}): {}", resp.status_code(), error_text)));
        }
        resp.json().await
    }

    pub async fn delete(&self, table: &str, id: i64) -> Result<()> {
        let url = format!("{}/rest/v1/{}?id=eq.{}", self.base_url, table, id);
        let headers = self.get_headers()?;
        
        let req = Request::new_with_init(
            &url,
            RequestInit::new()
                .with_method(Method::Delete)
                .with_headers(headers),
        )?;
        
        let mut resp = Fetch::Request(req).send().await?;
        if resp.status_code() != 200 && resp.status_code() != 204 {
            let error_text = resp.text().await?;
            return Err(Error::RustError(format!("Supabase error ({}): {}", resp.status_code(), error_text)));
        }
        Ok(())
    }
}
