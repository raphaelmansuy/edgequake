package edgequake

import (
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
)

// Sentinel errors.
var (
	ErrBadRequest   = errors.New("edgequake: bad request")
	ErrUnauthorized = errors.New("edgequake: unauthorized")
	ErrForbidden    = errors.New("edgequake: forbidden")
	ErrNotFound     = errors.New("edgequake: not found")
	ErrConflict     = errors.New("edgequake: conflict")
	ErrValidation   = errors.New("edgequake: validation error")
	ErrRateLimited  = errors.New("edgequake: rate limited")
	ErrServer       = errors.New("edgequake: server error")
)

// APIError represents an error returned by the EdgeQuake API.
type APIError struct {
	StatusCode int    `json:"-"`
	ErrorCode  string `json:"error,omitempty"`
	Message    string `json:"message,omitempty"`
	Details    string `json:"details,omitempty"`
}

func (e *APIError) Error() string {
	if e.Message != "" {
		return fmt.Sprintf("edgequake: %d %s: %s", e.StatusCode, e.ErrorCode, e.Message)
	}
	return fmt.Sprintf("edgequake: %d %s", e.StatusCode, e.ErrorCode)
}

// Is allows errors.Is matching with sentinel errors.
func (e *APIError) Is(target error) bool {
	switch {
	case target == ErrBadRequest:
		return e.StatusCode == http.StatusBadRequest
	case target == ErrUnauthorized:
		return e.StatusCode == http.StatusUnauthorized
	case target == ErrForbidden:
		return e.StatusCode == http.StatusForbidden
	case target == ErrNotFound:
		return e.StatusCode == http.StatusNotFound
	case target == ErrConflict:
		return e.StatusCode == http.StatusConflict
	case target == ErrValidation:
		return e.StatusCode == http.StatusUnprocessableEntity
	case target == ErrRateLimited:
		return e.StatusCode == http.StatusTooManyRequests
	case target == ErrServer:
		return e.StatusCode >= 500
	default:
		return false
	}
}

// IsRetryable returns true for 429 and 5xx errors.
func (e *APIError) IsRetryable() bool {
	return e.StatusCode == http.StatusTooManyRequests || e.StatusCode >= 500
}

func parseAPIError(resp *http.Response) error {
	apiErr := &APIError{StatusCode: resp.StatusCode}
	if resp.Body != nil {
		_ = json.NewDecoder(resp.Body).Decode(apiErr)
	}
	if apiErr.ErrorCode == "" {
		apiErr.ErrorCode = http.StatusText(resp.StatusCode)
	}
	return apiErr
}
