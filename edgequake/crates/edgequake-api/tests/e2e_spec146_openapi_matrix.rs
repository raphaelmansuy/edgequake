//! SPEC-146 G-146-51 — OpenAPI documents 401/403/404 for document detail + authz routes.
//!
//! Run: `cargo test -p edgequake-api --test e2e_spec146_openapi_matrix`

use edgequake_api::openapi::ApiDoc;
use serde_json::Value;
use utoipa::OpenApi;

fn openapi_json() -> Value {
    serde_json::to_value(ApiDoc::openapi()).expect("serialize OpenAPI")
}

/// JSON Pointer escape for path keys containing `/` (`~1`).
fn pointer_escape(path: &str) -> String {
    path.replace('~', "~0").replace('/', "~1")
}

fn path_responses<'a>(doc: &'a Value, path: &str, method: &str) -> Option<&'a Value> {
    let key = pointer_escape(path);
    doc.pointer(&format!("/paths/{key}/{method}/responses"))
}

fn assert_status_codes(responses: &Value, codes: &[&str], label: &str) {
    for code in codes {
        assert!(
            responses.get(*code).is_some(),
            "OpenAPI {label} missing response status {code}; have keys: {:?}",
            responses
                .as_object()
                .map(|m| m.keys().cloned().collect::<Vec<_>>())
        );
    }
}

#[test]
fn g146_51_document_detail_documents_401_403_404() {
    let doc = openapi_json();
    let responses = path_responses(&doc, "/api/v1/documents/{document_id}", "get")
        .unwrap_or_else(|| {
            let keys: Vec<_> = doc["paths"]
                .as_object()
                .map(|m| m.keys().cloned().collect())
                .unwrap_or_default();
            panic!("GET document detail missing from OpenAPI; paths sample: {keys:?}");
        });
    assert_status_codes(responses, &["401", "403", "404"], "document detail");
}

#[test]
fn g146_51_authz_break_glass_documents_401_403_404() {
    let doc = openapi_json();
    let list = path_responses(
        &doc,
        "/api/v1/workspaces/{workspace_id}/authz/break-glass",
        "get",
    )
    .expect("list break-glass path missing");
    assert_status_codes(list, &["401", "403", "404"], "break-glass list");

    let create = path_responses(
        &doc,
        "/api/v1/workspaces/{workspace_id}/authz/break-glass",
        "post",
    )
    .expect("create break-glass path missing");
    assert_status_codes(create, &["401", "403", "404"], "break-glass create");

    let revoke = path_responses(
        &doc,
        "/api/v1/workspaces/{workspace_id}/authz/break-glass/{session_id}",
        "delete",
    )
    .expect("revoke break-glass path missing");
    assert_status_codes(revoke, &["401", "403", "404"], "break-glass revoke");
}

#[test]
fn g146_51_authz_members_roles_policies_matrix() {
    let doc = openapi_json();
    for (path, method) in [
        (
            "/api/v1/workspaces/{workspace_id}/authz/members",
            "get",
        ),
        ("/api/v1/workspaces/{workspace_id}/authz/roles", "get"),
        (
            "/api/v1/workspaces/{workspace_id}/authz/policies",
            "get",
        ),
    ] {
        let responses =
            path_responses(&doc, path, method).unwrap_or_else(|| panic!("missing OpenAPI {path}"));
        assert_status_codes(responses, &["401", "403", "404"], path);
    }
}

#[test]
fn g146_51_patch_security_labels_documents_401_403_404() {
    let doc = openapi_json();
    let responses = path_responses(
        &doc,
        "/api/v1/documents/{document_id}/security-labels",
        "patch",
    )
    .expect("PATCH security-labels path missing");
    assert_status_codes(responses, &["401", "403", "404"], "PATCH security-labels");
}
