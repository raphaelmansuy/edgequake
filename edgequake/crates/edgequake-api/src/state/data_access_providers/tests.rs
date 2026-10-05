use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

struct FailingFactory {
    name: String,
    calls: AtomicUsize,
}
impl FailingFactory {
    fn new(name: &str) -> Arc<Self> {
        Arc::new(Self {
            name: name.into(),
            calls: AtomicUsize::new(0),
        })
    }
    fn fail<T>(&self) -> Result<T, StorageError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(StorageError::Connection(
            "selected provider unavailable".into(),
        ))
    }
}
#[async_trait]
impl RelationalProviderFactory for FailingFactory {
    fn provider_name(&self) -> &str {
        &self.name
    }
    async fn build(&self, _: &ProviderContext) -> Result<RelationalProviderRuntime, StorageError> {
        self.fail()
    }
}
#[async_trait]
impl VectorProviderFactory for FailingFactory {
    fn provider_name(&self) -> &str {
        &self.name
    }
    async fn build(&self, _: &ProviderContext) -> Result<VectorProviderRuntime, StorageError> {
        self.fail()
    }
}
#[async_trait]
impl GraphProviderFactory for FailingFactory {
    fn provider_name(&self) -> &str {
        &self.name
    }
    async fn build(&self, _: &ProviderContext) -> Result<GraphProviderRuntime, StorageError> {
        self.fail()
    }
}
fn context() -> ProviderContext {
    ProviderContext {
        namespace: "test".into(),
        embedding_dimension: 3,
        embedding_model: "test-model".into(),
        provision_defaults: false,
    }
}
#[tokio::test]
async fn validates_every_provider_name_before_any_io() {
    for invalid in ["", "Bad Provider", "provider/secret", "é", &"a".repeat(65)] {
        for axis in 0..3 {
            let r = FailingFactory::new(if axis == 0 { invalid } else { "relational" });
            let v = FailingFactory::new(if axis == 1 { invalid } else { "vector" });
            let g = FailingFactory::new(if axis == 2 { invalid } else { "graph" });
            let factories = ProviderFactories {
                relational: r.clone(),
                vector: v.clone(),
                graph: g.clone(),
            };
            let error = match factories.materialize(&context()).await {
                Ok(_) => panic!("invalid name accepted"),
                Err(e) => e,
            };
            assert!(matches!(error, StorageError::InvalidConfig(_)));
            for f in [r, v, g] {
                assert_eq!(f.calls.load(Ordering::SeqCst), 0);
            }
        }
    }
}
#[tokio::test]
async fn selected_provider_failure_propagates_without_fallback_or_later_builds() {
    let r = FailingFactory::new("relational");
    let v = FailingFactory::new("vector");
    let g = FailingFactory::new("graph");
    let factories = ProviderFactories {
        relational: r.clone(),
        vector: v.clone(),
        graph: g.clone(),
    };
    let error = match factories.materialize(&context()).await {
        Ok(_) => panic!("failure ignored"),
        Err(e) => e,
    };
    assert!(error.to_string().contains("selected provider unavailable"));
    assert_eq!(r.calls.load(Ordering::SeqCst), 1);
    assert_eq!(v.calls.load(Ordering::SeqCst), 0);
    assert_eq!(g.calls.load(Ordering::SeqCst), 0);
}
#[test]
fn replacing_one_axis_preserves_the_other_factories() {
    for axis in 0..3 {
        let original = FailingFactory::new("original");
        let replacement = FailingFactory::new("replacement");
        let r: Arc<dyn RelationalProviderFactory> = original.clone();
        let v: Arc<dyn VectorProviderFactory> = original.clone();
        let g: Arc<dyn GraphProviderFactory> = original.clone();
        let factories = ProviderFactories {
            relational: r.clone(),
            vector: v.clone(),
            graph: g.clone(),
        }
        .with_overrides(ProviderOverrides {
            relational: (axis == 0)
                .then(|| replacement.clone() as Arc<dyn RelationalProviderFactory>),
            vector: (axis == 1).then(|| replacement.clone() as Arc<dyn VectorProviderFactory>),
            graph: (axis == 2).then(|| replacement.clone() as Arc<dyn GraphProviderFactory>),
        });
        assert_eq!(Arc::ptr_eq(&r, &factories.relational), axis != 0);
        assert_eq!(Arc::ptr_eq(&v, &factories.vector), axis != 1);
        assert_eq!(Arc::ptr_eq(&g, &factories.graph), axis != 2);
    }
}
