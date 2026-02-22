# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name          = "edgequake"
  spec.version       = "0.4.0"
  spec.authors       = ["EdgeQuake"]
  spec.email         = ["dev@edgequake.io"]
  spec.summary       = "Ruby SDK for the EdgeQuake RAG API"
  spec.description   = "Zero-dependency Ruby client for EdgeQuake REST API with full CRUD support."
  spec.homepage      = "https://github.com/edgequake/edgequake"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.0"

  spec.files = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
