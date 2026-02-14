import Foundation

/// Internal HTTP helper using URLSession.
/// WHY: Zero external dependencies — Foundation is built-in.
final class HttpHelper: @unchecked Sendable {
    let config: EdgeQuakeConfig
    private let session: URLSession
    private let decoder: JSONDecoder
    private let encoder: JSONEncoder

    init(config: EdgeQuakeConfig) {
        self.config = config
        let urlConfig = URLSessionConfiguration.default
        urlConfig.timeoutIntervalForRequest = config.timeoutSeconds
        self.session = URLSession(configuration: urlConfig)
        self.decoder = JSONDecoder()
        self.decoder.keyDecodingStrategy = .convertFromSnakeCase
        self.encoder = JSONEncoder()
        self.encoder.keyEncodingStrategy = .convertToSnakeCase
    }

    /// Internal initializer for testing with a custom URLSession.
    init(config: EdgeQuakeConfig, session: URLSession) {
        self.config = config
        self.session = session
        self.decoder = JSONDecoder()
        self.decoder.keyDecodingStrategy = .convertFromSnakeCase
        self.encoder = JSONEncoder()
        self.encoder.keyEncodingStrategy = .convertToSnakeCase
    }

    // MARK: - Public API

    func get<T: Decodable>(_ path: String) async throws -> T {
        try await execute(buildRequest(path: path, method: "GET"))
    }

    func post<T: Decodable>(_ path: String, body: (any Encodable)? = nil) async throws -> T {
        try await execute(buildRequest(path: path, method: "POST", body: body))
    }

    func delete<T: Decodable>(_ path: String) async throws -> T {
        try await execute(buildRequest(path: path, method: "DELETE"))
    }

    // OODA-35: Added PUT and PATCH methods for complete API coverage
    func put<T: Decodable>(_ path: String, body: (any Encodable)? = nil) async throws -> T {
        try await execute(buildRequest(path: path, method: "PUT", body: body))
    }

    func patch<T: Decodable>(_ path: String, body: (any Encodable)? = nil) async throws -> T {
        try await execute(buildRequest(path: path, method: "PATCH", body: body))
    }

    func putRaw(_ path: String, body: (any Encodable)? = nil) async throws -> Data {
        try await executeRaw(buildRequest(path: path, method: "PUT", body: body))
    }

    func patchRaw(_ path: String, body: (any Encodable)? = nil) async throws -> Data {
        try await executeRaw(buildRequest(path: path, method: "PATCH", body: body))
    }

    func getRaw(_ path: String) async throws -> Data {
        try await executeRaw(buildRequest(path: path, method: "GET"))
    }

    func postRaw(_ path: String, body: (any Encodable)? = nil) async throws -> Data {
        try await executeRaw(buildRequest(path: path, method: "POST", body: body))
    }

    func deleteRaw(_ path: String) async throws -> Data {
        try await executeRaw(buildRequest(path: path, method: "DELETE"))
    }

    func decodeJSON<T: Decodable>(_ type: T.Type, from data: Data) throws -> T {
        try decoder.decode(type, from: data)
    }

    // MARK: - Internals

    private func buildRequest(path: String, method: String, body: (any Encodable)? = nil)
        -> URLRequest
    {
        let url = URL(string: "\(config.baseUrl)\(path)")!
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.timeoutInterval = config.timeoutSeconds

        if let key = config.apiKey { request.setValue(key, forHTTPHeaderField: "X-API-Key") }
        if let tid = config.tenantId { request.setValue(tid, forHTTPHeaderField: "X-Tenant-ID") }
        if let uid = config.userId { request.setValue(uid, forHTTPHeaderField: "X-User-ID") }
        if let wid = config.workspaceId {
            request.setValue(wid, forHTTPHeaderField: "X-Workspace-ID")
        }

        if let body = body {
            request.httpBody = try? encoder.encode(AnyEncodable(body))
        } else if method == "POST" || method == "PUT" || method == "PATCH" {
            request.httpBody = "{}".data(using: .utf8)
        }

        return request
    }

    private func execute<T: Decodable>(_ request: URLRequest) async throws -> T {
        let data = try await executeRaw(request)
        do {
            return try decoder.decode(T.self, from: data)
        } catch {
            let bodyStr = String(data: data, encoding: .utf8) ?? "<binary>"
            throw EdgeQuakeError(
                message:
                    "JSON decode failed: \(error.localizedDescription). Body: \(bodyStr.prefix(200))",
                statusCode: 0, responseBody: bodyStr
            )
        }
    }

    private func executeRaw(_ request: URLRequest) async throws -> Data {
        let (data, response) = try await session.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse else {
            throw EdgeQuakeError(message: "Not an HTTP response")
        }
        guard (200...299).contains(httpResponse.statusCode) else {
            let body = String(data: data, encoding: .utf8)
            throw EdgeQuakeError(
                message: "HTTP \(httpResponse.statusCode): \(body ?? "")",
                statusCode: httpResponse.statusCode, responseBody: body
            )
        }
        return data
    }
}

// MARK: - Type erasure for Encodable

private struct AnyEncodable: Encodable {
    private let _encode: (Encoder) throws -> Void

    init(_ base: any Encodable) {
        _encode = { encoder in try base.encode(to: encoder) }
    }

    func encode(to encoder: Encoder) throws {
        try _encode(encoder)
    }
}
