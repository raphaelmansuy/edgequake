//! In-memory list pagination — SPEC-027 IMP-020 (DRY SSOT).

/// Pagination metadata for a sliced in-memory collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlicePagination {
    pub page: usize,
    pub page_size: usize,
    pub total: usize,
    pub total_pages: usize,
    pub has_more: bool,
}

/// Paginate an owned vector. `page` is 1-indexed; `page_size` should be clamped by caller.
pub fn paginate_vec<T>(items: Vec<T>, page: usize, page_size: usize) -> (Vec<T>, SlicePagination) {
    let page = page.max(1);
    let page_size = page_size.max(1);
    let total = items.len();
    let total_pages = if total == 0 {
        0
    } else {
        total.div_ceil(page_size)
    };
    let offset = page.saturating_sub(1).saturating_mul(page_size);
    let page_items: Vec<T> = items.into_iter().skip(offset).take(page_size).collect();
    let has_more = total_pages > 0 && page < total_pages;

    (
        page_items,
        SlicePagination {
            page,
            page_size,
            total,
            total_pages,
            has_more,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paginate_first_page() {
        let (page, meta) = paginate_vec((1..=5).collect(), 1, 2);
        assert_eq!(page, vec![1, 2]);
        assert_eq!(meta.total, 5);
        assert_eq!(meta.page, 1);
        assert_eq!(meta.page_size, 2);
        assert_eq!(meta.total_pages, 3);
        assert!(meta.has_more);
    }

    #[test]
    fn paginate_last_page_partial() {
        let (page, meta) = paginate_vec((1..=5).collect(), 3, 2);
        assert_eq!(page, vec![5]);
        assert_eq!(meta.total_pages, 3);
        assert!(!meta.has_more);
    }

    #[test]
    fn paginate_empty() {
        let (page, meta) = paginate_vec(Vec::<i32>::new(), 1, 20);
        assert!(page.is_empty());
        assert_eq!(meta.total, 0);
        assert_eq!(meta.total_pages, 0);
        assert!(!meta.has_more);
    }
}
