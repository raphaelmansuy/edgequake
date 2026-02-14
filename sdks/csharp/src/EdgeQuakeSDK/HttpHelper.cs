using System.Net.Http.Json;
using System.Text;
using System.Text.Json;

namespace EdgeQuakeSDK;

/// <summary>HTTP helper using HttpClient.</summary>
public class HttpHelper
{
    private readonly HttpClient _client;
    private readonly EdgeQuakeConfig _config;

    public static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        PropertyNameCaseInsensitive = true,
    };

    public HttpHelper(EdgeQuakeConfig config) : this(config, null) { }

    /// <summary>Internal constructor accepting a custom HttpMessageHandler (for testing).</summary>
    internal HttpHelper(EdgeQuakeConfig config, HttpMessageHandler? handler)
    {
        _config = config;
        _client = handler is not null
            ? new HttpClient(handler) { BaseAddress = new Uri(config.BaseUrl.TrimEnd('/')) }
            : new HttpClient { BaseAddress = new Uri(config.BaseUrl.TrimEnd('/')) };

        _client.Timeout = TimeSpan.FromSeconds(config.TimeoutSeconds);

        _client.DefaultRequestHeaders.Add("Accept", "application/json");
        if (config.ApiKey is not null) _client.DefaultRequestHeaders.Add("X-API-Key", config.ApiKey);
        if (config.TenantId is not null) _client.DefaultRequestHeaders.Add("X-Tenant-ID", config.TenantId);
        if (config.UserId is not null) _client.DefaultRequestHeaders.Add("X-User-ID", config.UserId);
        if (config.WorkspaceId is not null) _client.DefaultRequestHeaders.Add("X-Workspace-ID", config.WorkspaceId);
    }

    public async Task<T> GetAsync<T>(string path) where T : class
    {
        var resp = await _client.GetAsync(path);
        return await HandleResponse<T>(resp);
    }

    public async Task<T> PostAsync<T>(string path, object? body = null) where T : class
    {
        var content = body is not null
            ? new StringContent(JsonSerializer.Serialize(body, JsonOptions), Encoding.UTF8, "application/json")
            : new StringContent("{}", Encoding.UTF8, "application/json");
        var resp = await _client.PostAsync(path, content);
        return await HandleResponse<T>(resp);
    }

    public async Task<T> DeleteAsync<T>(string path) where T : class
    {
        var resp = await _client.DeleteAsync(path);
        return await HandleResponse<T>(resp);
    }

    /// <summary>
    /// WHY: DELETE endpoints return 204 No Content — no body to deserialize.
    /// </summary>
    public async Task DeleteNoContentAsync(string path)
    {
        var resp = await _client.DeleteAsync(path);
        await EnsureSuccess(resp);
    }

    public async Task<T> PutAsync<T>(string path, object? body = null) where T : class
    {
        var content = body is not null
            ? new StringContent(JsonSerializer.Serialize(body, JsonOptions), Encoding.UTF8, "application/json")
            : new StringContent("{}", Encoding.UTF8, "application/json");
        var resp = await _client.PutAsync(path, content);
        return await HandleResponse<T>(resp);
    }

    /// <summary>WHY: PUT endpoints may return 204 No Content.</summary>
    public async Task PutNoContentAsync(string path, object? body = null)
    {
        var content = body is not null
            ? new StringContent(JsonSerializer.Serialize(body, JsonOptions), Encoding.UTF8, "application/json")
            : new StringContent("{}", Encoding.UTF8, "application/json");
        var resp = await _client.PutAsync(path, content);
        await EnsureSuccess(resp);
    }

    public async Task<T> PatchAsync<T>(string path, object? body = null) where T : class
    {
        var content = body is not null
            ? new StringContent(JsonSerializer.Serialize(body, JsonOptions), Encoding.UTF8, "application/json")
            : new StringContent("{}", Encoding.UTF8, "application/json");
        var req = new HttpRequestMessage(HttpMethod.Patch, path) { Content = content };
        var resp = await _client.SendAsync(req);
        return await HandleResponse<T>(resp);
    }

    /// <summary>WHY: PATCH endpoints may return 204 No Content.</summary>
    public async Task PatchNoContentAsync(string path, object? body = null)
    {
        var content = body is not null
            ? new StringContent(JsonSerializer.Serialize(body, JsonOptions), Encoding.UTF8, "application/json")
            : new StringContent("{}", Encoding.UTF8, "application/json");
        var req = new HttpRequestMessage(HttpMethod.Patch, path) { Content = content };
        var resp = await _client.SendAsync(req);
        await EnsureSuccess(resp);
    }

    public async Task<string> PostRawAsync(string path, object? body = null)
    {
        var content = body is not null
            ? new StringContent(JsonSerializer.Serialize(body, JsonOptions), Encoding.UTF8, "application/json")
            : new StringContent("{}", Encoding.UTF8, "application/json");
        var resp = await _client.PostAsync(path, content);
        await EnsureSuccess(resp);
        return await resp.Content.ReadAsStringAsync();
    }

    public async Task<string> GetRawAsync(string path)
    {
        var resp = await _client.GetAsync(path);
        await EnsureSuccess(resp);
        return await resp.Content.ReadAsStringAsync();
    }

    private async Task<T> HandleResponse<T>(HttpResponseMessage resp) where T : class
    {
        await EnsureSuccess(resp);
        var json = await resp.Content.ReadAsStringAsync();
        return JsonSerializer.Deserialize<T>(json, JsonOptions)
            ?? throw new EdgeQuakeException("Deserialization returned null");
    }

    private static async Task EnsureSuccess(HttpResponseMessage resp)
    {
        if (!resp.IsSuccessStatusCode)
        {
            var body = await resp.Content.ReadAsStringAsync();
            throw new EdgeQuakeException(
                $"HTTP {(int)resp.StatusCode}: {body}",
                (int)resp.StatusCode,
                body
            );
        }
    }
}
