package io.edgequake.sdk.resources;

import com.fasterxml.jackson.core.type.TypeReference;
import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.OperationModels.*;

import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** Cost operations at /api/v1/costs. */
public class CostService {

    private final HttpHelper http;

    public CostService(HttpHelper http) { this.http = http; }

    public CostSummary summary() {
        return http.get("/api/v1/costs/summary", null, CostSummary.class);
    }

    /** WHY: Route is /api/v1/costs/history (not /costs/breakdown). */
    public List<CostEntry> history(String startDate, String endDate) {
        Map<String, String> params = new LinkedHashMap<>();
        if (startDate != null) params.put("start_date", startDate);
        if (endDate != null) params.put("end_date", endDate);
        return http.get("/api/v1/costs/history", params,
                new TypeReference<List<CostEntry>>() {});
    }

    public BudgetInfo budget() {
        return http.get("/api/v1/costs/budget", null, BudgetInfo.class);
    }
}
