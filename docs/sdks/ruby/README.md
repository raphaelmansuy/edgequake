# Ruby SDK

**Location:** `sdks/ruby`

## Status

The gem specification references `lib/**/*.rb`, but this checkout may **not** include a complete `lib/` tree. Treat Ruby as **experimental** until the package layout and CI are restored.

## Intended usage (when the gem is complete)

```ruby
require "edgequake"

client = EdgeQuake::Client.new(
  base_url: ENV.fetch("EDGEQUAKE_BASE_URL", "http://localhost:8080"),
  api_key: ENV["EDGEQUAKE_API_KEY"]
)

client.health.check
```

## Verify before production

1. Confirm `lib/` exists and loads.  
2. Run `bundle exec rake test`.  
3. Compare request bodies for conversation bulk ops with `routes.rs` (`conversation_ids`, `affected`).

See [Brutal assessment](../BRUTAL-ASSESSMENT.md).
