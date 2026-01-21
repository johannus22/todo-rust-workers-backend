//! Ory Keto Read and Write API client for Cloudflare Workers.
//!
//! Talks to Keto's HTTP Read (e.g. 4466) and Write (e.g. 4467) APIs in Docker.
//! Keto's DB runs inside Docker; this client does not connect to the DB directly.

use crate::utils::context::AppContext;
use worker::*;

pub struct KetoClient {
    /// Base URL of Keto Read API (e.g. `http://localhost:4466`).
    pub read_url: String,
    /// Base URL of Keto Write API (e.g. `http://localhost:4467`).
    pub write_url: String,
}

/// Subject set for group-based checks: `namespace:object#relation`.
#[derive(Clone, Debug)]
pub struct SubjectSet {
    pub namespace: String,
    pub object: String,
    pub relation: String,
}

/// Params for a permission check.
#[derive(Clone, Debug)]
pub struct CheckParams {
    pub namespace: String,
    pub object: String,
    pub relation: String,
    /// Direct subject ID (e.g. `user:alice`). Use either this or `subject_set`.
    pub subject_id: Option<String>,
    /// Subject set (e.g. group members). Use either this or `subject_id`.
    pub subject_set: Option<SubjectSet>,
    /// Max depth when resolving subject sets. Default in Keto is 5.
    pub max_depth: Option<u32>,
}

/// Params for listing relation tuples.
#[derive(Clone, Debug, Default)]
pub struct ListParams {
    pub namespace: String,
    pub object: Option<String>,
    pub relation: Option<String>,
    pub subject_id: Option<String>,
    /// As `namespace:object#relation`.
    pub subject_set: Option<String>,
    pub page_size: Option<u32>,
    pub page_token: Option<String>,
}

impl KetoClient {
    /// Build client from env. Expects `KETO_READ_URL` and `KETO_WRITE_URL` (or secrets).
    pub fn from_env(ctx: &AppContext) -> Result<Self> {
        let read_url = ctx
            .env
            .var("KETO_READ_URL")
            .or_else(|_| ctx.env.secret("KETO_READ_URL"))
            .map(|v| v.to_string())?;
        let write_url = ctx
            .env
            .var("KETO_WRITE_URL")
            .or_else(|_| ctx.env.secret("KETO_WRITE_URL"))
            .map(|v| v.to_string())?;
        Ok(Self { read_url, write_url })
    }

    fn headers() -> Result<Headers> {
        let h = Headers::new();
        h.set("Content-Type", "application/json")?;
        Ok(h)
    }

    /// Check if a subject has a relation on an object. Uses `/relation-tuples/check`
    /// which returns `{ "allowed": bool }` (does not use HTTP status for deny).
    pub async fn check(&self, p: CheckParams) -> Result<bool> {
        let mut q: Vec<String> = vec![
            format!("namespace={}", p.namespace),
            format!("object={}", p.object),
            format!("relation={}", p.relation),
        ];
        if let Some(s) = &p.subject_id {
            q.push(format!("subject_id={}", s));
        }
        if let Some(ss) = &p.subject_set {
            q.push(format!("subject_set.namespace={}", ss.namespace));
            q.push(format!("subject_set.object={}", ss.object));
            q.push(format!("subject_set.relation={}", ss.relation));
        }
        if let Some(d) = p.max_depth {
            q.push(format!("max_depth={}", d));
        }
        let url = format!("{}/relation-tuples/check?{}", self.read_url, q.join("&"));

        let req = Request::new_with_init(
            &url,
            RequestInit::new()
                .with_method(Method::Get)
                .with_headers(Self::headers()?),
        )?;

        let mut resp = Fetch::Request(req).send().await?;
        let code = resp.status_code();
        let text = resp.text().await?;

        if code != 200 {
            return Err(Error::RustError(format!(
                "Keto check error ({}): {}",
                code, text
            )));
        }

        let json: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| Error::RustError(format!("Keto check json: {}", e)))?;
        Ok(json.get("allowed").and_then(|v| v.as_bool()).unwrap_or(false))
    }

    /// Expand a relation to see all subjects that have it (tree of subject_ids and subject_sets).
    pub async fn expand(
        &self,
        namespace: &str,
        object: &str,
        relation: &str,
        max_depth: Option<u32>,
    ) -> Result<serde_json::Value> {
        let mut q = format!(
            "namespace={}&object={}&relation={}",
            namespace, object, relation
        );
        if let Some(d) = max_depth {
            q.push_str(&format!("&max_depth={}", d));
        }
        let url = format!("{}/relation-tuples/expand?{}", self.read_url, q);

        let req = Request::new_with_init(
            &url,
            RequestInit::new()
                .with_method(Method::Get)
                .with_headers(Self::headers()?),
        )?;

        let mut resp = Fetch::Request(req).send().await?;
        let code = resp.status_code();
        let text = resp.text().await?;

        if code != 200 {
            return Err(Error::RustError(format!(
                "Keto expand error ({}): {}",
                code, text
            )));
        }

        serde_json::from_str(&text).map_err(|e| Error::RustError(format!("Keto expand json: {}", e)))
    }

    /// List relation tuples with optional filters. `namespace` is required.
    pub async fn list_relation_tuples(&self, p: ListParams) -> Result<serde_json::Value> {
        let mut q = format!("namespace={}", p.namespace);
        if let Some(o) = &p.object {
            q.push_str(&format!("&object={}", o));
        }
        if let Some(r) = &p.relation {
            q.push_str(&format!("&relation={}", r));
        }
        if let Some(s) = &p.subject_id {
            q.push_str(&format!("&subject_id={}", s));
        }
        if let Some(ss) = &p.subject_set {
            q.push_str(&format!("&subject_set={}", ss));
        }
        if let Some(n) = p.page_size {
            q.push_str(&format!("&page_size={}", n));
        }
        if let Some(t) = &p.page_token {
            q.push_str(&format!("&page_token={}", t));
        }

        let url = format!("{}/relation-tuples?{}", self.read_url, q);

        let req = Request::new_with_init(
            &url,
            RequestInit::new()
                .with_method(Method::Get)
                .with_headers(Self::headers()?),
        )?;

        let mut resp = Fetch::Request(req).send().await?;
        let code = resp.status_code();
        let text = resp.text().await?;

        if code != 200 {
            return Err(Error::RustError(format!(
                "Keto list error ({}): {}",
                code, text
            )));
        }

        serde_json::from_str(&text).map_err(|e| Error::RustError(format!("Keto list json: {}", e)))
    }

    /// Create a relation tuple via `PUT /admin/relation-tuples`. Idempotent if tuple exists.
    pub async fn create_relation_tuple(
        &self,
        namespace: &str,
        object: &str,
        relation: &str,
        subject_id: &str,
    ) -> Result<()> {
        let url = format!("{}/admin/relation-tuples", self.write_url);
        let body = serde_json::json!({
            "namespace": namespace,
            "object": object,
            "relation": relation,
            "subject_id": subject_id,
        });

        let req = Request::new_with_init(
            &url,
            RequestInit::new()
                .with_method(Method::Put)
                .with_headers(Self::headers()?)
                .with_body(Some(body.to_string().into())),
        )?;

        let mut resp = Fetch::Request(req).send().await?;
        let code = resp.status_code();
        if code != 201 && code != 200 && code != 409 {
            let text = resp.text().await?;
            return Err(Error::RustError(format!(
                "Keto create tuple error ({}): {}",
                code, text
            )));
        }
        Ok(())
    }

    /// Delete relation tuples matching the filters via `DELETE /admin/relation-tuples`.
    pub async fn delete_relation_tuple(
        &self,
        namespace: &str,
        object: &str,
        relation: &str,
        subject_id: &str,
    ) -> Result<()> {
        let q = format!(
            "namespace={}&object={}&relation={}&subject_id={}",
            namespace, object, relation, subject_id
        );
        let url = format!("{}/admin/relation-tuples?{}", self.write_url, q);

        let req = Request::new_with_init(
            &url,
            RequestInit::new()
                .with_method(Method::Delete)
                .with_headers(Self::headers()?),
        )?;

        let mut resp = Fetch::Request(req).send().await?;
        let code = resp.status_code();
        if code != 200 && code != 204 {
            let text = resp.text().await?;
            return Err(Error::RustError(format!(
                "Keto delete tuple error ({}): {}",
                code, text
            )));
        }
        Ok(())
    }
}
