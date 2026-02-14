package io.edgequake.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;

/** LivenessResponse from GET /live. */
public class LivenessResponse {
    @JsonProperty("alive") public boolean alive;
    @JsonProperty("uptime") public Long uptime;
}
