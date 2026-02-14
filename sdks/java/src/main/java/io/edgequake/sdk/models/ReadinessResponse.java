package io.edgequake.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Map;

/** ReadinessResponse from GET /ready. */
public class ReadinessResponse {
    @JsonProperty("ready") public boolean ready;
    @JsonProperty("checks") public Map<String, String> checks;
}
