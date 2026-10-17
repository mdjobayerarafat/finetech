package handlers

import (
	"net/http"
	"time"

	"github.com/bankpilot/mockbank/internal/middleware"
	"github.com/gin-gonic/gin"
	"github.com/jackc/pgx/v5/pgxpool"
)

type account struct {
	ID                   string `json:"id"`
	UserID               string `json:"user_id"`
	DisplayName          string `json:"display_name"`
	MaskedNumber         string `json:"masked_number"`
	Currency             string `json:"currency"`
	AvailableBalanceFen  int64  `json:"available_balance_fen"`
	ReservedBalanceFen   int64  `json:"reserved_balance_fen"`
	Status               string `json:"status"`
	CreatedAt            string `json:"created_at"`
}

// ListAccounts returns every account owned by the caller.
func ListAccounts(pool *pgxpool.Pool) gin.HandlerFunc {
	return func(c *gin.Context) {
		uid, _ := c.Get(string(middleware.CtxUserID))
		userID, _ := uid.(string)

		rows, err := pool.Query(c.Request.Context(),
			`SELECT id, user_id, display_name, masked_number, currency,
			        available_balance_fen, reserved_balance_fen, status, created_at
			   FROM mockbank.accounts
			  WHERE user_id = $1
			  ORDER BY created_at`,
			userID,
		)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "query_failed"})
			return
		}
		defer rows.Close()

		out := []account{}
		for rows.Next() {
			var a account
			var created time.Time
			if err := rows.Scan(&a.ID, &a.UserID, &a.DisplayName, &a.MaskedNumber, &a.Currency,
				&a.AvailableBalanceFen, &a.ReservedBalanceFen, &a.Status, &created); err != nil {
				c.JSON(http.StatusInternalServerError, gin.H{"error": "scan_failed"})
				return
			}
			a.CreatedAt = created.UTC().Format(time.RFC3339)
			out = append(out, a)
		}
		c.JSON(http.StatusOK, gin.H{"accounts": out})
	}
}

// GetBalance returns the balance for a single account the caller owns.
func GetBalance(pool *pgxpool.Pool) gin.HandlerFunc {
	return func(c *gin.Context) {
		uid, _ := c.Get(string(middleware.CtxUserID))
		userID, _ := uid.(string)
		accountID := c.Param("id")

		var (
			owner         string
			displayName   string
			maskedNumber  string
			currency      string
			availableFen  int64
			reservedFen   int64
			status        string
		)
		err := pool.QueryRow(c.Request.Context(),
			`SELECT user_id, display_name, masked_number, currency,
			        available_balance_fen, reserved_balance_fen, status
			   FROM mockbank.accounts WHERE id = $1`,
			accountID,
		).Scan(&owner, &displayName, &maskedNumber, &currency, &availableFen, &reservedFen, &status)
		if err != nil {
			c.JSON(http.StatusNotFound, gin.H{"error": "not_found"})
			return
		}
		if owner != userID {
			c.JSON(http.StatusForbidden, gin.H{"error": "not_owner"})
			return
		}
		c.JSON(http.StatusOK, gin.H{
			"account_id":            accountID,
			"display_name":          displayName,
			"masked_number":         maskedNumber,
			"currency":              currency,
			"available_balance_fen": availableFen,
			"reserved_balance_fen":  reservedFen,
			"status":                status,
		})
	}
}

type transaction struct {
	ID         string `json:"id"`
	AccountID  string `json:"account_id"`
	AmountFen  int64  `json:"amount_fen"`
	Direction  string `json:"direction"`
	Category   string `json:"category"`
	Merchant   string `json:"merchant"`
	Memo       string `json:"memo"`
	OccurredAt string `json:"occurred_at"`
	Status     string `json:"status"`
}

// ListTransactions returns recent transactions for an owned account.
func ListTransactions(pool *pgxpool.Pool) gin.HandlerFunc {
	return func(c *gin.Context) {
		uid, _ := c.Get(string(middleware.CtxUserID))
		userID, _ := uid.(string)
		accountID := c.Param("id")

		var owner string
		if err := pool.QueryRow(c.Request.Context(),
			`SELECT user_id FROM mockbank.accounts WHERE id = $1`, accountID,
		).Scan(&owner); err != nil {
			c.JSON(http.StatusNotFound, gin.H{"error": "not_found"})
			return
		}
		if owner != userID {
			c.JSON(http.StatusForbidden, gin.H{"error": "not_owner"})
			return
		}

		rows, err := pool.Query(c.Request.Context(),
			`SELECT id, account_id, amount_fen, direction, category, merchant, memo, occurred_at, status
			   FROM mockbank.transactions
			  WHERE account_id = $1
			  ORDER BY occurred_at DESC
			  LIMIT 200`,
			accountID,
		)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "query_failed"})
			return
		}
		defer rows.Close()
		out := []transaction{}
		for rows.Next() {
			var t transaction
			var occurred time.Time
			if err := rows.Scan(&t.ID, &t.AccountID, &t.AmountFen, &t.Direction, &t.Category,
				&t.Merchant, &t.Memo, &occurred, &t.Status); err != nil {
				c.JSON(http.StatusInternalServerError, gin.H{"error": "scan_failed"})
				return
			}
			t.OccurredAt = occurred.UTC().Format(time.RFC3339)
			out = append(out, t)
		}
		c.JSON(http.StatusOK, gin.H{"transactions": out})
	}
}
