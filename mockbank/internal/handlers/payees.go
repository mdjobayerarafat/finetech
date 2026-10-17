package handlers

import (
	"net/http"
	"time"

	"github.com/bankpilot/mockbank/internal/middleware"
	"github.com/gin-gonic/gin"
	"github.com/jackc/pgx/v5/pgxpool"
)

type payee struct {
	ID              string  `json:"id"`
	DisplayName     string  `json:"display_name"`
	PhoneTail       *string `json:"phone_tail,omitempty"`
	Verified        bool    `json:"verified"`
	FirstUsedAt     *string `json:"first_used_at,omitempty"`
}

// ListPayees returns the caller's payees.
func ListPayees(pool *pgxpool.Pool) gin.HandlerFunc {
	return func(c *gin.Context) {
		uid, _ := c.Get(string(middleware.CtxUserID))
		userID, _ := uid.(string)
		rows, err := pool.Query(c.Request.Context(),
			`SELECT id, display_name, phone_tail, verified, first_used_at
			   FROM mockbank.payees WHERE owner_user_id = $1 ORDER BY display_name`,
			userID,
		)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "query_failed"})
			return
		}
		defer rows.Close()
		out := []payee{}
		for rows.Next() {
			var p payee
			var phone *string
			var firstUsed *time.Time
			if err := rows.Scan(&p.ID, &p.DisplayName, &phone, &p.Verified, &firstUsed); err != nil {
				c.JSON(http.StatusInternalServerError, gin.H{"error": "scan_failed"})
				return
			}
			p.PhoneTail = phone
			if firstUsed != nil {
				s := firstUsed.UTC().Format(time.RFC3339)
				p.FirstUsedAt = &s
			}
			out = append(out, p)
		}
		c.JSON(http.StatusOK, gin.H{"payees": out})
	}
}
