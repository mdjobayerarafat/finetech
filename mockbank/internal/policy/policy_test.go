package policy

import "testing"

func TestDecideTransfer(t *testing.T) {
	cases := []struct {
		name       string
		amountFen  int64
		limitFen   int64
		wantTier   string
		wantReason string
	}{
		{"zero_is_red", 0, 1_000_000, Red, "non_positive_amount"},
		{"under_limit", 50_000, 1_000_000, Yellow, "within_daily_limit"},
		{"at_limit", 1_000_000, 1_000_000, Yellow, "within_daily_limit"},
		{"over_limit", 1_000_001, 1_000_000, Red, "over_daily_limit"},
		{"way_over", 5_000_000, 1_000_000, Red, "over_daily_limit"},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			d := DecideTransfer(tc.amountFen, tc.limitFen)
			if d.Tier != tc.wantTier {
				t.Errorf("tier = %q, want %q", d.Tier, tc.wantTier)
			}
			if d.Reason != tc.wantReason {
				t.Errorf("reason = %q, want %q", d.Reason, tc.wantReason)
			}
		})
	}
}

func TestDecideQuery(t *testing.T) {
	d := DecideQuery()
	if d.Tier != Green {
		t.Errorf("query tier = %q, want green", d.Tier)
	}
}

func TestDecideSubscriptionCancel(t *testing.T) {
	d := DecideSubscriptionCancel()
	if d.Tier != Yellow {
		t.Errorf("cancel tier = %q, want yellow", d.Tier)
	}
	if d.Requires != "confirmation" {
		t.Errorf("cancel requires = %q, want confirmation", d.Requires)
	}
}

func TestDecideCardAction(t *testing.T) {
	cases := []struct {
		action string
		tier   string
	}{
		{"issue", Yellow},
		{"freeze", Yellow},
		{"unfreeze", Yellow},
		{"update_limits", Yellow},
		{"report_lost", Red},
		{"unknown_thing", Red},
	}
	for _, tc := range cases {
		t.Run(tc.action, func(t *testing.T) {
			d := DecideCardAction(tc.action)
			if d.Tier != tc.tier {
				t.Errorf("card %s tier = %q, want %q", tc.action, d.Tier, tc.tier)
			}
		})
	}
}
