// Package audit writes append-only events with a SHA-256 hash chain.
package audit

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"time"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

// Event is one row in the audit log.
type Event struct {
	ActorUserID     string
	RequestID       string
	Action          string
	Tool            string
	Parameters      map[string]any
	Tier            string
	Result          string // ok | error | denied | blocked
	ErrorCode       string
}

// Writer appends events with a hash chain so tampering is detectable.
type Writer struct {
	pool *pgxpool.Pool
}

// New returns a Writer for the given pool.
func New(pool *pgxpool.Pool) *Writer {
	return &Writer{pool: pool}
}

// Append writes an event. Each event hashes prev_hash + canonical payload.
func (w *Writer) Append(ctx context.Context, e Event) error {
	prev, err := w.lastHash(ctx)
	if err != nil {
		return err
	}
	params, _ := json.Marshal(e.Parameters)
	id := "audit_" + uuid.NewString()
	payload := fmt.Sprintf("%s|%s|%s|%s|%s|%s|%s|%s|%s",
		id, e.ActorUserID, e.RequestID, e.Action, e.Tool, string(params), e.Tier, e.Result, e.ErrorCode)
	sum := sha256.Sum256([]byte(prev + "|" + payload))
	hash := hex.EncodeToString(sum[:])

	_, err = w.pool.Exec(ctx, `
		INSERT INTO mockbank.audit_events
		  (id, actor_user_id, request_id, action, tool, parameters_masked, tier, result, error_code, prev_hash, hash)
		VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
	`, id, e.ActorUserID, e.RequestID, e.Action, e.Tool, params, e.Tier, e.Result, e.ErrorCode, prev, hash)
	return err
}

// lastHash returns the most recent hash, or "" if the log is empty.
func (w *Writer) lastHash(ctx context.Context) (string, error) {
	var h string
	err := w.pool.QueryRow(ctx,
		`SELECT hash FROM mockbank.audit_events ORDER BY created_at DESC LIMIT 1`,
	).Scan(&h)
	if err != nil {
		// No rows is fine; the chain starts empty.
		return "", nil
	}
	return h, nil
}

// Safety records a safety_event (failed_mfa, denied_high_risk, etc).
func Safety(ctx context.Context, pool *pgxpool.Pool, userID, kind string, meta map[string]any) error {
	m, _ := json.Marshal(meta)
	_, err := pool.Exec(ctx, `
		INSERT INTO mockbank.safety_events (id, user_id, kind, metadata)
		VALUES ($1, $2, $3, $4)
	`, "sev_"+uuid.NewString(), userID, kind, m)
	return err
}

// FormatTime is exposed so the audit writer and tests can use a single timestamp format.
func FormatTime(t time.Time) string { return t.UTC().Format(time.RFC3339Nano) }
