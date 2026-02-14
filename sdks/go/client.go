package edgequake

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"math"
	"net/http"
	"net/url"
	"strings"
	"time"
)

// Client is the EdgeQuake API client.
type Client struct {
	cfg  *clientConfig
	http *http.Client

	Health        *HealthService
	Documents     *DocumentService
	Graph         *GraphService
	Entities      *EntityService
	Relationships *RelationshipService
	Query         *QueryService
	Chat          *ChatService
	Auth          *AuthService
	Users         *UserService
	APIKeys       *APIKeyService
	Tenants       *TenantService
	Conversations *ConversationService
	Folders       *FolderService
	Tasks         *TaskService
	Pipeline      *PipelineService
	Costs         *CostService
	Chunks        *ChunkService
	Provenance    *ProvenanceService
	Lineage       *LineageService
	Models        *ModelService
	Workspaces    *WorkspaceService
	PDF           *PDFService
}

// NewClient creates a new Client with the given options.
func NewClient(opts ...Option) *Client {
	cfg := defaultConfig()
	for _, o := range opts {
		o(cfg)
	}
	hc := cfg.httpClient
	if hc == nil {
		hc = &http.Client{Timeout: cfg.timeout}
	}
	c := &Client{cfg: cfg, http: hc}
	c.Health = &HealthService{c: c}
	c.Documents = &DocumentService{c: c}
	c.Graph = &GraphService{c: c}
	c.Entities = &EntityService{c: c}
	c.Relationships = &RelationshipService{c: c}
	c.Query = &QueryService{c: c}
	c.Chat = &ChatService{c: c}
	c.Auth = &AuthService{c: c}
	c.Users = &UserService{c: c}
	c.APIKeys = &APIKeyService{c: c}
	c.Tenants = &TenantService{c: c}
	c.Conversations = &ConversationService{c: c}
	c.Folders = &FolderService{c: c}
	c.Tasks = &TaskService{c: c}
	c.Pipeline = &PipelineService{c: c}
	c.Costs = &CostService{c: c}
	c.Chunks = &ChunkService{c: c}
	c.Provenance = &ProvenanceService{c: c}
	c.Lineage = &LineageService{c: c}
	c.Models = &ModelService{c: c}
	c.Workspaces = &WorkspaceService{c: c}
	c.PDF = &PDFService{c: c}
	return c
}

// BaseURL returns the configured base URL.
func (c *Client) BaseURL() string { return c.cfg.baseURL }

func (c *Client) newRequest(ctx context.Context, method, path string, body interface{}) (*http.Request, error) {
	u := strings.TrimRight(c.cfg.baseURL, "/") + "/" + strings.TrimLeft(path, "/")
	var bodyReader io.Reader
	if body != nil {
		b, err := json.Marshal(body)
		if err != nil {
			return nil, fmt.Errorf("edgequake: marshal body: %w", err)
		}
		bodyReader = bytes.NewReader(b)
	}
	req, err := http.NewRequestWithContext(ctx, method, u, bodyReader)
	if err != nil {
		return nil, err
	}
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}
	req.Header.Set("Accept", "application/json")
	req.Header.Set("User-Agent", c.cfg.userAgent)
	// WHY: API key uses X-API-Key header; JWT uses Authorization: Bearer.
	// This matches the actual auth middleware in edgequake-api.
	if c.cfg.apiKey != "" {
		req.Header.Set("X-API-Key", c.cfg.apiKey)
	}
	if c.cfg.bearerToken != "" {
		req.Header.Set("Authorization", "Bearer "+c.cfg.bearerToken)
	}
	if c.cfg.tenantID != "" {
		req.Header.Set("X-Tenant-ID", c.cfg.tenantID)
	}
	if c.cfg.workspaceID != "" {
		req.Header.Set("X-Workspace-ID", c.cfg.workspaceID)
	}
	if c.cfg.userID != "" {
		req.Header.Set("X-User-ID", c.cfg.userID)
	}
	return req, nil
}

func (c *Client) do(req *http.Request, v interface{}) error {
	maxAttempts := c.cfg.maxRetries + 1
	if maxAttempts < 1 {
		maxAttempts = 1
	}
	var lastErr error
	for attempt := 0; attempt < maxAttempts; attempt++ {
		if attempt > 0 {
			backoff := time.Duration(math.Pow(2, float64(attempt-1))) * 500 * time.Millisecond
			select {
			case <-time.After(backoff):
			case <-req.Context().Done():
				return req.Context().Err()
			}
			if req.GetBody != nil {
				body, err := req.GetBody()
				if err != nil {
					return fmt.Errorf("edgequake: retry body: %w", err)
				}
				req.Body = body
			}
		}
		resp, err := c.http.Do(req)
		if err != nil {
			lastErr = fmt.Errorf("edgequake: request: %w", err)
			continue
		}
		if resp.StatusCode >= 200 && resp.StatusCode < 300 {
			if v != nil && resp.StatusCode != http.StatusNoContent {
				defer resp.Body.Close()
				return json.NewDecoder(resp.Body).Decode(v)
			}
			resp.Body.Close()
			return nil
		}
		apiErr := parseAPIError(resp)
		resp.Body.Close()
		ae, ok := apiErr.(*APIError)
		if ok && ae.IsRetryable() && attempt < maxAttempts-1 {
			lastErr = apiErr
			continue
		}
		return apiErr
	}
	return lastErr
}

func (c *Client) get(ctx context.Context, path string, params url.Values, v interface{}) error {
	if len(params) > 0 {
		path = path + "?" + params.Encode()
	}
	req, err := c.newRequest(ctx, http.MethodGet, path, nil)
	if err != nil {
		return err
	}
	return c.do(req, v)
}

func (c *Client) post(ctx context.Context, path string, body, v interface{}) error {
	req, err := c.newRequest(ctx, http.MethodPost, path, body)
	if err != nil {
		return err
	}
	return c.do(req, v)
}

// WHY: put() and patch() with response decoding, and del() with response body,
// are available for future service methods that need them. Currently unused
// but retained as part of the complete HTTP method surface.
// Uncomment when needed:
//   func (c *Client) put(ctx, path, body, v) error { ... }
//   func (c *Client) patch(ctx, path, body, v) error { ... }
//   func (c *Client) del(ctx, path, v) error { ... }

func (c *Client) delNoContent(ctx context.Context, path string) error {
	req, err := c.newRequest(ctx, http.MethodDelete, path, nil)
	if err != nil {
		return err
	}
	return c.do(req, nil)
}

func (c *Client) postNoContent(ctx context.Context, path string, body interface{}) error {
	req, err := c.newRequest(ctx, http.MethodPost, path, body)
	if err != nil {
		return err
	}
	return c.do(req, nil)
}

func (c *Client) patchNoContent(ctx context.Context, path string, body interface{}) error {
	req, err := c.newRequest(ctx, http.MethodPatch, path, body)
	if err != nil {
		return err
	}
	return c.do(req, nil)
}

// getRaw performs a GET and returns the raw response body as bytes.
// WHY: The lineage export endpoint returns CSV or raw JSON, not a typed struct.
func (c *Client) getRaw(ctx context.Context, path string, params url.Values) ([]byte, error) {
	if len(params) > 0 {
		path = path + "?" + params.Encode()
	}
	req, err := c.newRequest(ctx, http.MethodGet, path, nil)
	if err != nil {
		return nil, err
	}
	maxAttempts := c.cfg.maxRetries + 1
	if maxAttempts < 1 {
		maxAttempts = 1
	}
	var lastErr error
	for attempt := 0; attempt < maxAttempts; attempt++ {
		if attempt > 0 {
			backoff := time.Duration(math.Pow(2, float64(attempt-1))) * 500 * time.Millisecond
			select {
			case <-time.After(backoff):
			case <-req.Context().Done():
				return nil, req.Context().Err()
			}
		}
		resp, err := c.http.Do(req)
		if err != nil {
			lastErr = fmt.Errorf("edgequake: request: %w", err)
			continue
		}
		defer resp.Body.Close()
		if resp.StatusCode >= 200 && resp.StatusCode < 300 {
			return io.ReadAll(resp.Body)
		}
		apiErr := parseAPIError(resp)
		ae, ok := apiErr.(*APIError)
		if ok && ae.IsRetryable() && attempt < maxAttempts-1 {
			lastErr = apiErr
			continue
		}
		return nil, apiErr
	}
	return nil, lastErr
}
