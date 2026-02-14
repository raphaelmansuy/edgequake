# frozen_string_literal: true

require "net/http"
require "json"
require "uri"

module EdgeQuake
  # Internal HTTP helper using Net::HTTP.
  # WHY: Zero external dependencies — net/http is stdlib.
  class HttpHelper
    def initialize(config)
      @config = config
    end

    def get(path)
      request(:get, path)
    end

    def post(path, body = nil)
      request(:post, path, body)
    end

    def delete(path)
      request(:delete, path)
    end

    def put(path, body = nil)
      request(:put, path, body)
    end

    def patch(path, body = nil)
      request(:patch, path, body)
    end

    def get_raw(path)
      request_raw(:get, path)
    end

    private

    def request(method, path, body = nil)
      data = request_raw(method, path, body)
      JSON.parse(data, symbolize_names: false)
    end

    def request_raw(method, path, body = nil)
      uri = URI("#{@config.base_url}#{path}")
      http = Net::HTTP.new(uri.host, uri.port)
      http.use_ssl = uri.scheme == "https"
      http.read_timeout = @config.timeout
      http.open_timeout = @config.timeout

      req = build_request(method, uri, body)
      resp = http.request(req)

      unless resp.is_a?(Net::HTTPSuccess)
        raise ApiError.new(
          "HTTP #{resp.code}: #{resp.body}",
          status_code: resp.code.to_i,
          response_body: resp.body
        )
      end

      resp.body
    end

    def build_request(method, uri, body)
      klass = case method
              when :get    then Net::HTTP::Get
              when :post   then Net::HTTP::Post
              when :delete then Net::HTTP::Delete
              when :put    then Net::HTTP::Put
              when :patch  then Net::HTTP::Patch
              else raise ArgumentError, "Unknown method: #{method}"
              end

      req = klass.new(uri)
      req["Content-Type"] = "application/json"
      req["Accept"] = "application/json"
      req["X-API-Key"] = @config.api_key if @config.api_key
      req["X-Tenant-ID"] = @config.tenant_id if @config.tenant_id
      req["X-User-ID"] = @config.user_id if @config.user_id
      req["X-Workspace-ID"] = @config.workspace_id if @config.workspace_id

      if body
        req.body = JSON.generate(body)
      elsif %i[post put patch].include?(method)
        req.body = "{}"
      end

      req
    end
  end
end
