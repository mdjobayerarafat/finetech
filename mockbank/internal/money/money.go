// Package money contains fen helpers. The ledger only ever sees integer fen.
package money

// Fen is a CNY amount in minor units (1/100 yuan). Never use floats.
type Fen int64

// FromYuan converts a yuan amount (e.g. 12.34) to fen. Use sparingly: prefer
// calling code that already works in fen.
func FromYuan(yuan float64) Fen { return Fen(int64(yuan * 100)) }

// Yuan returns the fen amount as a float yuan, for JSON display only.
func (f Fen) Yuan() float64 { return float64(f) / 100 }
