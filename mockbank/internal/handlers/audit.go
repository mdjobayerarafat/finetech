package handlers

import (
	"encoding/json"
	"net/http"
	"strconv"
	"time"

	"github.com/bankpilot/mockbank/internal/middleware"
	"github.com/gin-gonic/gin"
	"github.com/jackc/pgx/v5/pgxpool"
)

// ListAudit returns recent audit events. Admins see all; customers see their own.
func ListAudit(pool *pgxpool.Pool) gin.HandlerFunc {
	return func(c *gin.Context) {
		uid, _ := c.Get(string(middleware.CtxUserID))
		userID, _ := uid.(string)
		role, _ := c.Get(string(middleware.CtxUserRole))
		roleStr, _ := role.(string)

		limit := 50
		if v := c.Query("limit"); v != "" {
			if n, err := strconv.Atoi(v); err == nil && n > 0 && n <= 500 {
				limit = n
			}
		}

		ctx := c.Request.Context()
		var (
			rows pgxRows
			err  error
		)
		if roleStr == "admin" {
			rows, err = pool.Query(ctx,
				`SELECT id, created_at, COALESCE(actor_user_id,''), COALESCE(request_id,''),
				        action, COALESCE(tool,''), parameters_masked,
				        COALESCE(tier,''), result, COALESCE(error_code,'')
				   FROM mockbank.audit_events
				  ORDER BY created_at DESC
				  LIMIT $1`, limit)
		} else {
			rows, err = pool.Query(ctx,
				`SELECT id, created_at, COALESCE(actor_user_id,''), COALESCE(request_id,''),
				        action, COALESCE(tool,''), parameters_masked,
				        COALESCE(tier,''), result, COALESCE(error_code,'')
				   FROM mockbank.audit_events
				  WHERE actor_user_id = $1
				  ORDER BY created_at DESC
				  LIMIT $2`, userID, limit)
		}
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "query_failed", "detail": err.Error()})
			return
		}
		defer rows.Close()

		type ev struct {
			ID         string         `json:"id"`
			CreatedAt  string         `json:"created_at"`
			ActorUser  string         `json:"actor_user_id"`
			RequestID  string         `json:"request_id"`
			Action     string         `json:"action"`
			Tool       string         `json:"tool"`
			Parameters map[string]any `json:"parameters"`
			Tier       string         `json:"tier"`
			Result     string         `json:"result"`
			ErrorCode  string         `json:"error_code"`
		}
		outs := []ev{}
		for rows.Next() {
			var e ev
			var created time.Time
			var params []byte
			if err := rows.Scan(&e.ID, &created, &e.ActorUser, &e.RequestID, &e.Action,
				&e.Tool, &params, &e.Tier, &e.Result, &e.ErrorCode); err != nil {
				c.JSON(http.StatusInternalServerError, gin.H{"error": "scan_failed"})
				return
			}
			e.CreatedAt = created.UTC().Format(time.RFC3339)
			e.Parameters = decodeJSON(params)
			outs = append(outs, e)
		}
		c.JSON(http.StatusOK, gin.H{"events": outs})
	}
}

// pgxRows narrows the pgx return type to what we need.
type pgxRows interface {
	Next() bool
	Scan(...any) error
	Close()
}

func decodeJSON(b []byte) map[string]any {
	out := map[string]any{}
	if len(b) == 0 {
		return out
	}
	_ = json.Unmarshal(b, &out)
	return out
}
