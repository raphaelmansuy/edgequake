using System.Net;
using System.Text;
using System.Text.Json;

namespace EdgeQuakeSDK.Tests;

/// <summary>
/// Mock HttpMessageHandler that returns predefined responses.
/// WHY: Enables stateless unit testing without real HTTP calls.
/// </summary>
public class MockHttpMessageHandler : HttpMessageHandler
{
    public record RequestRecord(HttpMethod Method, string Url, string? Body);

    private readonly List<RequestRecord> _calls = new();
    private string _nextJson = "{}";
    private HttpStatusCode _nextStatus = HttpStatusCode.OK;

    public IReadOnlyList<RequestRecord> Calls => _calls;
    public RequestRecord? LastCall => _calls.Count > 0 ? _calls[^1] : null;

    public MockHttpMessageHandler() { }

    public MockHttpMessageHandler(string json, HttpStatusCode status = HttpStatusCode.OK)
    {
        _nextJson = json;
        _nextStatus = status;
    }

    public MockHttpMessageHandler WillReturn(string json, HttpStatusCode status = HttpStatusCode.OK)
    {
        _nextJson = json;
        _nextStatus = status;
        return this;
    }

    protected override async Task<HttpResponseMessage> SendAsync(
        HttpRequestMessage request, CancellationToken cancellationToken)
    {
        string? body = null;
        if (request.Content is not null)
            body = await request.Content.ReadAsStringAsync(cancellationToken);

        _calls.Add(new RequestRecord(request.Method, request.RequestUri!.PathAndQuery, body));

        return new HttpResponseMessage(_nextStatus)
        {
            Content = new StringContent(_nextJson, Encoding.UTF8, "application/json")
        };
    }
}
