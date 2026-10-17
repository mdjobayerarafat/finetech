package handlers

import (
	"net/http"

	"github.com/bankpilot/mockbank/internal/audit"
	"github.com/bankpilot/mockbank/internal/auth"
	"github.com/bankpilot/mockbank/internal/middleware"
	"github.com/gin-gonic/gin"
	"github.com/jackc/pgx/v5/pgxpool"
)

type loginReq struct {
	Username string `json:"username" binding:"required"`
	Password string `json:"password" binding:"required"`
}

// Login is a dev-only password grant. Returns a JWT.
func Login(pool *pgxpool.Pool, iss *auth.Issuer) gin.HandlerFunc {
	return func(c *gin.Context) {
		var req loginReq
		if err := c.ShouldBindJSON(&req); err != nil {
			c.JSON(http.StatusBadRequest, gin.H{"error": "bad_request", "detail": err.Error()})
			return
		}
		var (
			userID, role, lockState, lockReason, hash string
		)
		err := pool.QueryRow(c.Request.Context(),
			`SELECT id, role, safety_lock_state, COALESCE(safety_lock_reason,''), password_hash
			   FROM mockbank.users WHERE username = $1`,
			req.Username,
		).Scan(&userID, &role, &lockState, &lockReason, &hash)
		if err != nil {
			c.JSON(http.StatusUnauthorized, gin.H{"error": "invalid_credentials"})
			return
		}
		ok, err := auth.VerifyPassword(req.Password, hash)
		if err != nil || !ok {
			c.JSON(http.StatusUnauthorized, gin.H{"error": "invalid_credentials"})
			return
		}
		tok, err := iss.Issue(userID, role, false)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "token_error"})
			return
		}
		c.JSON(http.StatusOK, gin.H{
			"token":   tok,
			"user_id": userID,
			"role":    role,
			"safety_lock": gin.H{
				"state":  lockState,
				"reason": lockReason,
			},
		})
	}
}

type mfaReq struct {
	Code string `json:"code" binding:"required"`
}

// MFAVerify exchanges a one-time code for a short-lived MFA token.
// Returns a fresh JWT with the mfa=true claim. Subsequent calls must echo it
// in the X-MFA-Token header.
func MFAVerify(pool *pgxpool.Pool, iss *auth.Issuer, demoCode string, aw *audit.Writer) gin.HandlerFunc {
	return func(c *gin.Context) {
		uid, _ := c.Get(string(middleware.CtxUserID))
		userID, _ := uid.(string)
		role, _ := c.Get(string(middleware.CtxUserRole))
		roleStr, _ := role.(string)
		if userID == "" {
			c.JSON(http.StatusUnauthorized, gin.H{"error": "unauthenticated"})
			return
		}
		var req mfaReq
		if err := c.ShouldBindJSON(&req); err != nil {
			c.JSON(http.StatusBadRequest, gin.H{"error": "bad_request"})
			return
		}
		reqID, _ := c.Get(string(middleware.CtxRequestID))
		reqIDStr, _ := reqID.(string)

		if req.Code != demoCode {
			_, _ = pool.Exec(c.Request.Context(),
				`UPDATE mockbank.users SET failed_mfa_count = failed_mfa_count + 1 WHERE id = $1`,
				userID,
			)
			_ = audit.Safety(c.Request.Context(), pool, userID, "failed_mfa", map[string]any{"code_attempt": req.Code})
			_ = aw.Append(c.Request.Context(), audit.Event{
				ActorUserID: userID, RequestID: reqIDStr,
				Action: "mfa.verify", Tool: "auth",
				Tier: "red", Result: "denied", ErrorCode: "mfa_failed",
			})
			c.JSON(http.StatusForbidden, gin.H{"error": "mfa_failed"})
			return
		}
		_, _ = pool.Exec(c.Request.Context(),
			`UPDATE mockbank.users SET failed_mfa_count = 0 WHERE id = $1`, userID,
		)
		tok, err := iss.Issue(userID, roleStr, true)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "token_error"})
			return
		}
		_ = aw.Append(c.Request.Context(), audit.Event{
			ActorUserID: userID, RequestID: reqIDStr,
			Action: "mfa.verify", Tool: "auth",
			Tier: "red", Result: "ok",
		})
		c.JSON(http.StatusOK, gin.H{"mfa_token": tok, "user_id": userID})
	}
}
