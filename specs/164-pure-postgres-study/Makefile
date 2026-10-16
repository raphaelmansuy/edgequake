# SPEC-164 proof driver
.PHONY: proof proof-inventory harness-load

proof:
	./measurements/run_bench.sh

proof-inventory:
	./measurements/run_inventory_bench.sh

harness-load:
	docker cp measurements/harness.sql $${SPEC164_PG_CONTAINER:-eqdoc-scratch}:/tmp/spec164_harness.sql
	docker exec $${SPEC164_PG_CONTAINER:-eqdoc-scratch} \
	  psql -U $${SPEC164_PG_USER:-edgequake} -d $${SPEC164_PG_DB:-edgequake} \
	  -v ON_ERROR_STOP=1 -f /tmp/spec164_harness.sql
