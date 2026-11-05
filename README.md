# FAT — Local-first Workout Tracker

A personal workout tracker written in Rust. The UI is built with
[Slint](https://slint.dev/), data is stored in SQLite through `rusqlite`, and
the application runs fully offline. There is no JavaScript runtime, web server,
or cloud dependency.

## Download

Android (arm64, Android 8.0 or newer):

**[Download FAT 0.1.0 APK](dist/fat-0.1.0-arm64.apk)** — 9.6 MB

SHA-256: `52caaf2d95b986f33ac120c7658962ef194f6668c82c4750f37bdca62dd0d261`

The APK is release-signed and verified with APK Signature Scheme v2/v3. See
[Android](#android) for build, signing, and verification details.

## Features

- **Set-level workout logging** with weight, complete reps, partial reps, rest,
  RIR (reps in reserve), and free-form notes.
- **One workout day per calendar date** with stable per-day sequence numbers.
  Deleting a set never renumbers the remaining sets.
- **Exercise library** with categories, primary/secondary muscle assignments,
  and equipment. The catalogue is pre-seeded and user-extensible.
- **Profile and body measurements** stored as append-only snapshots. BMI is
  always derived from weight and height recorded in the *same* snapshot.
- **Analytics** for weight progression, volume, weekly volume, muscle
  distribution, and exercise frequency. Charts use recorded observations only;
  missing days are never interpolated or synthesized.
- **Muscle-aware volume attribution**: primary muscles receive 100% of a set's
  volume and secondary muscles receive 50%, reported separately.
- **Versioned migrations and backups**, with independent application and
  database schema versions.
- **Update checks** with semver comparison, release notes, checksum and optional
  Ed25519 signature verification, staged packages, pre-install database backups,
  and explicit rollback packages.

## Screens

The five primary sections are:

1. **Workout** — today's (or a selected date's) sets, add/edit/delete forms.
2. **History** — workout days with set counts, volume, and exercise names.
3. **Analytics** — period filters, exercise/muscle filters, chart selection.
4. **Library** — exercises, muscle groups, and equipment management.
5. **Profile** — user details, measurement snapshots, comparisons, BMI.

## Building and running

Requirements:

- Rust 1.92 or newer
- A Linux desktop with X11 or Wayland (validated), or another Slint-supported
  desktop platform
- Runtime libraries for the windowing stack. On Debian/Ubuntu, a typical
  install includes `libfontconfig1`, `libxkbcommon0`, and the usual X11/Wayland
  client libraries. The `fontique` dependency uses `fontconfig-dlopen`, so
  fontconfig development headers are not required to compile.

```bash
cargo build --release
cargo run --release
```

### Data directory

By default the app uses the platform local data directory:

```text
<data-local-dir>/fat-workout/
```

Override it for development or portable use:

```bash
FAT_DATA_DIR=/path/to/data cargo run
```

Files inside the data directory:

```text
workout.db                    SQLite database (WAL mode)
workout.db-wal / -shm         SQLite WAL files
backups/                      database backups created manually/for migrations
updates/
  config.json                 update configuration (created on first launch)
  state.json                  cached check/staging/install state
  latest-manifest.json        last fetched release manifest
  packages/                   downloaded, verified packages and release metadata
  backups/                    pre-update database backups
```

The database is local to the user. Nothing is uploaded by the app.

## Usage notes

- Workout dates cannot be in the future and must be on or after 1900-01-01.
- Weight is stored in kilograms as a REAL value.
- Effective reps are `complete_reps + partial_reps / 2.0`.
- Volume is `weight × effective_reps`.
- RIR is optional and must be between 0 and 10. Rest is optional and
  non-negative.
- Sets are ordered by `(date, sequence_number)`.
- Reference rows (exercises, muscles, equipment) are never physically deleted
  when referenced by history. They can be archived/deactivated instead.
- `No Equipment` (equipment id `1`) is reserved: it cannot be deleted or
  deactivated, and its name cannot be changed.
- Body measurements are append-only. The latest value for each metric is kept
  independently, but BMI is only shown for snapshots containing both weight and
  height.

## Architecture

```text
ui/app.slint              Slint markup, custom components, dark theme
src/ui/mod.rs             Controller: callbacks, formatting, background update worker
src/ui/charts.rs          Software line/bar chart renderer
src/ui/format.rs          Presentation-only formatting helpers

src/services/             Validation, workflows, calculations
  workout_service.rs      Set add/edit/delete, history, sequence preview
  library_service.rs      Exercise/muscle/equipment CRUD and guard rails
  profile_service.rs      Profile, measurements, BMI, comparisons, chart series
  analytics_service.rs    Report building and muscle-volume attribution
  update_service.rs       Check, download, verify, stage, install, rollback

src/database/             SQLite access
  schema.rs               Schema v1 DDL and required tables
  migrations.rs           Versioned migrations, validation, backup/restore, downgrade
  seed.rs                 Idempotent predefined catalogue
  backup.rs               Vacuum-into backups and integrity validation
  repositories/           Parameterized SQL for each aggregate

src/models/               Domain types
src/updates/              HTTPS/file fetch, checksums, signatures
src/utils/                Dates, text normalization, calculations
tests/end_to_end.rs       Cross-layer integration tests
```

The UI only parses text and routes intent. Validation, calculations, and
persistence remain in services/repositories. Repositories call
`ensure_foreign_keys` before operating, so a connection can never silently run
without FK enforcement.

## Database and migrations

The authoritative schema version is SQLite's `PRAGMA user_version`; applied
migrations are also recorded in the `schema_migrations` audit table. Schema and
application versions are independent: currently the app is `0.1.0` and the
schema is `1`.

Rules enforced in depth:

- Foreign keys are enabled on every connection (`PRAGMA foreign_keys = ON`) and
  checked before operations.
- WAL journaling, a 10-second busy timeout, and `synchronous = NORMAL`.
- Foreign-key restrictions protect historical workout rows.
- Check constraints cover dates, weights, rep counts, RIR, body-fat percentage,
  measurement ranges, categories, and active flags.
- Case-insensitive unique indexes prevent duplicated exercise/equipment/muscle
  names while preserving the user's preferred capitalization.
- Seeds use `app_metadata` markers, so renaming or deactivating a seeded record
  does not cause it to be recreated on the next launch.

Migrations run as `IMMEDIATE` transactions. Existing databases are backed up and
validated before migration; a failed migration is rolled back transactionally,
then the pre-migration backup is restored. Downgrades require every intervening
migration to declare a safe down operation. The initial schema migration is
intentionally irreversible, so a v1 database cannot be automatically
downgraded to v0.

## Update system

Updates are optional and disabled until a release source is configured. On first
launch the app creates `<data-dir>/updates/config.json`:

```json
{
  "release_source": "https://example.com/fat/releases.json",
  "public_key_hex": "0123456789abcdef...64 hex characters...",
  "installer_command": "xdg-open {package}",
  "allow_file_urls": false,
  "manifest_sha256": "optional SHA-256 pin for releases.json"
}
```

Fields:

| Field | Meaning |
| --- | --- |
| `release_source` | HTTPS URL of the JSON release manifest. Empty disables checks. |
| `public_key_hex` | Ed25519 publisher public key. When set, package signatures are mandatory. |
| `installer_command` | Optional desktop command; `{package}` is replaced by the verified absolute path. |
| `allow_file_urls` | Allows `file://` sources for offline/air-gapped installs. |
| `manifest_sha256` | Optional SHA-256 pin for the manifest itself. |

Plain `http://` sources are always rejected. The manifest and packages have
size limits, downloads stream to a `.part` file, and the SHA-256 checksum is
verified before the file is promoted. If a public key is configured, packages
without a valid signature are rejected. Packages are re-verified before
installation, so on-disk tampering after download is detected.

Example manifest:

```json
{
  "version": "0.2.0",
  "notes": "Adds custom chart ranges.",
  "released_at": "2026-11-05T12:00:00Z",
  "schema_min": 1,
  "schema_max": 2,
  "artifacts": {
    "linux-x86_64": {
      "url": "https://example.com/fat/fat-0.2.0-linux-x86_64.tar.gz",
      "sha256": "64 lowercase hex characters",
      "signature": "128 hex characters (required when public_key_hex is set)",
      "size_bytes": 12345678,
      "file_name": "fat-0.2.0-linux-x86_64.tar.gz"
    }
  },
  "rollback": {
    "version": "0.1.0",
    "notes": "Return to the previously installed release.",
    "released_at": "2026-10-01T12:00:00Z",
    "schema_min": 1,
    "schema_max": 1,
    "artifacts": {
      "linux-x86_64": {
        "url": "https://example.com/fat/fat-0.1.0-linux-x86_64.tar.gz",
        "sha256": "64 lowercase hex characters",
        "signature": "128 hex characters",
        "size_bytes": 12345678
      }
    }
  }
}
```

Platform keys currently recognized include `linux-x86_64`, `linux-aarch64`,
`android-arm64`, `android-x86_64`, `macos-x86_64`, `macos-aarch64`, and
`windows-x86_64`.

### Installation and rollback limitations

- The app **never silently replaces itself**. After a package is verified, the
  database is backed up and the package is staged. If `installer_command` is
  set, that command is launched; otherwise the user is given the staged path.
- Android installation is delegated to the host/system package installer. The
  host activity must expose the staged file through a `FileProvider` content
  URI. Android may reject a downgrade based on the version code even when the
  rollback package itself is valid.
- Rollback downloads an older, explicitly signed package from the manifest's
  `rollback` field. It backs up the database again, but restoring the database
  itself is a separate operator decision. An app rollback cannot undo
  irreversible schema migrations automatically.
- Updates are offline-safe: a failed check/download/verification leaves the
  installed application and database untouched.

## Testing

```bash
cargo test              # library, integration, and doc tests
cargo clippy --all-targets
```

The suite covers:

- calculations: partial reps, volume, RIR validity, BMI;
- date/period resolution and week boundaries;
- DB constraints, foreign-key enforcement, reserved equipment;
- schema creation, version rejection, migration backup/restore, downgrade
  refusal for irreversible migrations;
- workout sequencing, deletion without renumbering, future-date rejection,
  inactive reference handling;
- measurement append-only behavior and same-snapshot BMI;
- analytics actual-date behavior and primary/secondary muscle attribution;
- update manifest validation, HTTPS/file restrictions, download hashing,
  checksum failures, signature requirements/tampering, schema compatibility,
  pre-install staging, and post-download tampering.

## Android

The app also builds as a standalone Android package for 64-bit ARM devices.
Slint uses its Android activity backend and Skia renderer; everything else
(SQLite, services, charts, update verification) is the same Rust code as the
desktop build.

### Toolchain

Validated with:

- JDK 17
- Android SDK platform 35 and build-tools 35.0.0
- Android NDK 27.2.12479018
- Rust target `aarch64-linux-android`
- `cargo-apk` 0.10.0

Install the Rust target and packager:

```bash
rustup target add aarch64-linux-android
cargo install cargo-apk --locked
```

The build avoids desktop-only dependencies on Android: `fontique` is a
Linux-only dependency, while Android gets `backend-android-activity-06` and
Slint's Skia renderer.

### Build

Use the helper script with the SDK installed under `~/Android`:

```bash
scripts/build-android.sh release
```

It writes:

```text
target/release/apk/fat.apk      cargo-apk output
dist/fat-0.1.0-arm64.apk        convenience copy
```

The script expects `JAVA_HOME`, `ANDROID_HOME`, and `ANDROID_NDK_ROOT` to point
at the toolchain, with defaults of `~/Android/jdk-17`, `~/Android/sdk`, and
`~/Android/sdk/ndk/27.2.12479018`. A debug build is also available:

```bash
scripts/build-android.sh debug
```

Building from scratch compiles Skia and can take several minutes.

The generated manifest targets:

| Property | Value |
| --- | --- |
| Package | `com.himanshu.fat` |
| Version | `0.1.0` |
| Minimum SDK | 26 |
| Target SDK | 35 |
| ABI | `arm64-v8a` |
| Activity | `android.app.NativeActivity` |
| Orientation | portrait |

The Android entry point is `android_main` in `src/lib.rs`. It resolves
`AndroidApp::internal_data_path()` and passes that app-private directory to the
same UI controller used on desktop, so the database and update files stay inside
the app sandbox and no storage permission is required. The native library is
linked with 16 KiB ELF segment alignment for newer Android devices, and the
manifest requests the `INTERNET` permission only for optional HTTPS update
checks.

### Release signing

`cargo-apk` requires an explicit key for release builds. The helper script uses:

```text
~/.android/fat-release.keystore
~/.android/fat-release.keystore.pass
```

Override them with `CARGO_APK_RELEASE_KEYSTORE` and
`CARGO_APK_RELEASE_KEYSTORE_PASSWORD`, or set
`CARGO_APK_RELEASE_KEYSTORE_PASSWORD_FILE` to a different password file. Keep
the keystore and password out of version control: Android only accepts updates
signed by the same key. The debug profile instead falls back to the standard
`~/.android/debug.keystore`.

### Verification

```bash
apksigner verify --verbose dist/fat-0.1.0-arm64.apk
aapt dump badging dist/fat-0.1.0-arm64.apk | head
```

The release APK is verified with APK Signature Scheme v2 and v3. Runtime
behaviour has not been validated on a physical device or emulator in this
development environment; installation requires an arm64 device with Android 8.0
(API 26) or newer:

```bash
adb install -r dist/fat-0.1.0-arm64.apk
```

For in-app updates, the staged package must be exposed through an Android
`FileProvider` content URI and the system package installer given temporary read
access. Android may reject a version-code downgrade even when the rollback
package is correctly signed.

## License

MIT.
