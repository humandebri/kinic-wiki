# Dependency Forks

This project patches selected IC stable-memory crates while multi-database canister storage depends on widened mount IDs.

## Current patches

- `ic-stable-structures`: `https://github.com/humandebri/stable-structures`, rev `a24a7d7572e36eda104abd8280946aeb7ac4e060`
- `stable-fs`: `https://github.com/humandebri/stable-fs`, rev `677e54471d683a6a988234dc1675b499ebed23d9`
- `ic-wasi-polyfill`: `https://github.com/humandebri/ic-wasi-polyfill`, rev `4a9b462e2e382bec6505f9c98bb4daa145317093`

## Reason

The multi-database VFS canister needs one index DB plus many per-database SQLite files. The upstream mount ID shape is too small for the target layout. These forks widen mount IDs from `u8` to `u16`, allowing user DB mount IDs `11..=32767`.

## Mainline requirement

Before merging this dependency strategy into a long-lived mainline branch, move the forks under the ICME-Lab organization or replace them with upstream releases that contain the mount ID widening.

Track upstream PRs for the widening and keep each pinned revision tied to a reviewed diff. Do not update these revisions mechanically.

## Update checks

When changing any patched revision:

- verify mount IDs above `255` can be allocated and used for a database file
- run `./.local/check.sh`
- confirm `crates/vfs_canister/vfs.did` still matches the generated Candid interface
- review archive/restore smoke coverage because restore allocates a fresh mount ID
