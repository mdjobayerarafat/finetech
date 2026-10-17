// Seed SQL is embedded here for go:embed.
package db

import _ "embed"

//go:embed seed.sql
var seedSQL string
