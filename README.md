# FAT — Workout Tracker

A local-first workout tracker for logging sets, reviewing training history,
understanding progress, and maintaining an exercise library. This branch is a
web rewrite of the original Rust desktop app: the domain rules and five primary
screens are preserved, with a Flask API, SQLite storage, and a Vue 3 interface.

The original Rust implementation remains on the `main` branch. The `python-vue`
branch replaces it at the repository root with this Flask + Vue application.

## Features

- **Set-level workout logging** with weight (kg), complete reps, partial reps,
  optional rest, optional RIR (0–10), and free-form notes.
- **Stable per-day sequence numbers.** Deleting a set never renumbers the
  remaining sets for that date.
- **One workout day per calendar date**, with add/edit/delete flows for past
  and present dates. Future workout dates are rejected.
- **Exercise library** with categories, primary/secondary muscle assignments,
  equipment, search, and active/inactive management.
- **Reference data protection.** Exercises, muscles, and equipment are never
  deleted; they are deactivated instead. `No Equipment` is reserved at equipment
  id `1` and can never be deleted, renamed, or deactivated.
- **Append-only body measurements.** Each snapshot records only the values that
  were measured. BMI is derived only when weight and height appear in the same
  snapshot.
- **Analytics** for weight progression, volume, weekly volume, muscle
  distribution, exercise frequency, and RIR trends.
- **Muscle-aware volume attribution:** primary muscles receive 100% of a set's
  volume, secondary muscles receive 50%.
- **Real observations only.** Charts and reports never interpolate or synthesize
  missing dates.
- **Accounts and roles.** Session-based sign-in with `admin` and `user` roles.
  Every workout, body measurement, history view, and analytics report is scoped
  to the signed-in account.
- **Admin-managed library.** Administrators create and maintain exercises,
  muscle groups, and equipment. Members have read-only access to the shared
  catalogue.
- **Account administration.** Administrators can create accounts, reset
  passwords, change roles, and deactivate users without deleting their history.
- **Local-first storage.** Data lives in SQLite on the machine serving the app.
  Passwords are hashed with Werkzeug and sessions use an HTTP-only cookie.

## Screens

1. **Workout** — today's (or a selected date's) sets, with add/edit/delete forms.
2. **History** — a searchable archive of workout days with set counts, volume,
   and exercise names.
3. **Analytics** — period and custom-range filters, exercise/muscle filters,
   summary metrics, and custom SVG/CSS charts.
4. **Library** — exercises, muscle groups, and equipment. Administrators manage
   the catalogue; members browse it read-only.
5. **Profile** — account details, password change, latest metrics, comparisons,
   measurement trends, and append-only snapshot history.
6. **Users** *(administrators only)* — account creation, roles, password resets,
   and activation/deactivation.

## Accounts and roles

Two accounts are seeded when they are missing:

| Username | Default password | Role |
| --- | --- | --- |
| `admin` | `admin123` | Administrator |
| `athlete` | `athlete123` | Member |

Change these defaults, especially before exposing the app through a tunnel:

```bash
export FAT_ADMIN_USERNAME=admin
export FAT_ADMIN_PASSWORD='a-long-unique-password'
export FAT_USER_USERNAME=athlete
export FAT_USER_PASSWORD='another-long-password'
```

Seeding only creates accounts that do not exist. It never renames an existing
account or resets a live password on restart.

- **Administrators** can manage the shared exercise library and the user list.
  They also keep their own private workouts and measurements.
- **Members** can sign in, log their own sets and body measurements, and view
  their own history and analytics. They cannot change the shared library or
  manage accounts.
- Accounts are deactivated rather than deleted, preserving all historical rows.
  At least one active administrator must remain.
- Signed-in users can change their password from the Profile screen. An
  administrator can reset a password from the Users screen.

## Stack

- Python 3.12+ with Flask, Flask-SQLAlchemy, and SQLAlchemy
- SQLite, with foreign keys and check constraints enabled
- Vue 3, Vue Router, Axios, Vite
- `uv` for Python environments
- `pnpm` for frontend dependencies
- Custom SVG/CSS charts (no charting framework)
- Cloudflare Tunnel for temporary public access

## Quick start

Requirements:

- Python 3.12 or newer
- [`uv`](https://docs.astral.sh/uv/)
- Node.js 20 or newer and [`pnpm`](https://pnpm.io/)
- `curl` (for the helper scripts)
- `cloudflared` (only for the tunnel)

Install dependencies:

```bash
uv sync
pnpm --dir frontend install
```

### Development

Run both servers with the helper script:

```bash
./devstack.sh
```

Then open <http://127.0.0.1:5173>. Vite proxies `/api` to the Flask server on
port `5000`.

Or run them separately:

```bash
# terminal 1
uv run python -m backend.app

# terminal 2
pnpm --dir frontend dev
```

Vite's proxy target can be changed with `VITE_API_PROXY`:

```bash
VITE_API_PROXY=http://127.0.0.1:5001 pnpm --dir frontend dev
```

### Production-style local server

Flask serves the built Vue app from `frontend/dist`:

```bash
pnpm --dir frontend build
uv run python -m backend.app
```

Open <http://127.0.0.1:5000>.

`FAT_HOST` and `FAT_PORT` control the bind address. For a directly reachable
server (not a tunnel), use `FAT_HOST=0.0.0.0`.

### Session configuration

Set a strong, stable `SECRET_KEY` so sessions survive restarts and cannot be
forged:

```bash
export SECRET_KEY="$(python -c 'import secrets; print(secrets.token_urlsafe(48))')"
```

When the app is served over HTTPS (for example through a Cloudflare tunnel), set
`FAT_COOKIE_SECURE=1` so the session cookie is marked `Secure`. Leave it unset
for plain `http://127.0.0.1` development, otherwise the browser will not send the
cookie over HTTP. Sessions are HTTP-only and `SameSite=Lax` by default.

## Cloudflare Tunnel

`cloudflared` quick tunnels are the fastest way to expose a local instance. The
URL is ephemeral and remains available only while `cloudflared` is running.

With the app already running on port `5000`:

```bash
cloudflared tunnel --url http://127.0.0.1:5000
```

Or use the helper, which builds the frontend if needed, starts the backend when
it is not already healthy, and opens the tunnel:

```bash
./scripts/tunnel.sh
```

The `https://<random>.trycloudflare.com` address is printed by `cloudflared`.
Share that URL only with people who should have an account: the login page is
public, but workout data requires signing in. Quick tunnels expose the app
publicly to anyone with the link, so treat the URL as a secret while it is
active and use strong account passwords.

For a stable hostname, configure a named Cloudflare Tunnel and point its public
hostname at `http://127.0.0.1:5000`. Named tunnels use your Cloudflare account's
DNS and access controls; see the Cloudflare Tunnel documentation for setup.

## Data

By default the SQLite database is created at:

```text
instance/workout.db
```

Override it with `DATABASE_URL`, for example:

```bash
DATABASE_URL=sqlite:////tmp/fat-demo.db uv run python -m backend.app
```

The schema, seed catalogue, and default accounts are created automatically on
first start. Seed markers keep the predefined exercises, muscle groups, and
equipment from being recreated after an administrator renames or deactivates
them.

Databases created before role-based accounts are migrated in place on startup:
the existing profile becomes the administrator account (`id` `1`), legacy
workout rows are assigned to it, and the authentication columns and per-user
workout key are added without deleting history. Back up the database file before
upgrading production data.

To reset the local database during development (this deletes all logged data):

```bash
rm -f instance/workout.db
uv run python -m backend.app
```

Back up the file instead of deleting it if you want to keep your history.

## Domain rules

- Weight is stored in kilograms as a floating-point value.
- Effective reps are `complete_reps + partial_reps / 2.0`.
- Set volume is `weight × effective_reps`.
- RIR is optional and must be between `0` and `10`.
- Rest is optional and must be non-negative.
- A set must contain at least one complete or partial rep.
- Workout dates cannot be in the future and must be on or after `1900-01-01`.
- Sets are ordered by `(user_id, date, sequence_number)`; sequence numbers are
  allocated per account and per day and never reused or renumbered after
  deletion.
- Primary muscle volume is attributed at `100%`; secondary at `50%`. The two
  components remain separate in analytics data.
- Body measurements are append-only. The latest value for each metric is
  selected independently, but BMI only comes from a snapshot containing both
  weight and height.
- Charts use actual records only. Empty dates and weeks are omitted.
- Inactive exercises and equipment remain valid references on historical sets
  but cannot be selected for new ones.
- `No Equipment` (id `1`) is reserved and protected in both the service layer and
  SQLite triggers.

## Tests

```bash
uv run pytest
```

The suite covers calculations, sequence allocation and deletion, validation,
library guard rails, reserved equipment protection, measurement snapshots and
BMI rules, analytics attribution, weekly grouping, filters, empty ranges,
session authentication, role-based access, per-user isolation, account
management, and in-place migration of pre-RBAC databases.

## API overview

All API routes are under `/api` and return JSON.

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/api/health` | Health check |
| `POST` | `/api/auth/login` | Sign in and open a session |
| `POST` | `/api/auth/logout` | End the current session |
| `GET` | `/api/auth/me` | Current signed-in user |
| `PUT` | `/api/auth/password` | Change the current password |
| `GET` | `/api/admin/users` | List accounts (administrators only) |
| `POST` | `/api/admin/users` | Create an account (administrators only) |
| `PUT` | `/api/admin/users/<id>` | Update role, password, or active state |
| `GET` | `/api/workouts?date=YYYY-MM-DD` | Workout day and its sets |
| `GET` | `/api/workouts/history` | Workout day summaries |
| `POST` | `/api/workouts` | Add a set |
| `PUT` | `/api/workouts/<date>/<sequence>` | Update a set |
| `DELETE` | `/api/workouts/<date>/<sequence>` | Delete a set |
| `GET` | `/api/library/exercises` | Exercise catalogue |
| `POST` | `/api/library/exercises` | Create an exercise (administrators only) |
| `PUT` | `/api/library/exercises/<id>` | Update an exercise (administrators only) |
| `PATCH` | `/api/library/exercises/<id>/active` | Activate/deactivate (administrators only) |
| `GET` | `/api/library/muscle-groups` | Muscle groups |
| `POST` | `/api/library/muscle-groups` | Create a muscle group (administrators only) |
| `PUT` | `/api/library/muscle-groups/<id>` | Update a muscle group (administrators only) |
| `PATCH` | `/api/library/muscle-groups/<id>/active` | Activate/deactivate (administrators only) |
| `GET` | `/api/library/equipment` | Equipment catalogue |
| `POST` | `/api/library/equipment` | Create equipment (administrators only) |
| `PUT` | `/api/library/equipment/<id>` | Update equipment (administrators only) |
| `PATCH` | `/api/library/equipment/<id>/active` | Activate/deactivate (administrators only) |
| `GET` | `/api/profile` | Profile, latest metrics, comparisons, snapshots |
| `PUT` | `/api/profile` | Update the signed-in profile |
| `POST` | `/api/profile/measurements` | Append a measurement snapshot |
| `GET` | `/api/profile/series?metric=weight&period=all` | Measurement trend |
| `GET` | `/api/analytics?period=30` | Analytics report for the signed-in user |

All routes except `/api/health`, `/api/auth/login`, and `/api/auth/logout`
require an authenticated session cookie. Admin-only routes return `403` for
members. Workout, measurement, history, and analytics responses only contain
the signed-in account's records.

Library list endpoints accept `include_inactive=true` and `search=...`.
Analytics accepts `period=7|30|90|6m|12m|all|custom`, `start`, `end`,
`exercise_id`, and `muscle_group_id`.

## Project layout

```text
backend/
  app.py                     Flask entry point
  config.py                  Runtime configuration
  application/
    __init__.py              App factory, SPA serving, error handling
    auth.py                  Session loading and role guards
    models.py                SQLAlchemy schema and constraints
    bootstrap.py             Schema creation, migrations, triggers, seed bootstrap
    data_seed.py             Predefined catalogue and default accounts
    calculations.py          Effective reps, volume, BMI
    services/                Workout, library, profile, analytics, auth logic
    routes/                  JSON API blueprints (auth, users, workouts, library, profile, analytics)
frontend/
  src/
    api/                     Axios API modules
    components/              Shared UI, modals, custom charts
    layouts/                 Desktop sidebar and mobile navigation
    stores/auth.js           Client-side account state and actions
    views/                   Login, Workout, History, Analytics, Library, Profile, Users
    assets/theme.css         Design tokens and shared styles
  vite.config.js             Dev proxy and build configuration
tests/                       Pytest domain, API, auth, and migration tests
devstack.sh                  Start Flask + Vite together
scripts/tunnel.sh            Start/build the app and open a quick tunnel
```

## License

This project is for personal use. See the repository history for the original
Rust implementation and its supporting material.
