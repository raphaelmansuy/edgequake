# EdgeQuake Ruby SDK

Official Ruby client for the [EdgeQuake](https://github.com/edgequake/edgequake) RAG API.

## Requirements

- Ruby ≥ 3.0
- No external dependencies (uses stdlib `net/http`)

## Installation

Add to your `Gemfile`:

```ruby
gem "edgequake"
```

Or install directly:

```bash
gem install edgequake
```

## Quick Start

```ruby
require "edgequake"

client = EdgeQuake::Client.new(config: EdgeQuake::Config.new(
  base_url: "http://localhost:8080",
  api_key:  "sk-your-key",
))

# Health check
health = client.health.check
puts health["status"]  # => "healthy"

# Upload a document
doc = client.documents.upload_text(title: "My Doc", content: "Hello, world!")
puts doc["id"]

# Query the knowledge graph
result = client.query.execute(query: "What do you know?", mode: "hybrid")
puts result["answer"]
```

## Configuration

| Parameter      | Type      | Default                 | Description           |
| -------------- | --------- | ----------------------- | --------------------- |
| `base_url`     | `String`  | `http://localhost:8080` | API base URL          |
| `api_key`      | `String`  | `nil`                   | API key               |
| `tenant_id`    | `String`  | `nil`                   | Tenant ID header      |
| `user_id`      | `String`  | `nil`                   | User ID header        |
| `workspace_id` | `String`  | `nil`                   | Workspace ID header   |
| `timeout`      | `Integer` | `60`                    | Request timeout (sec) |

```ruby
config = EdgeQuake::Config.new(
  base_url:     "https://api.example.com",
  api_key:      "sk-my-key",
  tenant_id:    "tenant-1",
  user_id:      "user-1",
  workspace_id: "ws-1",
  timeout:      120,
)
client = EdgeQuake::Client.new(config: config)
```

## Services

### Health

```ruby
status = client.health.check
# => { "status" => "healthy", "version" => "0.1.0", ... }
```

### Documents

```ruby
# List documents
docs = client.documents.list(page: 1, page_size: 20)

# Get a document
doc = client.documents.get(id: "doc-id")

# Upload text
doc = client.documents.upload_text(title: "Title", content: "Body", file_type: "txt")

# Delete
client.documents.delete(id: "doc-id")
```

### Entities

```ruby
# List entities
entities = client.entities.list(page: 1, page_size: 20)

# Get an entity
entity = client.entities.get(name: "ENTITY_NAME")

# Create
client.entities.create(
  entity_name: "NODE", entity_type: "concept",
  description: "A concept node", source_id: "src-1"
)

# Check existence
client.entities.exists?(name: "NODE")

# Delete
client.entities.delete(name: "NODE")
```

### Relationships

```ruby
rels = client.relationships.list(page: 1, page_size: 20)
```

### Graph

```ruby
# Full graph
graph = client.graph.get

# Search nodes
results = client.graph.search(query: "Alice")
```

### Query

```ruby
result = client.query.execute(query: "What is EdgeQuake?", mode: "hybrid")
puts result["answer"]
```

Modes: `hybrid`, `local`, `global`, `naive`.

### Chat

```ruby
response = client.chat.completions(message: "Hello!", mode: "hybrid", stream: false)
puts response["choices"][0]["message"]["content"]
```

### Tenants

```ruby
tenants = client.tenants.list
```

### Users

```ruby
users = client.users.list
```

### API Keys

```ruby
keys = client.api_keys.list
```

### Tasks

```ruby
tasks = client.tasks.list
```

### Pipeline

```ruby
status  = client.pipeline.status
metrics = client.pipeline.queue_metrics
```

### Models

```ruby
catalog  = client.models.catalog
health   = client.models.health
provider = client.models.provider_status
```

### Costs

```ruby
costs = client.costs.summary
puts costs["total_cost_usd"]
```

## Error Handling

All API errors raise `EdgeQuake::ApiError` (subclass of `StandardError`):

```ruby
begin
  client.documents.get(id: "nonexistent")
rescue EdgeQuake::ApiError => e
  puts e.message        # "HTTP 404: ..."
  puts e.status_code    # 404
  puts e.response_body  # raw JSON string
end
```

| Property        | Type      | Description       |
| --------------- | --------- | ----------------- |
| `status_code`   | `Integer` | HTTP status code  |
| `response_body` | `String`  | Raw response body |
| `message`       | `String`  | Error description |

## Testing

```bash
bundle install
ruby -Ilib -Itest test/unit_test.rb
```

## Project Structure

```
lib/
├── edgequake.rb             # Entry point (requires all modules)
└── edgequake/
    ├── client.rb            # Main client with service accessors
    ├── config.rb            # Configuration
    ├── error.rb             # ApiError class
    ├── http_helper.rb       # Net::HTTP helper
    └── services.rb          # 14 service classes
test/
├── e2e_test.rb              # Integration tests (needs running server)
├── mock_http_helper.rb      # Mock for unit testing
└── unit_test.rb             # 59 unit tests, 120 assertions
```

## License

Apache-2.0
