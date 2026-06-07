//! RwLock error mapping for in-memory adapters (SPEC-017 STORE-DRY-002).

use crate::error::StorageError;

/// Map poisoned lock errors to a consistent storage error message.
pub fn map_lock_err<T>(err: std::sync::PoisonError<T>) -> StorageError {
    StorageError::Database(format!("Lock error: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, RwLock};

    #[test]
    fn map_lock_err_formats_poison_message() {
        let lock = Arc::new(RwLock::new(0i32));
        let lock2 = Arc::clone(&lock);
        let handle = std::thread::spawn(move || {
            let _guard = lock2.write().unwrap();
            panic!("intentional poison");
        });
        let _ = handle.join();

        let err = lock.read().map_err(map_lock_err).unwrap_err();
        assert!(err.to_string().contains("Lock error:"));
    }
}
