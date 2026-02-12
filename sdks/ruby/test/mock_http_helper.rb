# frozen_string_literal: true

require_relative "../lib/edgequake"

# Mock HTTP helper that returns predefined responses without network calls.
# WHY: Enables stateless unit testing of all service methods.
module EdgeQuake
  class MockHttpHelper < HttpHelper
    attr_reader :calls

    def initialize(response = "{}", status = 200)
      super(Config.new)
      @calls = []
      @next_response = response
      @next_status = status
    end

    def will_return(json, status = 200)
      @next_response = json
      @next_status = status
      self
    end

    def last_call
      @calls.last
    end

    private

    def request_raw(method, path, body = nil)
      @calls << { method: method, path: path, body: body }

      if @next_status < 200 || @next_status >= 300
        raise ApiError.new(
          "HTTP #{@next_status}: #{@next_response}",
          status_code: @next_status,
          response_body: @next_response
        )
      end

      @next_response
    end
  end
end
