# FinTech Pioneers — Mobile (Flutter)

> Simulation-only mobile client for the FinTech Pioneers AI banking agent.
> Bilingual (English / 中文) · mirrors the Next.js web frontend's surface and
> the Rust backend's `/api/v1` contract.

## Quick start

```bash
# 1. install Flutter 3.47+
flutter --version

# 2. fetch dependencies
flutter pub get

# 3. run on the connected Android emulator / iOS simulator
flutter run
```

The app points at `http://10.0.2.2:8080/api/v1` by default — the Android
emulator's loopback to the host machine. Override it for other platforms:

```bash
flutter run --dart-define=API_BASE=http://localhost:8080/api/v1
flutter run --dart-define=API_BASE=https://staging.fintechpioneers.example/api/v1
```

Demo accounts (seeded by the backend):

| Role | Username | Password |
|---|---|---|
| Customer · Alex | `demo` | `demo1234` |
| Customer · Mei | `intl_student` | `demo1234` |
| Admin | `admin` | `admin1234` |

## What's inside

```
lib/
├── main.dart                  # entrypoint, sets system UI + ProviderScope
├── app.dart                   # MaterialApp, theme, auth-gated routing
├── core/
│   ├── api_client.dart        # Dio + session cookie + CSRF
│   ├── api.dart               # typed API surface + Riverpod providers
│   ├── api_error.dart         # ApiError + friendly error mapper
│   ├── money.dart             # integer-fen helpers (no double ledger math)
│   ├── language.dart          # AppLanguage enum (en, zh)
│   └── i18n.dart              # bilingual string tables
├── models/                    # JSON parsers for every backend resource
│   ├── user.dart, account.dart, transfer.dart, agent.dart
│   ├── card.dart, wealth.dart, subscription.dart, budget.dart
├── state/
│   ├── auth.dart              # AuthNotifier + currentUserProvider
│   └── settings.dart          # language preference (persisted)
├── theme/
│   ├── app_theme.dart         # Material 3 dark + glass tokens
│   └── aurora.dart            # animated backdrop wash
├── screens/
│   ├── login_screen.dart      # bilingual login + demo-account quick fill
│   ├── shell.dart             # bottom nav (Dashboard/Chat/Transfers/Cards/Wealth/More)
│   ├── dashboard_screen.dart  # total balance, accounts, spending pie, recent txns, daily digest
│   ├── chat_screen.dart       # assistant: agent/messages + plan preview + ConfirmationCard + voice
│   ├── transfers_screen.dart  # smart-transfer sheet (preview → confirm via chat)
│   ├── cards_screen.dart      # virtual card lifecycle: issue/freeze/cancel/report-lost
│   ├── wealth_screen.dart     # catalog + holdings
│   ├── more_screen.dart       # subscriptions, budgets, goals, forecast, audit, admin, profile
│   ├── forecast_screen.dart   # "can I afford it?" simulator
│   ├── budgets_screen.dart    # category caps with utilization
│   ├── goals_screen.dart      # savings goals with contribute flow
│   ├── receipt_screen.dart    # camera/gallery → /receipts/scan
│   ├── profile_screen.dart    # user info, language, sign out
│   └── admin_screen.dart      # admin: overview, audit chain, users, safety events
└── widgets/
    ├── glass_card.dart        # frosted-glass container (BackdropFilter)
    ├── tier_badge.dart        # Green / Yellow / Red pill
    ├── confirmation_card.dart # the marquee widget: tier + params + confirm/MFA
    ├── account_card.dart      # account row + transaction tile
    ├── voice_button.dart      # record → /voice/stt → fills chat input
    ├── network_banner.dart    # offline banner via connectivity_plus
    ├── digest_card.dart       # daily digest card on dashboard
    └── empty_state.dart       # reusable "nothing here" placeholder
```

## Architecture

- **State**: Riverpod (`flutter_riverpod`). Auth + settings are global;
  per-screen data is `FutureProvider.autoDispose` against the typed `Api`.
- **HTTP**: Dio with an interceptor that attaches `Cookie: bp_session=…`
  and `X-CSRF-Token` automatically on state-changing calls. Session is
  persisted via `shared_preferences`.
- **Money**: every server amount is integer fen (`amount_fen`); the UI
  layer is the only place that ever calls `NumberFormat`. No `double` in
  ledger math.
- **Risk tiers**: a `RiskTier` enum (Green/Yellow/Red) shared by the
  confirmation card, transfer sheet, and card actions. Yellow gets a
  one-tap confirm; Red gets a 6-digit mock-MFA sheet.
- **Bilingual**: a flat-key dictionary (`Tr.of(lang).t('chat.title')`)
  keeps the two languages in lock-step. The same key is reused on every
  screen so adding a string touches both tables.
- **Simulation banner**: every screen that touches money carries a
  one-line "Simulation only — no real bank accounts" reminder. The
  login hero makes it more prominent.

## iOS / Android specifics

- **iOS**: `Info.plist` declares `NSMicrophoneUsageDescription`,
  `NSCameraUsageDescription`, `NSPhotoLibraryUsageDescription`.
- **Android**: `AndroidManifest.xml` declares `RECORD_AUDIO`, `CAMERA`,
  `READ_MEDIA_IMAGES`, `INTERNET`, `ACCESS_NETWORK_STATE`. Launch
  background is the brand's deep-indigo color.

## Tests

```bash
flutter test
```

Smoke test boots the app and checks the login screen renders. Per-screen
tests can be added under `test/` as the surface grows.

## Limitations

- Voice STT sends raw bytes — the backend's `/voice/stt` is a stub.
- Receipt scan is best-effort; the backend's `/receipts/scan` falls back
  to a deterministic mock if the LLM call fails.
- No real biometrics, push notifications, or secure storage — this is a
  simulation build, not a production banking app.
