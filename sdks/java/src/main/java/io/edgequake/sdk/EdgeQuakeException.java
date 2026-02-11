package io.edgequake.sdk;

/**
 * Exception thrown when the EdgeQuake API returns a non-2xx status.
 */
public class EdgeQuakeException extends RuntimeException {

    private final int statusCode;
    private final String responseBody;

    public EdgeQuakeException(int statusCode, String responseBody) {
        super("EdgeQuake API error " + statusCode + ": " + responseBody);
        this.statusCode = statusCode;
        this.responseBody = responseBody;
    }

    public EdgeQuakeException(String message, Throwable cause) {
        super(message, cause);
        this.statusCode = 0;
        this.responseBody = "";
    }

    public int statusCode() { return statusCode; }
    public String responseBody() { return responseBody; }
}
