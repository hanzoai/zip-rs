//! Policy-evaluation handler.
//!
//! Demonstrates a realistic shape for an IAM / gateway authz extension:
//! the host hands in a subject + action + context map; the handler
//! returns an allow/deny decision plus a reason. The match logic is
//! intentionally trivial — the point is to show how `#[handler]` lets
//! the logic be written as ordinary, testable Rust.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use zip_rs::{handler, serde_json, Result};

#[derive(Deserialize)]
struct PolicyRequest {
    subject: String,
    action: String,
    #[serde(default)]
    resource: String,
    #[serde(default)]
    context: BTreeMap<String, serde_json::Value>,
}

#[derive(Serialize)]
struct PolicyResponse {
    allow: bool,
    reason: String,
}

#[handler]
fn evaluate(req: PolicyRequest) -> Result<PolicyResponse> {
    if req.subject.is_empty() {
        return Err("subject required".into());
    }
    if req.action.is_empty() {
        return Err("action required".into());
    }

    // Rule 1: superusers can do anything.
    if req.subject == "root" || req.context.get("role").and_then(|v| v.as_str()) == Some("admin") {
        return Ok(PolicyResponse {
            allow: true,
            reason: "admin override".into(),
        });
    }

    // Rule 2: read actions on public resources are allowed.
    if req.action == "read" && req.resource.starts_with("public:") {
        return Ok(PolicyResponse {
            allow: true,
            reason: "public resource".into(),
        });
    }

    // Rule 3: subject must own the resource for any write.
    if matches!(req.action.as_str(), "write" | "update" | "delete") {
        let owner_match = req.resource.starts_with(&format!("{}:", req.subject));
        if owner_match {
            return Ok(PolicyResponse {
                allow: true,
                reason: "owner".into(),
            });
        }
        return Ok(PolicyResponse {
            allow: false,
            reason: "not owner".into(),
        });
    }

    Ok(PolicyResponse {
        allow: false,
        reason: "no rule matched".into(),
    })
}
