# Iteration 132 – Observe

## Focus: Provider/Model Lineage (Item 15)

### Requirement
> For each assistant message, you must store the provider and model used to generate the message as lineage information in the database. This information must be retrievable via the API and displayed in the webui.

### Current State

Need to verify:
1. Database schema stores provider/model
2. API returns lineage info
3. WebUI displays provider/model in messages
