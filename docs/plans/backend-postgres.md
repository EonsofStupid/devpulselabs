# The backbone — Postgres as the one backend

**R312 (ruled 2026-11-25).** **Status:** planned in full; BE0 is blocked on
one permission (§7). No code yet. This is built before the
[internal build](internal-build.md) and before the components (M2 to M5 of
[move and organize](organize-code-and-data.md)).

## 1. Goal

One backend holds every relationship in DevPULSE Labs: a lesson to its
project, a project to its chats, a chat to its images, research and notes,
a bot to its provider, and the vectors that let all of it be searched by
meaning. It is Postgres with pgvector. The app starts it and stops it as its
own subprocess, and it can also run as a service or be an installed
Postgres. Files on disk stay grouped in folders that match what the backend
says.

## 2. Audit — what stores data today (read in the code, 2026-11-25)

| Store | Engine | Runs | Holds | Relationships | Where |
|---|---|---|---|---|---|
| Library index | Turso (SQLite-compatible) | Inside the app | One table, `artifacts`: every media file, with show and episode as plain text columns | None. No foreign key anywhere. | `crates/index/src/lib.rs:184` |
| AI accounts | Turso, a second file | Inside the app | `chat_provider_activation`, `chat_provider_settings` | None | `crates/ai/accounts/src/lib.rs:299` |
| AI profiles | Turso, a third file | Inside the app | `chat_provider_profiles` | None | `crates/ai/profiles/src/lib.rs:367` |
| Estate | Fjall key-value store with a Moka cache | Inside the app | Editor documents, undo history, last route, agent session state, as bytes | None; keys are strings | `crates/estate/src/lib.rs` |
| Signal ledger | Parquet files | Writer thread, off unless `--ledger` | Statistics events | None | `crates/ledger/src/lib.rs` |
| Chats | Journal files per project | Files | Chats and drafts | Only by folder: `.usr/.ai/projects/<id>/chats` | `crates/chat/projects/src/lib.rs` |
| Everything else | Plain files, written by 30 crates | Files | Lessons, slides, comps, guides, captures | Only by folder | `crates/doc`, `crates/composer`, others |
| Vectors | None | — | Nothing stores an embedding | — | `crates/vector` draws shapes |

**Findings.**

1. Four engines (Turso three times, Fjall, Parquet, files) and none of them
   records that one thing belongs to another. Grouping exists only as folder
   names and text columns.
2. Three separate database files cannot be joined in one query.
3. Nothing can be written by two processes at once; every engine lives
   inside the app.
4. There is no search by meaning.
5. R276 (2026-10-13) asked for the databases as "dedicated subprocesses
   running alongside devpulse". It was recorded and not acted on.
6. ADR-014 (2026-07-25) rejected Postgres in a sentence that came from no
   operator words. It is withdrawn in BE1's PR.

## 3. Design

### 3.1 Standards followed

| Subject | Standard used |
|---|---|
| Server | PostgreSQL 18, the current long-supported line (to November 2030) |
| Vectors | pgvector, HNSW index, cosine distance |
| Start and stop | Postgres's own tools: `initdb`, `pg_ctl start -w`, `pg_isready`, `pg_ctl stop -m fast` |
| Client | The `postgres` crate (the maintained Rust driver) and the `pgvector` crate for the vector type |
| Schema changes | Numbered, forward-only SQL files applied in order and recorded in a history table (`refinery`) |
| Identity | `uuid` keys made by Postgres 18's built-in `uuidv7()`, which sort by time |
| Time | `timestamptz`, always UTC |
| Names | Lower snake case; plural tables; `<thing>_id` foreign keys; one Postgres schema per domain |
| Access | Listens on this computer only (`127.0.0.1`), SCRAM password login, one app role that cannot create roles or databases |
| Carrying data | A logical dump (`pg_dump`), because a Postgres data folder only works on the OS, chip and major version that made it |

### 3.2 Three ways to run, one address in the code

The app holds one backend address. Nothing above `dpl-backend` knows which
mode made it.

| Mode | Who starts Postgres | Data | Built in |
|---|---|---|---|
| **Carried** | The app, as its subprocess, from the tool depot | In the library | BE1 |
| **Service** | The computer (Windows service, systemd, launchd), set up by one command in the app | Chosen at setup | BE6 |
| **Installed** | Already running; the app is given its address | That server's own | BE6 |

Platforms: Windows x64 first. Linux x64, Linux ARM and macOS in BE6.

### 3.3 Carried mode, exactly

```text
<library>\.usr\.ai\backend\
  backend.json                 mode, Postgres major, schema number
  portable.sql                 logical dump, rewritten on every clean stop
  data\pg18-windows-x64\       the live data folder for this OS and chip
  log\                         Postgres's own log, rotated
  secret                       the generated password for the app role
```

- **First start:** `initdb` makes the data folder with a generated password.
  If `portable.sql` exists (the library came from another computer), it is
  restored into the new folder.
- **Every start:** the app picks a free port, runs `pg_ctl start -w`, checks
  `pg_isready`, then applies any new schema files. The status bar shows
  "Backend: starting", then "ready".
- **Every clean stop:** `pg_dump` rewrites `portable.sql`, then
  `pg_ctl stop -m fast`.
- **After a crash:** Postgres recovers its own data on the next start. A
  leftover `postmaster.pid` from a dead process is cleared by `pg_ctl`.
- **Second app window on the same library:** it reads the port from the
  running server's `postmaster.pid` and connects; it does not start a
  second server.
- **Library unplugged or read-only:** the backend is not started, and the
  app says "This library is not connected" as it does today.
- **A start that fails** shows the reason in plain words with Retry and
  "Open the backend log". The app does not run half-working.

### 3.4 Files and backend: who is the truth

- **Content is files.** Lessons, slides, images, chat journals, notes and
  research documents stay as files in folders.
- **Relationships and vectors are the backend.** Each row points at its
  file by a path relative to the library.
- **Every relationship is also written beside its file** in a small
  `links.json`, so `cargo xtask backend rebuild` can remake every row from
  the files alone. This is the existing rule of the index ("rebuildable by
  design"), kept.
- **Folders match rows.** A project's files are grouped at
  `<library>\.usr\.ai\projects\<project id>\` in `chats\`, `research\`,
  `notes\`, `media\`, `context\` and `logs\`.

### 3.5 Schema, first version

One Postgres schema per domain. Every table has `id uuid primary key
default uuidv7()`, `created_at` and `updated_at`.

| Schema | Table | Key columns | Belongs to |
|---|---|---|---|
| `work` | `projects` | `name`, `folder` | — |
| `work` | `lessons` | `title`, `show_id`, `episode_folder` | `project_id` |
| `library` | `artifacts` | today's 16 columns | `project_id`, `lesson_id` (both optional) |
| `ai` | `providers` | `provider_id`, `kind` (subscription, API key, local) | — |
| `ai` | `provider_activations` | today's columns | `provider_id` |
| `ai` | `provider_settings` | `permission_mode` | `provider_id` |
| `ai` | `provider_profiles` | `display_name`, `endpoint`, `model` | `provider_id` |
| `ai` | `bots` | `name`, `persona`, `instructions` | `provider_profile_id` |
| `chat` | `chats` | `title`, `journal_path` | `project_id`, `bot_id` (optional) |
| `chat` | `messages` | `seq`, `role`, `body`, `sent_at` | `chat_id` |
| `knowledge` | `notes` | `title`, `path` | `project_id`, `lesson_id` (optional) |
| `knowledge` | `sources` | `title`, `url`, `path` (research) | `project_id` |
| `knowledge` | `links` | `from_kind`, `from_id`, `to_kind`, `to_id`, `relation` | any two rows |
| `knowledge` | `chunks` | `owner_kind`, `owner_id`, `seq`, `body` | a message, note or source |
| `knowledge` | `embeddings` | `model`, `embedding vector(n)` | `chunk_id` |
| `app` | `state` | `tree`, `key`, `value bytea` (the estate) | — |

Foreign keys are declared, with `on delete` stated on every one.

### 3.6 Code layout

```text
crates/backend/                  dpl-backend: the only crate that knows Postgres runs as a process
  Cargo.toml
  migrations/
    V0001__work.sql
    V0002__library.sql
    V0003__ai.sql
    V0004__chat.sql
    V0005__knowledge.sql
    V0006__app_state.sql
  src/
    lib.rs                       Backend::open(layout) -> Backend; Backend::client()
    address.rs                   the one address type; reads backend.json
    process/
      mod.rs
      locate.rs                  finds the depot's Postgres, checks its hash
      init.rs                    initdb, password, restore of portable.sql
      start.rs                   free port, pg_ctl start, pg_isready
      stop.rs                    pg_dump, pg_ctl stop
    migrate.rs                   applies migrations/, refuses a newer schema than it knows
    rebuild.rs                   remakes rows from files and links.json
    tests.rs
```

- **Each domain crate owns its own queries** in a `store.rs`:
  `crates/index`, `crates/ai/accounts`, `crates/ai/profiles`,
  `crates/chat/projects`, `crates/estate`. They take a client from
  `dpl-backend` and never start a process or open a connection themselves.
- **All schema files live in `crates/backend/migrations/`**, one history.
- `dpl-backend` sits in the layer stack beside `dpl-roots`;
  `docs/design/architecture.md` and the audit table in
  `crates/xtask/src/main.rs` are amended in BE1's commit.
- The depot gains `postgres` (family `backend`) in
  `dptools-deps/sources.local.toml`, staged by `cargo xtask stage-tool-deps`
  like every other helper. Only `bin`, `lib` and `share` are staged; the
  zip's pgAdmin, docs and debug symbols are left out.

### 3.7 Tests and the gate

- `dpl-backend` tests start a real Postgres on a scratch data folder under
  `target/gate-tests`. Nothing is mocked.
- Both hosted lanes fetch the pinned Postgres by checksum before the test
  phase. How the lanes get it (cache or image) is settled in BE1 and is
  **not verified yet**.
- `cargo green` gains a `backend` phase: start, apply every schema file to
  an empty database, rebuild from a fixture library, stop.

## 4. Milestones (one PR each, in this order)

| | Milestone | Done when |
|---|---|---|
| **BE0** | Proof on this machine: unpack 18.7, build pgvector 0.8.7, start, one relational query, one vector query, clean stop | The output is shown to the operator; inventory row 66 signed off |
| **BE1** | `dpl-backend` carried mode; the app starts and stops it; status bar shows its state; ADR-014's sentence withdrawn; `backend` gate phase | Real exe on a scratch library: start, restart, crash recovery, second window, unplugged library, at 1440×900 and 1100×700 |
| **BE2** | Schema V0001 to V0005; projects, lessons, chats, notes and research written as rows and as `links.json`; `rebuild`; `portable.sql` | Delete the data folder, rebuild, and get identical rows; carry a library to a fresh folder and restore |
| **BE3** | Chunks and embeddings; an embedding helper as a depot subprocess; search by meaning over chats, notes and research | A search on a scratch library returns the right chat, with timings recorded |
| **BE4** | The library index and both AI stores read and write Postgres; their rows are moved once and verified; Turso removed from the workspace | Same library contents before and after; no `turso` in `Cargo.lock` |
| **BE5** | The estate reads and writes `app.state`; rows moved once and verified; Fjall removed | Editor state survives restart; no `fjall` in `Cargo.lock` |
| **BE6** | Service mode and installed mode; Linux x64, Linux ARM and macOS binaries in the depot | Each mode starts, answers and survives a reboot on Windows and Linux; macOS proven when `mac-parity.md` lands |

After BE6 the only stores are Postgres, files, and the Parquet statistics
ledger (kept by R241).

## 5. Rules kept

- One backend. A replaced store is removed in the PR that replaces it.
- A one-time move copies, verifies row counts and contents, and leaves the
  old file untouched until the operator deletes it.
- The backend never listens beyond this computer in carried mode.
- No vendor credential is ever stored in the backend. `ai` tables hold
  non-secret settings only, as today.
- The operator's library content is never changed by a move or a rebuild.
- Proof runs use a scratch library, one at a time.

## 6. Risks

| Risk | Handling |
|---|---|
| pgvector has no official Windows binary; it is built from source with Visual Studio's tools | BE0 proves the build on this machine. Visual Studio 18 is installed here. **Not verified yet.** |
| No official ready-to-unpack Postgres for Linux and macOS from the Windows source | BE6 picks the source and brings it for sign-off. **Not verified yet.** |
| Startup takes longer | Measured in BE1; the status bar shows progress; the window opens while the backend starts |
| Size: the download is 390 MB with pgAdmin, docs and symbols | Only `bin`, `lib`, `share` are staged. The staged size is measured in BE0. **Not measured yet.** |
| Antivirus or a firewall prompt on first start | Loopback only, which Windows does not prompt for. Checked in BE1. |
| Two libraries open at once | Each library has its own data folder and port |
| Hosted lanes need Postgres | Settled in BE1 before any test depends on it |
| A library on a FAT or network drive | Postgres needs a local NTFS, ext4 or APFS folder. BE1 detects this and offers to keep the data folder on this computer, with `portable.sql` in the library |

## 7. What is needed from the operator

1. **One permission.** Unpacking and running the downloaded Postgres was
   stopped twice by this session's permission check. Either run it once
   yourself or allow it:

   ```text
   ! powershell -Command "Expand-Archive -Force 'D:\projects\dptools-deps\artifacts\windows-x86_64\postgres\18.7\postgresql-18.7-1-windows-x64-binaries.zip' 'D:\projects\dptools-deps\artifacts\windows-x86_64\postgres\18.7'"
   ```

   and say that running `postgres`, `initdb`, `pg_ctl` and `psql` from that
   folder on a scratch data folder is allowed.
2. **Sign off inventory row 66** (Postgres 18.7 and pgvector 0.8.7).

## 8. Evidence

- 2026-11-25: no Postgres is installed on this machine (`postgres`, `psql`,
  `pg_ctl` not on the path; no `C:\Program Files\PostgreSQL`).
- 2026-11-25: `postgresql-18.7-1-windows-x64-binaries.zip`, 389,515,503
  bytes, sha256
  `ead0bbc27fb678fa24dcf86b738981323042abd073c744103bf7f3aed23f9a33`,
  downloaded to the depot. Not unpacked, not run.
- 2026-11-25: Postgres 18 is supported to 2030-11-14 and 18.7 is its
  current release; pgvector's latest tag is v0.8.7 (read from the projects'
  own version feeds).
