---
title: Ruby SDK
description: Use the EdgeQuake Ruby gem edgequake 0.4.0 from the monorepo. Requires Ruby 3.0+. Not on RubyGems yet.
---

# Ruby SDK

Ruby client for EdgeQuake. Gem name: **`edgequake`** version **0.4.0**. Requires **Ruby 3.0+**. **Not published** on RubyGems; path-install from `sdks/ruby`.

```bash
# Gemfile
gem "edgequake", path: "sdks/ruby"
```

```ruby
require "edgequake"

client = EdgeQuake::Client.new(
  base_url: "http://localhost:8080",
  api_key: "eq-..."
)

docs = client.documents.list(page: 1, page_size: 20)
answer = client.query.execute(query: "What is in my documents?", mode: "mix")
puts answer
```

Class: `EdgeQuake::Client`. Connections are not wrapped. See [SDK overview](../README.md).
