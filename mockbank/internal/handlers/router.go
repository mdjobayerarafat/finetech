package handlers

import (
	"github.com/bankpilot/mockbank/internal/audit"
	"github.com/bankpilot/mockbank/internal/auth"
	"github.com/bankpilot/mockbank/internal/config"
	"github.com/bankpilot/mockbank/internal/middleware"
	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

// NewRouter wires every route. Auth runs first; the rest of the chain depends
// on the route.
func NewRouter(pool *pgxpool.Pool, cfg *config.Config) *gin.Engine {
	r := gin.New()
	r.Use(gin.Recovery())
	r.Use(requestIDMiddleware())

	iss := auth.NewIssuer(cfg.SessionSecret, cfg.SessionTTLSecs)
	aw := audit.New(pool)

	// Public
	r.GET("/healthz", Health(pool))
	r.POST("/api/v1/auth/login", Login(pool, iss))

	// Authenticated
	authed := r.Group("/api/v1", middleware.AuthRequired(iss))
	authed.POST("/mfa/verify", MFAVerify(pool, iss, cfg.MFADemoCode, aw))

	authed.GET("/accounts", ListAccounts(pool))
	authed.GET("/accounts/:id/balance", GetBalance(pool))
	authed.GET("/accounts/:id/transactions", ListTransactions(pool))
	authed.GET("/payees", ListPayees(pool))

	// Mutating: require Idempotency-Key
	mut := authed.Group("", middleware.IdempotencyKey())
	mut.POST("/transfers", CreateTransfer(pool, cfg.DailyLimitFen, aw))

	// Yellow/Red actions: require MFA token for Red tier
	mut.POST("/subscriptions/:id/cancel", CancelSubscription(pool, aw))

	// Reads
	authed.GET("/transfers", ListTransfers(pool))
	authed.GET("/reports/spending", SpendingReport(pool))
	authed.GET("/bills/recurring", RecurringBills(pool))
	authed.GET("/subscriptions", ListSubscriptions(pool))
	authed.GET("/audit", ListAudit(pool))

	return r
}

// requestIDMiddleware ensures every request has an X-Request-ID for audit
// correlation.
func requestIDMiddleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		id := c.GetHeader("X-Request-ID")
		if id == "" {
			id = uuid.NewString()
		}
		c.Set(string(middleware.CtxRequestID), id)
		c.Writer.Header().Set("X-Request-ID", id)
		c.Next()
	}
}
