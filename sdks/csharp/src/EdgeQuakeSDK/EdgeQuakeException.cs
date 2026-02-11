namespace EdgeQuakeSDK;

/// <summary>HTTP error from the EdgeQuake API.</summary>
public class EdgeQuakeException : Exception
{
    public int? StatusCode { get; }
    public string? ResponseBody { get; }

    public EdgeQuakeException(string message, int? statusCode = null, string? responseBody = null)
        : base(message)
    {
        StatusCode = statusCode;
        ResponseBody = responseBody;
    }
}
