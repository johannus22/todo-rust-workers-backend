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

    /// Check if a subject has a relation on an object. Uses `/relation-tuples/check/openapi`
    /// which returns `{ "allowed": bool }` with HTTP 200 (avoids 403/404 on deny).
    pub async fn check(&self, p: CheckParams) -> Result<bool> {
        let mut body = serde_json::json!({
            "namespace": p.namespace,
            "object": p.object,
            "relation": p.relation,
        });

        if let Some(s) = &p.subject_id {
            body["subject_id"] = serde_json::Value::String(s.clone());
        }
        if let Some(ss) = &p.subject_set {
            body["subject_set"] = serde_json::json!({
                "namespace": ss.namespace,
                "object": ss.object,
                "relation": ss.relation,
            });
        }
        if let Some(d) = p.max_depth {
            body["max_depth"] = serde_json::Value::Number(d.into());
        }

        let candidates = [
            format!("{}/relation-tuples/check/openapi", self.read_url),
            format!("{}/relation-tuples/check", self.read_url),
            format!("{}/v1/relation-tuples/check/openapi", self.read_url),
            format!("{}/v1/relation-tuples/check", self.read_url),
        ];

        let mut last_error: Option<String> = None;
        for url in candidates {
            let req = Request::new_with_init(
                &url,
                RequestInit::new()
                    .with_method(Method::Post)
                    .with_headers(Self::headers()?)
                    .with_body(Some(body.to_string().into())),
            )?;

            let mut resp = Fetch::Request(req).send().await?;
            let code = resp.status_code();
            let text = resp.text().await?;

            if code == 200 {
                let json: serde_json::Value = serde_json::from_str(&text)
                    .map_err(|e| Error::RustError(format!("Keto check json: {}", e)))?;
                return Ok(json
                    .get("allowed")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false));
            }

            if code != 404 {
                return Err(Error::RustError(format!(
                    "Keto check error ({}): {}",
                    code, text
                )));
            }

            last_error = Some(format!("{} -> {}", url, text));
        }

        // Fallback for older/variant APIs: use list with exact filters.
        let list = self
            .list_relation_tuples(ListParams {
                namespace: p.namespace,
                object: Some(p.object),
                relation: Some(p.relation),
                subject_id: p.subject_id,
                subject_set: p.subject_set.map(|ss| {
                    if ss.relation.is_empty() {
                        format!("{}:{}", ss.namespace, ss.object)
                    } else {
                        format!("{}:{}#{}", ss.namespace, ss.object, ss.relation)
                    }
                }),
                page_size: Some(1),
                page_token: None,
            })
            .await;

        if let Ok(json) = list {
            let allowed = json
                .get("relation_tuples")
                .and_then(|v| v.as_array())
                .map(|a| !a.is_empty())
                .unwrap_or(false);
            return Ok(allowed);
        }

        Err(Error::RustError(format!(
            "Keto check error (404): no check endpoint found; last: {}",
            last_error.unwrap_or_else(|| "none".to_string())
        )))
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

    /// Create a relation tuple via `PUT /relation-tuples` on the Write API. Idempotent if tuple exists.
    pub async fn create_relation_tuple(
        &self,
        namespace: &str,
        object: &str,
        relation: &str,
        subject_id: &str,
    ) -> Result<()> {
        let url = format!("{}/relation-tuples", self.write_url);
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

    /// Delete relation tuples matching the filters via `DELETE /relation-tuples` on the Write API.
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
        let url = format!("{}/relation-tuples?{}", self.write_url, q);

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
