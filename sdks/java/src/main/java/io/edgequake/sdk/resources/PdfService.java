package io.edgequake.sdk.resources;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.OperationModels.*;

/**
 * PDF operations at /api/v1/documents/pdf.
 * WHY: PDF endpoints are nested under /documents/pdf/ in the real API.
 */
public class PdfService {

    private final HttpHelper http;

    public PdfService(HttpHelper http) { this.http = http; }

    public PdfProgressResponse progress(String trackId) {
        return http.get("/api/v1/documents/pdf/progress/" + trackId, null,
                PdfProgressResponse.class);
    }

    public PdfContentResponse content(String pdfId) {
        return http.get("/api/v1/documents/pdf/" + pdfId + "/content", null,
                PdfContentResponse.class);
    }

    public PdfProgressResponse status(String pdfId) {
        return http.get("/api/v1/documents/pdf/" + pdfId, null, PdfProgressResponse.class);
    }
}
