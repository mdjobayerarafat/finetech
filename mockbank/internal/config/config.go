// Package config loads env-driven settings. No file I/O on hot paths.
package config

import (
	"errors"
	"os"
	"strconv"
)

// Config holds runtime settings for the mockbank service.
type Config struct {
	Host           string
	Port           string
	AppEnv         string
	DatabaseURL    string
	SessionSecret  string
	SessionTTLSecs int

	DailyLimitFen   int64
	MFADemoCode     string
	MaxFailedMFA    int
}

// Load reads the environment and returns a populated Config.
func Load() (*Config, error) {
	cfg := &Config{
		Host:           getenv("HOST", "0.0.0.0"),
		Port:           getenv("PORT", "8090"),
		AppEnv:         getenv("APP_ENV", "development"),
		DatabaseURL:    os.Getenv("DATABASE_URL"),
		SessionSecret:  os.Getenv("SESSION_SECRET"),
		SessionTTLSecs: getenvInt("SESSION_TTL_SECONDS", 43200),
		DailyLimitFen:  int64(getenvInt("DAILY_LIMIT_FEN", 1_000_000)),
		MFADemoCode:    getenv("MFA_DEMO_CODE", "000000"),
		MaxFailedMFA:   getenvInt("MAX_FAILED_MFA", 3),
	}
	if cfg.DatabaseURL == "" {
		return nil, errors.New("DATABASE_URL is required")
	}
	if cfg.SessionSecret == "" {
		return nil, errors.New("SESSION_SECRET is required")
	}
	return cfg, nil
}

func getenv(key, def string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return def
}

func getenvInt(key string, def int) int {
	v := os.Getenv(key)
	if v == "" {
		return def
	}
	n, err := strconv.Atoi(v)
	if err != nil {
		return def
	}
	return n
}
