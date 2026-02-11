import Foundation

/// Typed error for EdgeQuake API calls.
public struct EdgeQuakeError: Error, LocalizedError {
    public let message: String
    public let statusCode: Int
    public let responseBody: String?

    public var errorDescription: String? { message }

    public init(message: String, statusCode: Int = 0, responseBody: String? = nil) {
        self.message = message
        self.statusCode = statusCode
        self.responseBody = responseBody
    }
}
