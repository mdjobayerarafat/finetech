package handlers

import (
	"errors"
	"net/http"
	"time"

	"github.com/bankpilot/mockbank/internal/audit"
	"github.com/bankpilot/mockbank/internal/middleware"
	"github.com/bankpilot/mockbank/internal/policy"
	"github.com/gin-gonic/gin"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

// ListSubscriptions is identical to /bills/recurring; kept as a separate route
// so the agent can call either surface.
func ListSubscriptions(pool *pgxpool.Pool) gin.HandlerFunc {
	return RecurringBills(pool)
}

// CancelSubscription flips the cancellation_status to 'cancellation_requested'
// (the demo simulates a one-step cancel). Yellow tier — confirmation only.
func CancelSubscription(pool *pgxpool.Pool, aw *audit.Writer) gin.HandlerFunc {
	return func(c *gin.Context) {
		ctx := c.Request.Context()
		uid, _ := c.Get(string(middleware.CtxUserID))
		userID, _ := uid.(string)
		reqID, _ := c.Get(string(middleware.CtxRequestID))
		reqIDStr, _ := reqID.(string)
		subID := c.Param("id")

		dec := policy.DecideSubscriptionCancel()

		var owner, status string
		err := pool.QueryRow(ctx,
			`SELECT user_id, cancellation_status FROM mockbank.subscriptions WHERE id = $1`,
			subID,
		).Scan(&owner, &status)
		if err != nil {
			if errors.Is(err, pgx.ErrNoRows) {
				c.JSON(http.StatusNotFound, gin.H{"error": "not_found"})
				return
			}
			c.JSON(http.StatusInternalServerError, gin.H{"error": "query_failed"})
			return
		}
		if owner != userID {
			c.JSON(http.StatusForbidden, gin.H{"error": "not_owner"})
			return
		}
		if status == "cancelled" || status == "cancellation_requested" {
			c.JSON(http.StatusOK, gin.H{"id": subID, "status": status, "note": "already_in_terminal_state"})
			return
		}

		_, err = pool.Exec(ctx,
			`UPDATE mockbank.subscriptions
			    SET cancellation_status = 'cancellation_requested',
			        cancelled_at = $1
			  WHERE id = $2`,
			time.Now().UTC(), subID,
		)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "update_failed"})
			return
		}
		_ = aw.Append(ctx, audit.Event{
			ActorUserID: userID, RequestID: reqIDStr,
			Action: "subscriptions.cancel", Tool: "subscriptions",
			Parameters: map[string]any{"subscription_id": subID},
			Tier: dec.Tier, Result: "ok",
		})
		c.JSON(http.StatusOK, gin.H{
			"id":     subID,
			"status": "cancellation_requested",
			"tier":   dec.Tier,
		})
	}
}
