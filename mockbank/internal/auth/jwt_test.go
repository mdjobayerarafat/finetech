package auth

import "testing"

func TestHashAndVerifyPassword(t *testing.T) {
	const pw = "demo1234"
	h, err := HashPassword(pw)
	if err != nil {
		t.Fatalf("hash: %v", err)
	}
	ok, err := VerifyPassword(pw, h)
	if err != nil {
		t.Fatalf("verify: %v", err)
	}
	if !ok {
		t.Fatalf("verify returned false for correct password")
	}
	bad, err := VerifyPassword("wrong", h)
	if err != nil {
		t.Fatalf("verify wrong: %v", err)
	}
	if bad {
		t.Fatalf("verify returned true for wrong password")
	}
}

func TestIssueAndVerifyJWT(t *testing.T) {
	iss := NewIssuer("test-secret-do-not-use-in-prod-1234", 60)
	tok, err := iss.Issue("u_demo", "customer", false)
	if err != nil {
		t.Fatalf("issue: %v", err)
	}
	claims, err := iss.Verify(tok)
	if err != nil {
		t.Fatalf("verify: %v", err)
	}
	if claims.UserID != "u_demo" {
		t.Errorf("userID = %q", claims.UserID)
	}
	if claims.Role != "customer" {
		t.Errorf("role = %q", claims.Role)
	}
	if claims.MFA {
		t.Errorf("mfa should be false")
	}

	// MFA-flavoured token
	mfaTok, err := iss.Issue("u_demo", "customer", true)
	if err != nil {
		t.Fatalf("issue mfa: %v", err)
	}
	mc, err := iss.Verify(mfaTok)
	if err != nil {
		t.Fatalf("verify mfa: %v", err)
	}
	if !mc.MFA {
		t.Errorf("mfa flag missing")
	}
}
