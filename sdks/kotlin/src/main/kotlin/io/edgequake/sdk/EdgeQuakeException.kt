package io.edgequake.sdk

/** WHY: Typed exception with status code for callers to handle HTTP errors. */
class EdgeQuakeException(
    message: String,
    val statusCode: Int = 0,
    val responseBody: String? = null,
    cause: Throwable? = null
) : RuntimeException(message, cause)
