error id: file://<WORKSPACE>/sdks/java/src/main/java/io/edgequake/sdk/models/ReadinessResponse.java:com/fasterxml/jackson/annotation/JsonProperty#
file://<WORKSPACE>/sdks/java/src/main/java/io/edgequake/sdk/models/ReadinessResponse.java
empty definition using pc, found symbol in pc: com/fasterxml/jackson/annotation/JsonProperty#
empty definition using semanticdb
empty definition using fallback
non-local guesses:

offset: 74
uri: file://<WORKSPACE>/sdks/java/src/main/java/io/edgequake/sdk/models/ReadinessResponse.java
text:
```scala
package io.edgequake.sdk.models;

import com.fasterxml.jackson.annotation.@@JsonProperty;
import java.util.Map;

/** ReadinessResponse from GET /ready. */
public class ReadinessResponse {
    @JsonProperty("ready") public boolean ready;
    @JsonProperty("checks") public Map<String, String> checks;
}

```


#### Short summary: 

empty definition using pc, found symbol in pc: com/fasterxml/jackson/annotation/JsonProperty#