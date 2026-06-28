Voici le rappel du message relatif à EdgeQuake dans le menu :
https://novagen.talkspirit.com/#/l/chat/eb8d267346f5bb26b81a49c9ed102824

le doc de test :

Le log (large )

edgequake | 2026-04-29T11:13:39.112020Z ERROR edgequake_api::processor::text_insert: CRITICAL: Pipeline processing failed - document marked as failed document_id=de72be47-c094-4a2d-a3f1-f093b83237ce workspace_id=Some("b7319190-1209-4812-927f-06015bbbcb7f") tenant_id=Some("eac0f49d-c64d-4362-8089-d9d3862d12f6") content_length=231764 error=Embedding error: API error: Mistral embeddings API error (400 Bad Request): {"object":"error","message":"Too many inputs in request, split into more batches.","type":"invalid_request_prompt","param":null,"code":"3210","raw_status_code":400}
edgequake | 2026-04-29T11:13:39.112373Z DEBUG sqlx::query: summary="SELECT value FROM public.eq_eq_default_kv …" db.statement="\n\nSELECT value FROM public.eq_eq_default_kv WHERE key = $1\n" rows_affected=1 rows_returned=1 elapsed=145.679µs elapsed_secs=0.000145679
edgequake | 2026-04-29T11:13:39.118528Z DEBUG sqlx::query: summary="INSERT INTO public.eq_eq_default_kv (key, …" db.statement="\n\n\n INSERT INTO public.eq_eq_default_kv (key, value, updated_at)\n VALUES ($1, $2, NOW())\n ON CONFLICT (key) DO UPDATE SET\n value = EXCLUDED.value,\n updated_at = NOW()\n \n" rows_affected=1 rows_returned=0 elapsed=6.026369ms elapsed_secs=0.006026369
edgequake | 2026-04-29T11:13:39.120937Z ERROR edgequake_tasks::worker: Task processing failed worker_id=0 task_id=insert-5ac7ffc7-2213-4b94-a584-bb7c1835a749 tenant_id=eac0f49d-c64d-4362-8089-d9d3862d12f6 retry_count=1 max_retries=3 consecutive_timeouts=0 error=Processing error: Pipeline processing failed: Embedding error: API error: Mistral embeddings API error (400 Bad Request): {"object":"error","message":"Too many inputs in request, split into more batches.","type":"invalid_request_prompt","param":null,"code":"3210","raw_status_code":400}