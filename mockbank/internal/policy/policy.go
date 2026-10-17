// Package policy implements the Green / Yellow / Red tier decision.
// Mirrors backendrust/src/policy.rs so the two backends agree.
package policy

// Tier values.
const (
	Green  = "green"
	Yellow = "yellow"
	Red    = "red"
)

// Decision is the result of evaluating a banking action.
type Decision struct {
	Tier     string
	Reason   string
	Requires string // confirmation, mfa, both
}

// DecideTransfer picks a tier based on the daily-limit threshold.
// The caller is responsible for verifying the safety lock and the user's
// risk profile before applying this decision.
func DecideTransfer(amountFen, dailyLimitFen int64) Decision {
	if amountFen <= 0 {
		return Decision{Tier: Red, Reason: "non_positive_amount", Requires: "mfa"}
	}
	if amountFen > dailyLimitFen {
		return Decision{Tier: Red, Reason: "over_daily_limit", Requires: "mfa"}
	}
	return Decision{Tier: Yellow, Reason: "within_daily_limit", Requires: "confirmation"}
}

// DecideQuery is Green: balance, history, statements.
func DecideQuery() Decision { return Decision{Tier: Green, Reason: "read_only"} }

// DecideSubscriptionCancel is Yellow: user confirmation only.
func DecideSubscriptionCancel() Decision {
	return Decision{Tier: Yellow, Reason: "subscription_cancel", Requires: "confirmation"}
}

// DecideCardAction maps a card kind to a tier.
func DecideCardAction(kind string) Decision {
	switch kind {
	case "issue", "freeze", "unfreeze", "update_limits":
		return Decision{Tier: Yellow, Reason: "card_" + kind, Requires: "confirmation"}
	case "report_lost":
		return Decision{Tier: Red, Reason: "card_report_lost", Requires: "mfa"}
	}
	return Decision{Tier: Red, Reason: "unknown_card_action", Requires: "mfa"}
}
