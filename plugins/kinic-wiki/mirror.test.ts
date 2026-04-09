import test from "node:test";
import assert from "node:assert/strict";

import {
  parseMirrorFrontmatter,
  serializeMirrorFile,
  stripManagedFrontmatter
} from "./frontmatter";
import { findDeletedTrackedNodes } from "./mirror_logic";
import { parsePluginSettings, TrackedNodeState } from "./types";

test("managed mirror frontmatter roundtrips path and etag", () => {
  const content = serializeMirrorFile({
    path: "/Wiki/nested/bar.md",
    kind: "file",
    etag: "etag-1",
    updated_at: 42,
    mirror: true
  }, "# Bar\n");

  const frontmatter = parseMirrorFrontmatter(content);
  assert.deepEqual(frontmatter, {
    path: "/Wiki/nested/bar.md",
    kind: "file",
    etag: "etag-1",
    updated_at: 42,
    mirror: true
  });
  assert.equal(stripManagedFrontmatter(content).trim(), "# Bar");
});

test("plugin settings preserve tracked nodes", () => {
  const settings = parsePluginSettings({
    replicaHost: "http://127.0.0.1:8000",
    canisterId: "aaaaa-aa",
    mirrorRoot: "Wiki",
    autoPullOnStartup: true,
    lastSnapshotRevision: "snap-1",
    lastSyncedAt: 42,
    trackedNodes: [{
      path: "/Wiki/foo.md",
      kind: "file",
      etag: "etag-1"
    }]
  });

  assert.equal(settings.trackedNodes.length, 1);
  assert.equal(settings.trackedNodes[0].etag, "etag-1");
});

test("deletedTrackedNodes does not treat broken frontmatter files as deleted", () => {
  const trackedNodes: TrackedNodeState[] = [{
    path: "/Wiki/foo.md",
    kind: "file",
    etag: "etag-1"
  }];
  const deleted = findDeletedTrackedNodes(
    trackedNodes,
    (remotePath) => remoteToLocalPath("Wiki", remotePath),
    (localPath) => localPath === "Wiki/foo.md"
  );

  assert.deepEqual(deleted, []);
});

test("deletedTrackedNodes returns only tracked nodes whose local file is missing", () => {
  const trackedNodes: TrackedNodeState[] = [
    { path: "/Wiki/foo.md", kind: "file", etag: "etag-1" },
    { path: "/Wiki/missing.md", kind: "file", etag: "etag-2" }
  ];
  const deleted = findDeletedTrackedNodes(
    trackedNodes,
    (remotePath) => remoteToLocalPath("Wiki", remotePath),
    (localPath) => localPath === "Wiki/foo.md"
  );

  assert.deepEqual(deleted, [trackedNodes[1]]);
});

test("deletedTrackedNodes keeps normal mirror files out of delete set", () => {
  const trackedNodes: TrackedNodeState[] = [{
    path: "/Wiki/nested/bar.md",
    kind: "file",
    etag: "etag-1"
  }];
  const deleted = findDeletedTrackedNodes(
    trackedNodes,
    (remotePath) => remoteToLocalPath("Wiki", remotePath),
    (localPath) => localPath === "Wiki/nested/bar.md"
  );

  assert.deepEqual(deleted, []);
});

function remoteToLocalPath(mirrorRoot: string, remotePath: string): string {
  const normalized = remotePath.replace(/\/+/g, "/");
  if (!normalized.startsWith("/Wiki")) {
    throw new Error(`unsupported remote path outside /Wiki: ${remotePath}`);
  }
  return `${mirrorRoot}/${normalized.replace(/^\/Wiki\/?/, "")}`.replace(/\/+/g, "/").replace(/\/$/, "");
}
