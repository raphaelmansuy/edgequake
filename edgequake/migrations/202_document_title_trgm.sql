-- The scope picker reads the newest documents and drops titles in memory.
-- On 80,000 documents, the 50 oldest titles that contain `uniquezephyr`
-- are not in that page. Requiring `lower(title) LIKE` on
-- `idx_documents_created_at` still removes those 79,950 newer rows
-- (38.139 ms warm, 2,557 buffers).
--
-- `idx_documents_title_trgm` is a GIN on `lower(title)`. A custom plan
-- bitmaps it and stops at the 50 matches (0.168 ms, 17 buffers, tenant
-- union). A one-letter query that matches every title stays on the time
-- index and stops at LIMIT (0.104 ms). A generic plan always starts at
-- the GIN and reads all 80,000 titles for that letter (46.237 ms), so
-- the statement uses `force_custom_plan`.

CREATE INDEX IF NOT EXISTS idx_documents_title_trgm
    ON public.documents USING gin (lower(title) public.gin_trgm_ops);
