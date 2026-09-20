//! Provider-independent UUID identity types.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

macro_rules! uuid_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub const fn new(value: Uuid) -> Self {
                Self(value)
            }

            pub const fn as_uuid(&self) -> &Uuid {
                &self.0
            }

            pub const fn into_uuid(self) -> Uuid {
                self.0
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }

        impl From<$name> for Uuid {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

uuid_id!(
    /// Tenant identity at the data-access boundary.
    TenantId
);
uuid_id!(
    /// Workspace identity at the data-access boundary.
    WorkspaceId
);
uuid_id!(
    /// Canonical document identity.
    DocumentId
);
uuid_id!(
    /// Persisted graph node transport identity.
    GraphNodeKey
);
uuid_id!(
    /// Persisted graph edge transport identity.
    GraphEdgeKey
);
uuid_id!(
    /// Persisted embedding transport identity.
    EmbeddingKey
);
