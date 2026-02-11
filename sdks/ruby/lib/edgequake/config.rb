# frozen_string_literal: true

module EdgeQuake
  # Configuration for the EdgeQuake client.
  class Config
    attr_accessor :base_url, :api_key, :tenant_id, :user_id, :workspace_id, :timeout

    def initialize(
      base_url: "http://localhost:8080",
      api_key: nil,
      tenant_id: nil,
      user_id: nil,
      workspace_id: nil,
      timeout: 60
    )
      @base_url = base_url.chomp("/")
      @api_key = api_key
      @tenant_id = tenant_id
      @user_id = user_id
      @workspace_id = workspace_id
      @timeout = timeout
    end
  end
end
