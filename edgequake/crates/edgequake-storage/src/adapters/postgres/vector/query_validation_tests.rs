use super::{PgVectorStorage, PostgresConfig};
use crate::{
    traits::{MetadataFilter, VectorStorage},
    StorageError,
};

#[tokio::test]
async fn mismatched_query_dimensions_fail_before_database_access() {
    let storage = PgVectorStorage::with_dimension(PostgresConfig::default(), 3);
    let filter = MetadataFilter {
        workspace_id: Some(uuid::Uuid::new_v4().to_string()),
        vector_type: Some("chunk".into()),
        ..Default::default()
    };
    for embedding in [vec![], vec![0.1; 2], vec![0.1; 1536]] {
        assert!(matches!(
            storage.query(&embedding, 5, None).await,
            Err(StorageError::InvalidQuery(_))
        ));
        for metadata in [None, Some(&filter)] {
            assert!(matches!(
                storage.query_filtered(&embedding, 5, None, metadata).await,
                Err(StorageError::InvalidQuery(_))
            ));
        }
    }
    assert!(storage.validate_query_dimension(&[0.1; 3]).is_ok());
}
