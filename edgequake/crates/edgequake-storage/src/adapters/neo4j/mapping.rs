//! Provider-neutral graph revisions and their Neo4j edge-as-node encoding.

use std::collections::HashMap;

use edgequake_storage_contracts::{
    AccessError, AccessResult, AccessScope, EdgeDirection, GraphEdgeKey, GraphNodeKey,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

/// Canonical payload retained in `object_revisions.payload` for graph facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GraphObjectPayload {
    Entity {
        logical_id: GraphNodeKey,
        #[serde(default)]
        properties: Map<String, Value>,
    },
    Edge {
        logical_id: GraphEdgeKey,
        source: GraphNodeKey,
        target: GraphNodeKey,
        relationship_type: String,
        direction: EdgeDirection,
        #[serde(default)]
        properties: Map<String, Value>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityRevision {
    pub scope: AccessScope,
    pub logical_id: GraphNodeKey,
    pub physical_id: Uuid,
    pub revision: u64,
    pub digest: [u8; 32],
    pub contribution_manifest: Vec<Uuid>,
    pub properties: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeRevision {
    pub scope: AccessScope,
    pub logical_id: GraphEdgeKey,
    pub physical_id: Uuid,
    pub source: GraphNodeKey,
    pub target: GraphNodeKey,
    pub relationship_type: String,
    pub direction: EdgeDirection,
    pub revision: u64,
    pub digest: [u8; 32],
    pub contribution_manifest: Vec<Uuid>,
    pub properties: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct EntityRevisionRow {
    tenant_id: String,
    workspace_id: String,
    logical_id: String,
    physical_id: String,
    revision: u64,
    digest: String,
    contribution_manifest: Vec<String>,
    properties_json: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct EdgeRevisionRow {
    tenant_id: String,
    workspace_id: String,
    logical_id: String,
    physical_id: String,
    source_id: String,
    target_id: String,
    relationship_type: String,
    direction: &'static str,
    revision: u64,
    digest: String,
    contribution_manifest: Vec<String>,
    properties_json: String,
}

impl EntityRevision {
    pub(crate) fn to_row(&self) -> AccessResult<EntityRevisionRow> {
        validate_revision(self.revision, &self.contribution_manifest)?;
        Ok(EntityRevisionRow {
            tenant_id: self.scope.tenant().to_string(),
            workspace_id: self.scope.workspace().to_string(),
            logical_id: self.logical_id.to_string(),
            physical_id: self.physical_id.to_string(),
            revision: self.revision,
            digest: encode_digest(&self.digest),
            contribution_manifest: manifest_strings(&self.contribution_manifest),
            properties_json: serde_json::to_string(&self.properties)
                .map_err(|error| AccessError::CorruptData(error.to_string()))?,
        })
    }

    pub(crate) fn from_legacy(
        node_id: &str,
        mut properties: HashMap<String, Value>,
    ) -> AccessResult<Self> {
        let logical_id = parse_uuid_key(node_id, "node logical id").map(GraphNodeKey::new)?;
        let scope = take_scope(&mut properties)?;
        let physical_id = take_uuid(&mut properties, "physical_id")?;
        let revision = take_u64(&mut properties, "revision")?;
        let digest = take_digest(&mut properties)?;
        let contribution_manifest = take_manifest(&mut properties)?;
        Ok(Self {
            scope,
            logical_id,
            physical_id,
            revision,
            digest,
            contribution_manifest,
            properties: properties.into_iter().collect(),
        })
    }
}

impl EdgeRevision {
    pub(crate) fn to_row(&self) -> AccessResult<EdgeRevisionRow> {
        validate_revision(self.revision, &self.contribution_manifest)?;
        if self.relationship_type.trim().is_empty() {
            return Err(AccessError::InvalidInput(
                "relationship_type must not be empty".into(),
            ));
        }
        Ok(EdgeRevisionRow {
            tenant_id: self.scope.tenant().to_string(),
            workspace_id: self.scope.workspace().to_string(),
            logical_id: self.logical_id.to_string(),
            physical_id: self.physical_id.to_string(),
            source_id: self.source.to_string(),
            target_id: self.target.to_string(),
            relationship_type: self.relationship_type.clone(),
            direction: match self.direction {
                EdgeDirection::Directed => "directed",
                EdgeDirection::Undirected => "undirected",
            },
            revision: self.revision,
            digest: encode_digest(&self.digest),
            contribution_manifest: manifest_strings(&self.contribution_manifest),
            properties_json: serde_json::to_string(&self.properties)
                .map_err(|error| AccessError::CorruptData(error.to_string()))?,
        })
    }

    pub(crate) fn from_legacy(
        source: &str,
        target: &str,
        mut properties: HashMap<String, Value>,
    ) -> AccessResult<Self> {
        let source = parse_uuid_key(source, "edge source").map(GraphNodeKey::new)?;
        let target = parse_uuid_key(target, "edge target").map(GraphNodeKey::new)?;
        let scope = take_scope(&mut properties)?;
        let physical_id = take_uuid(&mut properties, "physical_id")?;
        let logical_id = GraphEdgeKey::new(take_uuid(&mut properties, "logical_id")?);
        let revision = take_u64(&mut properties, "revision")?;
        let digest = take_digest(&mut properties)?;
        let contribution_manifest = take_manifest(&mut properties)?;
        let relationship_type = take_string(&mut properties, "relationship_type")?;
        let direction = match properties
            .remove("direction")
            .and_then(|value| value.as_str().map(str::to_owned))
            .as_deref()
        {
            None | Some("directed") => EdgeDirection::Directed,
            Some("undirected") => EdgeDirection::Undirected,
            Some(value) => {
                return Err(AccessError::InvalidInput(format!(
                    "invalid edge direction '{value}'"
                )))
            }
        };
        Ok(Self {
            scope,
            logical_id,
            physical_id,
            source,
            target,
            relationship_type,
            direction,
            revision,
            digest,
            contribution_manifest,
            properties: properties.into_iter().collect(),
        })
    }
}

pub(crate) fn encode_digest(digest: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

pub(crate) fn decode_digest(value: &str) -> AccessResult<[u8; 32]> {
    if value.len() != 64 {
        return Err(AccessError::CorruptData(
            "Neo4j graph digest is not 64 hexadecimal characters".into(),
        ));
    }
    let mut digest = [0; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        digest[index] = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Ok(digest)
}

fn hex_nibble(value: u8) -> AccessResult<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(AccessError::CorruptData(
            "Neo4j graph digest contains non-hexadecimal data".into(),
        )),
    }
}

fn validate_revision(revision: u64, manifest: &[Uuid]) -> AccessResult<()> {
    if revision == 0 || revision > i64::MAX as u64 {
        return Err(AccessError::InvalidInput(
            "graph revision must be between 1 and i64::MAX".into(),
        ));
    }
    let mut sorted = manifest.to_vec();
    sorted.sort_unstable();
    if sorted.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(AccessError::InvalidInput(
            "contribution manifest contains duplicate IDs".into(),
        ));
    }
    Ok(())
}

fn manifest_strings(manifest: &[Uuid]) -> Vec<String> {
    let mut values: Vec<String> = manifest.iter().map(Uuid::to_string).collect();
    values.sort_unstable();
    values
}

fn parse_uuid_key(value: &str, field: &str) -> AccessResult<Uuid> {
    Uuid::parse_str(value)
        .map_err(|_| AccessError::InvalidInput(format!("{field} must be an application UUID")))
}

fn take_scope(properties: &mut HashMap<String, Value>) -> AccessResult<AccessScope> {
    use edgequake_storage_contracts::{TenantId, WorkspaceId};
    let tenant = take_uuid(properties, "tenant_id")?;
    let workspace = take_uuid(properties, "workspace_id")?;
    Ok(AccessScope::new(
        TenantId::new(tenant),
        WorkspaceId::new(workspace),
    ))
}

fn take_uuid(properties: &mut HashMap<String, Value>, field: &str) -> AccessResult<Uuid> {
    parse_uuid_key(&take_string(properties, field)?, field)
}

fn take_string(properties: &mut HashMap<String, Value>, field: &str) -> AccessResult<String> {
    properties
        .remove(field)
        .and_then(|value| value.as_str().map(str::to_owned))
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AccessError::InvalidInput(format!("missing string property '{field}'")))
}

fn take_u64(properties: &mut HashMap<String, Value>, field: &str) -> AccessResult<u64> {
    properties
        .remove(field)
        .and_then(|value| value.as_u64())
        .ok_or_else(|| AccessError::InvalidInput(format!("missing integer property '{field}'")))
}

fn take_digest(properties: &mut HashMap<String, Value>) -> AccessResult<[u8; 32]> {
    decode_digest(&take_string(properties, "digest")?)
}

fn take_manifest(properties: &mut HashMap<String, Value>) -> AccessResult<Vec<Uuid>> {
    properties
        .remove("contribution_manifest")
        .map(|value| {
            value
                .as_array()
                .ok_or_else(|| {
                    AccessError::InvalidInput(
                        "contribution_manifest must be a UUID string array".into(),
                    )
                })?
                .iter()
                .map(|item| {
                    item.as_str()
                        .ok_or_else(|| {
                            AccessError::InvalidInput(
                                "contribution_manifest must be a UUID string array".into(),
                            )
                        })
                        .and_then(|item| parse_uuid_key(item, "contribution ID"))
                })
                .collect()
        })
        .transpose()
        .map(Option::unwrap_or_default)
}

#[cfg(test)]
mod tests {
    use edgequake_storage_contracts::{TenantId, WorkspaceId};

    use super::*;

    fn scope() -> AccessScope {
        AccessScope::new(
            TenantId::new(Uuid::from_u128(1)),
            WorkspaceId::new(Uuid::from_u128(2)),
        )
    }

    #[test]
    fn edge_as_node_row_uses_application_ids_and_static_direction_property() {
        let edge = EdgeRevision {
            scope: scope(),
            logical_id: GraphEdgeKey::new(Uuid::from_u128(3)),
            physical_id: Uuid::from_u128(4),
            source: GraphNodeKey::new(Uuid::from_u128(5)),
            target: GraphNodeKey::new(Uuid::from_u128(5)),
            relationship_type: "SELF_REFERENCE".into(),
            direction: EdgeDirection::Directed,
            revision: 7,
            digest: [0xab; 32],
            contribution_manifest: vec![Uuid::from_u128(8)],
            properties: Map::new(),
        };

        let row = serde_json::to_value(edge.to_row().unwrap()).unwrap();
        assert_eq!(row["source_id"], row["target_id"]);
        assert_eq!(row["direction"], "directed");
        assert_eq!(row["digest"].as_str().unwrap().len(), 64);
        assert!(row.get("elementId").is_none());
    }

    #[test]
    fn digest_hex_round_trips() {
        let digest = [0x5a; 32];
        assert_eq!(decode_digest(&encode_digest(&digest)).unwrap(), digest);
    }

    #[test]
    fn duplicate_contribution_ids_are_rejected() {
        let id = Uuid::from_u128(9);
        let entity = EntityRevision {
            scope: scope(),
            logical_id: GraphNodeKey::new(Uuid::from_u128(3)),
            physical_id: Uuid::from_u128(4),
            revision: 1,
            digest: [1; 32],
            contribution_manifest: vec![id, id],
            properties: Map::new(),
        };
        assert!(matches!(entity.to_row(), Err(AccessError::InvalidInput(_))));
    }
}
