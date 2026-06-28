//! Standalone AsyncAPI 2.6 document for WebSocket progress channels (SPEC-027 OAS-008 A++).

use serde_json::{json, Value};

/// Build the canonical AsyncAPI document served at `/api-docs/asyncapi.json`.
pub fn asyncapi_document() -> Value {
    json!({
        "asyncapi": "2.6.0",
        "info": {
            "title": "EdgeQuake WebSocket Progress",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Real-time pipeline and document processing progress over WebSocket (RFC 6455)."
        },
        "defaultContentType": "application/json",
        "servers": {
            "local": {
                "url": "ws://localhost:8080",
                "protocol": "ws",
                "description": "Local development backend"
            }
        },
        "channels": {
            "/ws/pipeline/progress": {
                "description": "Global pipeline progress stream (all tracks in workspace context)",
                "bindings": {
                    "ws": {
                        "method": "GET",
                        "headers": {
                            "type": "object",
                            "properties": {
                                "Authorization": {
                                    "type": "string",
                                    "description": "Bearer JWT or X-API-Key when auth enabled"
                                }
                            }
                        }
                    }
                },
                "subscribe": {
                    "message": {
                        "name": "ProgressEvent",
                        "payload": {
                            "type": "object",
                            "properties": {
                                "track_id": { "type": "string", "format": "uuid" },
                                "phase": { "type": "string" },
                                "progress": { "type": "number", "minimum": 0, "maximum": 100 },
                                "message": { "type": "string" }
                            },
                            "example": {
                                "track_id": "f6fa9cad-bbff-4892-a855-3bd7d70da044",
                                "phase": "entity_extraction",
                                "progress": 42.5,
                                "message": "Extracting entities from chunk 3/10"
                            }
                        }
                    }
                }
            },
            "/ws/progress/{track_id}": {
                "description": "Per-track document processing progress",
                "parameters": {
                    "track_id": {
                        "description": "Document track UUID",
                        "schema": { "type": "string", "format": "uuid" }
                    }
                },
                "bindings": {
                    "ws": { "method": "GET" }
                },
                "subscribe": {
                    "message": {
                        "name": "TrackProgressEvent",
                        "payload": {
                            "type": "object",
                            "example": {
                                "track_id": "f6fa9cad-bbff-4892-a855-3bd7d70da044",
                                "phase": "completed",
                                "progress": 100.0,
                                "message": "Processing complete"
                            }
                        }
                    }
                }
            }
        }
    })
}

/// Sidecar fragment embedded in OpenAPI root (`x-edgequake-asyncapi`).
pub fn asyncapi_sidecar() -> Value {
    asyncapi_document()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_has_required_channels() {
        let doc = asyncapi_document();
        assert_eq!(doc["asyncapi"], "2.6.0");
        assert!(doc["channels"]["/ws/pipeline/progress"].is_object());
        assert!(doc["channels"]["/ws/progress/{track_id}"].is_object());
    }
}
