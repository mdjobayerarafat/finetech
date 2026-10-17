// Package db owns the pgx connection pool and the schema bootstrap.
package db

import (
	"context"
	_ "embed"
	"errors"
	"fmt"

	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

//go:embed schema.sql
var schemaSQL string

// Connect opens a pgx pool and returns it.
func Connect(ctx context.Context, dsn string) (*pgxpool.Pool, error) {
	cfg, err := pgxpool.ParseConfig(dsn)
	if err != nil {
		return nil, fmt.Errorf("parse dsn: %w", err)
	}
	cfg.MaxConns = 8
	pool, err := pgxpool.NewWithConfig(ctx, cfg)
	if err != nil {
		return nil, fmt.Errorf("new pool: %w", err)
	}
	if err := pool.Ping(ctx); err != nil {
		pool.Close()
		return nil, fmt.Errorf("ping: %w", err)
	}
	return pool, nil
}

// Bootstrap runs the embedded schema and seeds demo data on first boot.
func Bootstrap(ctx context.Context, pool *pgxpool.Pool) error {
	if _, err := pool.Exec(ctx, schemaSQL); err != nil {
		return fmt.Errorf("apply schema: %w", err)
	}
	if err := seedIfEmpty(ctx, pool); err != nil {
		return fmt.Errorf("seed: %w", err)
	}
	return nil
}

func seedIfEmpty(ctx context.Context, pool *pgxpool.Pool) error {
	var n int
	if err := pool.QueryRow(ctx, `SELECT COUNT(*) FROM mockbank.users`).Scan(&n); err != nil {
		if !errors.Is(err, pgx.ErrNoRows) {
			return err
		}
		n = 0
	}
	if n > 0 {
		return nil
	}
	if _, err := pool.Exec(ctx, seedSQL); err != nil {
		return err
	}
	return nil
}
