package handlers

import (
	"errors"
	"net/http"
	"strconv"
	"time"

	"github.com/bankpilot/mockbank/internal/audit"
	"github.com/bankpilot/mockbank/internal/middleware"
	"github.com/bankpilot/mockbank/internal/policy"
	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

type transferReq struct {
	SourceAccountID string `json:"source_account_id" binding:"required"`
	PayeeID         string `json:"payee_id"          binding:"required"`
	AmountFen       int64  `json:"amount_fen"        binding:"required"`
	Memo            string `json:"memo"`
}

// CreateTransfer applies the Green/Yellow/Red policy, the daily limit, and
// the safety lock, then writes the transfer + a paired transaction row.
func CreateTransfer(pool *pgxpool.Pool, dailyLimitFen int64, aw *audit.Writer) gin.HandlerFunc {
	return func(c *gin.Context) {
		ctx := c.Request.Context()
		uid, _ := c.Get(string(middleware.CtxUserID))
		userID, _ := uid.(string)
		reqID, _ := c.Get(string(middleware.CtxRequestID))
		reqIDStr, _ := reqID.(string)
		idem := c.GetHeader("X-Idempotency-Key")

		var req transferReq
		if err := c.ShouldBindJSON(&req); err != nil {
			c.JSON(http.StatusBadRequest, gin.H{"error": "bad_request", "detail": err.Error()})
			return
		}

		// Idempotency: return the prior transfer if the same key was used.
		if idem != "" {
			var existingID, existingStatus, existingTier string
			err := pool.QueryRow(ctx,
				`SELECT id, status, tier FROM mockbank.transfers WHERE idempotency_key = $1 AND user_id = $2`,
				idem, userID,
			).Scan(&existingID, &existingStatus, &existingTier)
			if err == nil {
				c.JSON(http.StatusOK, gin.H{
					"transfer_id":      existingID,
					"status":           existingStatus,
					"tier":             existingTier,
					"idempotent_replay": true,
				})
				return
			}
			if !errors.Is(err, pgx.ErrNoRows) {
				c.JSON(http.StatusInternalServerError, gin.H{"error": "idempotency_lookup_failed"})
				return
			}
		}

		// Safety lock check.
		var lockState, lockReason string
		err := pool.QueryRow(ctx,
			`SELECT safety_lock_state, COALESCE(safety_lock_reason,'')
			   FROM mockbank.users WHERE id = $1`, userID,
		).Scan(&lockState, &lockReason)
		if err != nil {
			c.JSON(http.StatusUnauthorized, gin.H{"error": "user_not_found"})
			return
		}
		if lockState == "locked" {
			_ = aw.Append(ctx, audit.Event{
				ActorUserID: userID, RequestID: reqIDStr,
				Action: "transfers.create", Tool: "transfers",
				Parameters: map[string]any{"amount_fen": req.AmountFen, "payee_id": req.PayeeID},
				Tier: "red", Result: "blocked", ErrorCode: "safety_locked",
			})
			c.JSON(http.StatusForbidden, gin.H{
				"error": "safety_locked",
				"reason": lockReason,
			})
			return
		}

		// Verify source account ownership.
		var ownerID, srcStatus string
		var availableFen int64
		err = pool.QueryRow(ctx,
			`SELECT user_id, status, available_balance_fen
			   FROM mockbank.accounts WHERE id = $1`, req.SourceAccountID,
		).Scan(&ownerID, &srcStatus, &availableFen)
		if err != nil {
			c.JSON(http.StatusBadRequest, gin.H{"error": "source_account_not_found"})
			return
		}
		if ownerID != userID {
			c.JSON(http.StatusForbidden, gin.H{"error": "not_source_owner"})
			return
		}
		if srcStatus != "active" {
			c.JSON(http.StatusLocked, gin.H{"error": "source_not_active", "status": srcStatus})
			return
		}

		// Verify payee ownership.
		var payeeOwner, payeeDest string
		var payeeVerified bool
		err = pool.QueryRow(ctx,
			`SELECT owner_user_id, mock_destination, verified
			   FROM mockbank.payees WHERE id = $1`, req.PayeeID,
		).Scan(&payeeOwner, &payeeDest, &payeeVerified)
		if err != nil {
			c.JSON(http.StatusBadRequest, gin.H{"error": "payee_not_found"})
			return
		}
		if payeeOwner != userID {
			c.JSON(http.StatusForbidden, gin.H{"error": "not_payee_owner"})
			return
		}

		// Tier decision.
		dec := policy.DecideTransfer(req.AmountFen, dailyLimitFen)
		// Promotion: unverified payee -> always yellow; over-threshold -> red
		if !payeeVerified {
			dec = policy.Decision{Tier: policy.Yellow, Reason: "unverified_payee", Requires: "confirmation"}
		}

		// Insufficient funds: short-circuit before any tier logic.
		if req.AmountFen > availableFen {
			_ = aw.Append(ctx, audit.Event{
				ActorUserID: userID, RequestID: reqIDStr,
				Action: "transfers.create", Tool: "transfers",
				Parameters: map[string]any{"amount_fen": req.AmountFen, "payee_id": req.PayeeID},
				Tier: dec.Tier, Result: "error", ErrorCode: "insufficient_funds",
			})
			c.JSON(http.StatusUnprocessableEntity, gin.H{"error": "insufficient_funds"})
			return
		}

		// Red tier requires the X-MFA-Token. The AuthRequired middleware ran
		// first; this handler checks the header itself so the response can be
		// precise.
		if dec.Tier == policy.Red {
			raw := c.GetHeader("X-MFA-Token")
			if raw == "" {
				_ = aw.Append(ctx, audit.Event{
					ActorUserID: userID, RequestID: reqIDStr,
					Action: "transfers.create", Tool: "transfers",
					Parameters: map[string]any{"amount_fen": req.AmountFen, "payee_id": req.PayeeID},
					Tier: "red", Result: "denied", ErrorCode: "mfa_required",
				})
				c.JSON(http.StatusForbidden, gin.H{
					"error":    "mfa_required",
					"tier":     "red",
					"reason":   dec.Reason,
					"requires": "X-MFA-Token header; obtain via POST /api/v1/mfa/verify",
				})
				return
			}
		}

		// Daily-limit check across all channels.
		today := time.Now().UTC().Truncate(24 * time.Hour)
		var todayTotal int64
		err = pool.QueryRow(ctx,
			`SELECT COALESCE(SUM(amount_fen),0)
			   FROM mockbank.transfers
			  WHERE user_id = $1 AND posted_at >= $2 AND status = 'posted'`,
			userID, today,
		).Scan(&todayTotal)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "limit_lookup_failed"})
			return
		}
		if todayTotal+req.AmountFen > dailyLimitFen {
			_ = audit.Safety(ctx, pool, userID, "denied_high_risk", map[string]any{
				"reason":         "over_daily_limit",
				"amount_fen":     req.AmountFen,
				"today_total_fen": todayTotal,
			})
			_ = aw.Append(ctx, audit.Event{
				ActorUserID: userID, RequestID: reqIDStr,
				Action: "transfers.create", Tool: "transfers",
				Parameters: map[string]any{"amount_fen": req.AmountFen, "payee_id": req.PayeeID},
				Tier: "red", Result: "denied", ErrorCode: "over_daily_limit",
			})
			c.JSON(http.StatusForbidden, gin.H{
				"error":  "over_daily_limit",
				"tier":   "red",
				"limit_fen": dailyLimitFen,
				"today_total_fen": todayTotal,
			})
			return
		}

		// Execute. Transfer + transaction + balance update in one tx.
		tx, err := pool.Begin(ctx)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "tx_begin_failed"})
			return
		}
		defer tx.Rollback(ctx)

		transferID := "xfer_" + uuid.NewString()
		txID := "tx_" + uuid.NewString()
		now := time.Now().UTC()

		_, err = tx.Exec(ctx,
			`INSERT INTO mockbank.transfers
			   (id, user_id, source_account_id, payee_id, amount_fen, memo, idempotency_key, status, tier, posted_at)
			 VALUES ($1,$2,$3,$4,$5,$6,$7,'posted',$8,$9)`,
			transferID, userID, req.SourceAccountID, req.PayeeID, req.AmountFen, req.Memo, idem, dec.Tier, now,
		)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "transfer_insert_failed", "detail": err.Error()})
			return
		}
		_, err = tx.Exec(ctx,
			`INSERT INTO mockbank.transactions
			   (id, account_id, amount_fen, direction, category, merchant, memo, occurred_at, status)
			 VALUES ($1,$2,$3,'debit','transfer',$4,$5,$6,'posted')`,
			txID, req.SourceAccountID, -req.AmountFen, payeeDest, req.Memo, now,
		)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "transaction_insert_failed", "detail": err.Error()})
			return
		}
		_, err = tx.Exec(ctx,
			`UPDATE mockbank.accounts
			    SET available_balance_fen = available_balance_fen - $1
			  WHERE id = $2`,
			req.AmountFen, req.SourceAccountID,
		)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "balance_update_failed", "detail": err.Error()})
			return
		}
		if err := tx.Commit(ctx); err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "tx_commit_failed"})
			return
		}

		_ = aw.Append(ctx, audit.Event{
			ActorUserID: userID, RequestID: reqIDStr,
			Action: "transfers.create", Tool: "transfers",
			Parameters: map[string]any{
				"amount_fen": req.AmountFen, "payee_id": req.PayeeID,
				"source_account_id": req.SourceAccountID, "memo": req.Memo,
			},
			Tier: dec.Tier, Result: "ok",
		})

		c.JSON(http.StatusOK, gin.H{
			"transfer_id": transferID,
			"transaction_id": txID,
			"status":       "posted",
			"tier":         dec.Tier,
			"reason":       dec.Reason,
		})
	}
}

// ListTransfers returns recent transfers for the caller (for the audit UI).
func ListTransfers(pool *pgxpool.Pool) gin.HandlerFunc {
	return func(c *gin.Context) {
		uid, _ := c.Get(string(middleware.CtxUserID))
		userID, _ := uid.(string)
		limit := 50
		if v := c.Query("limit"); v != "" {
			if n, err := strconv.Atoi(v); err == nil && n > 0 && n <= 200 {
				limit = n
			}
		}
		rows, err := pool.Query(c.Request.Context(),
			`SELECT id, source_account_id, payee_id, amount_fen, memo, status, tier, posted_at
			   FROM mockbank.transfers
			  WHERE user_id = $1
			  ORDER BY posted_at DESC
			  LIMIT $2`,
			userID, limit,
		)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "query_failed"})
			return
		}
		defer rows.Close()
		type out struct {
			ID              string `json:"id"`
			SourceAccountID string `json:"source_account_id"`
			PayeeID         string `json:"payee_id"`
			AmountFen       int64  `json:"amount_fen"`
			Memo            string `json:"memo"`
			Status          string `json:"status"`
			Tier            string `json:"tier"`
			PostedAt        string `json:"posted_at"`
		}
		outs := []out{}
		for rows.Next() {
			var o out
			var posted time.Time
			if err := rows.Scan(&o.ID, &o.SourceAccountID, &o.PayeeID, &o.AmountFen,
				&o.Memo, &o.Status, &o.Tier, &posted); err != nil {
				c.JSON(http.StatusInternalServerError, gin.H{"error": "scan_failed"})
				return
			}
			o.PostedAt = posted.UTC().Format(time.RFC3339)
			outs = append(outs, o)
		}
		c.JSON(http.StatusOK, gin.H{"transfers": outs})
	}
}
