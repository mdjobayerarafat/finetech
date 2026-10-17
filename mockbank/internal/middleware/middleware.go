// Package middleware contains the request-scoped middlewares.
package middleware

import (
	"net/http"
	"strings"

	"github.com/bankpilot/mockbank/internal/auth"
	"github.com/gin-gonic/gin"
)

// CtxKey is the type for context keys set by middlewares.
type CtxKey string

const (
	CtxUserID    CtxKey = "userID"
	CtxUserRole  CtxKey = "userRole"
	CtxRequestID CtxKey = "requestID"
)

// AuthRequired verifies the bearer token and stashes user info in the context.
func AuthRequired(iss *auth.Issuer) gin.HandlerFunc {
	return func(c *gin.Context) {
		h := c.GetHeader("Authorization")
		if !strings.HasPrefix(h, "Bearer ") {
			c.AbortWithStatusJSON(http.StatusUnauthorized, gin.H{"error": "missing_bearer_token"})
			return
		}
		tok := strings.TrimPrefix(h, "Bearer ")
		claims, err := iss.Verify(tok)
		if err != nil {
			c.AbortWithStatusJSON(http.StatusUnauthorized, gin.H{"error": "invalid_token", "detail": err.Error()})
			return
		}
		c.Set(string(CtxUserID), claims.UserID)
		c.Set(string(CtxUserRole), claims.Role)
		c.Next()
	}
}

// IdempotencyKey enforces that mutating routes carry an Idempotency-Key header.
func IdempotencyKey() gin.HandlerFunc {
	return func(c *gin.Context) {
		if c.GetHeader("X-Idempotency-Key") == "" {
			c.AbortWithStatusJSON(http.StatusBadRequest, gin.H{
				"error": "missing_idempotency_key",
			})
			return
		}
		c.Next()
	}
}
