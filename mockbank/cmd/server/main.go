// Package main wires up the mockbank HTTP server.
package main

import (
	"context"
	"errors"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/bankpilot/mockbank/internal/auth"
	"github.com/bankpilot/mockbank/internal/config"
	"github.com/bankpilot/mockbank/internal/db"
	"github.com/bankpilot/mockbank/internal/handlers"
	"github.com/gin-gonic/gin"
)

func main() {
	cfg, err := config.Load()
	if err != nil {
		log.Fatalf("config: %v", err)
	}

	pool, err := db.Connect(context.Background(), cfg.DatabaseURL)
	if err != nil {
		log.Fatalf("db connect: %v", err)
	}
	defer pool.Close()

	if err := db.Bootstrap(context.Background(), pool); err != nil {
		log.Fatalf("db bootstrap: %v", err)
	}

	// Rewrite placeholder password hashes the seed inserted, with real Argon2
	// hashes for the demo users. Runs on every boot; cost is one UPDATE each.
	devPasswords := map[string]string{
		"admin":        "admin1234",
		"demo":         "demo1234",
		"intl_student": "demo1234",
		"locked_vip":   "demo1234",
		"auditor":      "demo1234",
	}
	if err := auth.SeedDevUsers(context.Background(), pool, devPasswords); err != nil {
		log.Printf("warn: dev user seed: %v", err)
	}

	if cfg.AppEnv == "production" {
		gin.SetMode(gin.ReleaseMode)
	}

	r := handlers.NewRouter(pool, cfg)

	srv := &http.Server{
		Addr:              cfg.Host + ":" + cfg.Port,
		Handler:           r,
		ReadHeaderTimeout: 5 * time.Second,
	}

	go func() {
		log.Printf("mockbank listening on %s", srv.Addr)
		if err := srv.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			log.Fatalf("listen: %v", err)
		}
	}()

	stop := make(chan os.Signal, 1)
	signal.Notify(stop, os.Interrupt, syscall.SIGTERM)
	<-stop
	log.Println("shutting down")

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := srv.Shutdown(ctx); err != nil {
		log.Printf("shutdown: %v", err)
	}
}
