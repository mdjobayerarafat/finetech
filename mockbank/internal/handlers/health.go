// Package handlers wires routes. Each resource is its own file for clarity.
package handlers

import (
	"context"
	"net/http"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/jackc/pgx/v5/pgxpool"
)

// Health is a simple liveness probe + DB ping.
func Health(pool *pgxpool.Pool) gin.HandlerFunc {
	return func(c *gin.Context) {
		ctx, cancel := context.WithTimeout(c.Request.Context(), 2*time.Second)
		defer cancel()
		if err := pool.Ping(ctx); err != nil {
			c.JSON(http.StatusServiceUnavailable, gin.H{"status": "down", "detail": err.Error()})
			return
		}
		c.JSON(http.StatusOK, gin.H{"status": "ok", "service": "mockbank"})
	}
}
