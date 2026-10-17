package handlers

import (
	"net/http"
	"time"

	"github.com/bankpilot/mockbank/internal/middleware"
	"github.com/gin-gonic/gin"
	"github.com/jackc/pgx/v5/pgxpool"
)

// SpendingReport groups the caller's transactions by category for a window.
func SpendingReport(pool *pgxpool.Pool) gin.HandlerFunc {
	return func(c *gin.Context) {
		uid, _ := c.Get(string(middleware.CtxUserID))
		userID, _ := uid.(string)
		from := c.Query("from")
		to := c.Query("to")
		if from == "" {
			from = time.Now().UTC().AddDate(0, -1, 0).Format("2006-01-02")
		}
		if to == "" {
			to = time.Now().UTC().Format("2006-01-02")
		}
		rows, err := pool.Query(c.Request.Context(),
			`SELECT t.category, SUM(t.amount_fen), COUNT(*)
			   FROM mockbank.transactions t
			   JOIN mockbank.accounts a ON a.id = t.account_id
			  WHERE a.user_id = $1
			    AND t.occurred_at >= $2::date
			    AND t.occurred_at <  ($3::date + INTERVAL '1 day')
			    AND t.direction = 'debit'
			  GROUP BY t.category
			  ORDER BY SUM(t.amount_fen)`,
			userID, from, to,
		)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "query_failed", "detail": err.Error()})
			return
		}
		defer rows.Close()
		type bucket struct {
			Category   string `json:"category"`
			TotalFen   int64  `json:"total_fen"`
			Count      int    `json:"count"`
		}
		out := []bucket{}
		var total int64
		for rows.Next() {
			var b bucket
			if err := rows.Scan(&b.Category, &b.TotalFen, &b.Count); err != nil {
				c.JSON(http.StatusInternalServerError, gin.H{"error": "scan_failed"})
				return
			}
			total += -b.TotalFen
			out = append(out, b)
		}
		c.JSON(http.StatusOK, gin.H{
			"from": from,
			"to":   to,
			"total_spend_fen": total,
			"buckets": out,
		})
	}
}

// RecurringBills returns the caller's subscriptions — server-detected recurring charges.
func RecurringBills(pool *pgxpool.Pool) gin.HandlerFunc {
	return func(c *gin.Context) {
		uid, _ := c.Get(string(middleware.CtxUserID))
		userID, _ := uid.(string)
		rows, err := pool.Query(c.Request.Context(),
			`SELECT id, merchant, estimated_amount_fen, interval_days, next_expected_at, last_charge_at, cancellation_status
			   FROM mockbank.subscriptions
			  WHERE user_id = $1
			  ORDER BY next_expected_at`,
			userID,
		)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "query_failed"})
			return
		}
		defer rows.Close()
		type sub struct {
			ID                   string  `json:"id"`
			Merchant             string  `json:"merchant"`
			EstimatedAmountFen   int64   `json:"estimated_amount_fen"`
			IntervalDays         int     `json:"interval_days"`
			NextExpectedAt       string  `json:"next_expected_at"`
			LastChargeAt         *string `json:"last_charge_at,omitempty"`
			CancellationStatus   string  `json:"cancellation_status"`
		}
		out := []sub{}
		for rows.Next() {
			var s sub
			var next time.Time
			var last *time.Time
			if err := rows.Scan(&s.ID, &s.Merchant, &s.EstimatedAmountFen, &s.IntervalDays, &next, &last, &s.CancellationStatus); err != nil {
				c.JSON(http.StatusInternalServerError, gin.H{"error": "scan_failed"})
				return
			}
			s.NextExpectedAt = next.UTC().Format(time.RFC3339)
			if last != nil {
				s2 := last.UTC().Format(time.RFC3339)
				s.LastChargeAt = &s2
			}
			out = append(out, s)
		}
		c.JSON(http.StatusOK, gin.H{"subscriptions": out})
	}
}
