// Package auth issues and verifies HS256 JWTs, hashes passwords, and seeds
// the dev users. Uses Argon2id with a stable, documented parameter set so
// the Go service owns its own credential store (no shared hashes with Rust).
package auth

import (
	"context"
	"crypto/rand"
	"crypto/subtle"
	"encoding/base64"
	"errors"
	"fmt"
	"strings"
	"time"

	"github.com/golang-jwt/jwt/v5"
	"github.com/jackc/pgx/v5/pgxpool"
	"golang.org/x/crypto/argon2"
)

// Argon2 parameters (tune for ~50ms on a modern laptop).
const (
	argonTime    = 2
	argonMemory  = 19_456
	argonThreads = 1
	argonKeyLen  = 32
	argonSaltLen = 16
)

// Claims is the JWT payload.
type Claims struct {
	UserID string `json:"sub"`
	Role   string `json:"role"`
	MFA    bool   `json:"mfa"`
	jwt.RegisteredClaims
}

// Issuer mints and verifies tokens.
type Issuer struct {
	secret []byte
	ttl    time.Duration
}

// NewIssuer builds an Issuer with the given HS256 secret and TTL in seconds.
func NewIssuer(secret string, ttlSeconds int) *Issuer {
	return &Issuer{
		secret: []byte(secret),
		ttl:    time.Duration(ttlSeconds) * time.Second,
	}
}

// Issue creates a signed JWT for a user.
func (i *Issuer) Issue(userID, role string, mfa bool) (string, error) {
	now := time.Now().UTC()
	c := Claims{
		UserID: userID,
		Role:   role,
		MFA:    mfa,
		RegisteredClaims: jwt.RegisteredClaims{
			IssuedAt:  jwt.NewNumericDate(now),
			ExpiresAt: jwt.NewNumericDate(now.Add(i.ttl)),
			Issuer:    "mockbank",
		},
	}
	t := jwt.NewWithClaims(jwt.SigningMethodHS256, c)
	return t.SignedString(i.secret)
}

// Verify parses and validates a token string. Returns the claims on success.
func (i *Issuer) Verify(tokenStr string) (*Claims, error) {
	t, err := jwt.ParseWithClaims(tokenStr, &Claims{}, func(t *jwt.Token) (any, error) {
		if _, ok := t.Method.(*jwt.SigningMethodHMAC); !ok {
			return nil, fmt.Errorf("unexpected signing method: %v", t.Header["alg"])
		}
		return i.secret, nil
	})
	if err != nil {
		return nil, err
	}
	c, ok := t.Claims.(*Claims)
	if !ok || !t.Valid {
		return nil, errors.New("invalid token")
	}
	return c, nil
}

// HashPassword returns an Argon2id encoded hash:
//   $argon2id$v=19$m=19456,t=2,p=1$<saltB64>$<hashB64>
func HashPassword(password string) (string, error) {
	salt := make([]byte, argonSaltLen)
	if _, err := rand.Read(salt); err != nil {
		return "", err
	}
	h := argon2.IDKey([]byte(password), salt, argonTime, argonMemory, argonThreads, argonKeyLen)
	return fmt.Sprintf(
		"$argon2id$v=%d$m=%d,t=%d,p=%d$%s$%s",
		19, argonMemory, argonTime, argonThreads,
		base64.RawStdEncoding.EncodeToString(salt),
		base64.RawStdEncoding.EncodeToString(h),
	), nil
}

// VerifyPassword checks a password against an Argon2id encoded hash.
func VerifyPassword(password, encoded string) (bool, error) {
	parts := strings.Split(encoded, "$")
	if len(parts) != 6 || parts[1] != "argon2id" {
		return false, errors.New("invalid hash format")
	}
	var version int
	if _, err := fmt.Sscanf(parts[2], "v=%d", &version); err != nil {
		return false, err
	}
	var m, t uint32
	var p uint8
	if _, err := fmt.Sscanf(parts[3], "m=%d,t=%d,p=%d", &m, &t, &p); err != nil {
		return false, err
	}
	salt, err := base64.RawStdEncoding.DecodeString(parts[4])
	if err != nil {
		return false, err
	}
	want, err := base64.RawStdEncoding.DecodeString(parts[5])
	if err != nil {
		return false, err
	}
	got := argon2.IDKey([]byte(password), salt, t, m, p, uint32(len(want)))
	return subtle.ConstantTimeCompare(got, want) == 1, nil
}

// SeedDevUsers rewrites the placeholder password hashes for the demo users.
func SeedDevUsers(ctx context.Context, pool *pgxpool.Pool, passwords map[string]string) error {
	for username, pw := range passwords {
		h, err := HashPassword(pw)
		if err != nil {
			return err
		}
		_, err = pool.Exec(ctx,
			`UPDATE mockbank.users SET password_hash = $1 WHERE username = $2`,
			h, username,
		)
		if err != nil {
			return fmt.Errorf("seed %s: %w", username, err)
		}
	}
	return nil
}
