package edgequake

import (
	"net/http"
	"time"
)

// Option configures the Client.
type Option func(*clientConfig)

type clientConfig struct {
	baseURL     string
	apiKey      string
	bearerToken string
	tenantID    string
	workspaceID string
	userID      string
	httpClient  *http.Client
	userAgent   string
	timeout     time.Duration
	maxRetries  int
}

func defaultConfig() *clientConfig {
	return &clientConfig{
		baseURL:    "http://localhost:8080",
		userAgent:  "edgequake-go/0.1.0",
		timeout:    30 * time.Second,
		maxRetries: 3,
	}
}

// WithBaseURL sets the EdgeQuake server URL.
func WithBaseURL(url string) Option {
	return func(c *clientConfig) { c.baseURL = url }
}

// WithAPIKey sets the API key for authentication.
func WithAPIKey(key string) Option {
	return func(c *clientConfig) { c.apiKey = key }
}

// WithBearerToken sets a JWT bearer token.
func WithBearerToken(token string) Option {
	return func(c *clientConfig) { c.bearerToken = token }
}

// WithTenantID sets the X-Tenant-ID header.
func WithTenantID(id string) Option {
	return func(c *clientConfig) { c.tenantID = id }
}

// WithWorkspaceID sets the X-Workspace-ID header.
func WithWorkspaceID(id string) Option {
	return func(c *clientConfig) { c.workspaceID = id }
}

// WithUserID sets the X-User-ID header for user-scoped endpoints (conversations, folders).
func WithUserID(id string) Option {
	return func(c *clientConfig) { c.userID = id }
}

// WithHTTPClient provides a custom http.Client.
func WithHTTPClient(hc *http.Client) Option {
	return func(c *clientConfig) { c.httpClient = hc }
}

// WithTimeout sets the request timeout.
func WithTimeout(d time.Duration) Option {
	return func(c *clientConfig) { c.timeout = d }
}

// WithMaxRetries sets the max retry count.
func WithMaxRetries(n int) Option {
	return func(c *clientConfig) { c.maxRetries = n }
}
