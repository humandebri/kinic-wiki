# DB Lifecycle

This document describes the operational model for multiple SQLite databases in one VFS canister.

## Identity Model

The database unit is `database_id`.

Principals are attached through `database_members`:

- `owner`: all operations, including grant, revoke, delete, archive, and restore
- `writer`: read and write VFS nodes
- `reader`: read VFS nodes and query/search/list

One database can have multiple principals. One principal can belong to multiple databases.

## Memory Layout

Stable-memory mount IDs are partitioned by purpose:

- `0..9`: WASI filesystem memory for tmp files and directory metadata
- `10`: index DB
- `11..=32767`: user DB slots
- `32768..=65534`: reserved

The index DB tracks database metadata and membership. User DBs hold VFS node data, search data, and link data.
The index DB also stores an internal `usage_events` ledger for update calls.

Hot or restoring DBs consume one active user DB slot. Archived and deleted DBs release their active slot.

## Status

Databases move through four statuses:

- `hot`: mounted and usable for VFS read/write/search/list
- `archived`: not mounted, slot released, snapshot metadata retained
- `deleted`: not mounted, slot released, not restorable unless an external archive was taken first
- `restoring`: mounted for chunk import, VFS operations rejected until finalize succeeds

Only `hot` DBs are available to normal VFS APIs.

## Size Tracking

`logical_size_bytes` tracks the SQLite file size for a database.

It is updated after VFS mutations and restore finalization. It is useful for visibility and planning, but it is not a stable-memory billing or shrink metric.

Deleting or archiving a DB releases the mount slot. It does not imply that canister stable memory shrinks.

## Usage Ledger

`usage_events` records update calls only. Query calls are not recorded.

Each event stores method, database ID when present, caller principal, success flag, observed cycle delta, error text, and timestamp.
The cycle delta is an operational observation from canister balance before and after the update, not a guaranteed one-to-one IC billing statement.

## Delete

`delete_database` is owner-only.

Delete is a soft delete in the index:

- status becomes `deleted`
- active mount ID is cleared
- logical size is set to `0`
- the slot becomes reusable

Delete is treated as irreversible. If recovery is required, archive first and store the exported bytes outside the canister.

## Archive

Archive is a low-level snapshot byte export flow:

1. `begin_database_archive(database_id)` returns the current DB file size.
2. `read_database_archive_chunk(database_id, offset, max_bytes)` exports file bytes by range.
3. Caller stores the bytes outside the canister.
4. `finalize_database_archive(database_id, snapshot_hash)` marks the DB archived and releases the slot.

The canister does not persist archive bytes. The caller owns external storage and retry behavior.

`snapshot_hash` is metadata in v1. The canister stores it but does not verify restored bytes against it.

## Restore

Restore is a low-level snapshot byte import flow:

1. `begin_database_restore(database_id, snapshot_hash, size_bytes)` moves an archived or deleted DB to `restoring` and allocates a new slot.
2. `write_database_restore_chunk(database_id, offset, bytes)` writes imported bytes.
3. `finalize_database_restore(database_id)` checks the file size, runs DB migrations, and returns the DB to `hot`.

Restore can only begin from `archived` or `deleted`. It cannot begin from `hot` or while already `restoring`.

Finalize verifies only that imported SQLite file size equals the `size_bytes` supplied to `begin_database_restore`. Content hash verification is not implemented in v1.

If finalize fails because the file size is wrong, the DB stays `restoring`. The caller can write missing bytes and retry finalize.

## Current Limits

- At most 32757 active user DB slots per canister: mount IDs `11..=32767`.
- Archive/restore APIs are canister-level primitives. The CLI does not yet provide archive export/import commands.
- Caffeine or external object storage integration is out of scope for v1.
