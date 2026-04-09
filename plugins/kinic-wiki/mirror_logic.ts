// Where: plugins/kinic-wiki/mirror_logic.ts
// What: Pure helpers for mirror deletion detection.
// Why: Deletion checks should be testable without the Obsidian runtime module.
import { TrackedNodeState } from "./types";

export function findDeletedTrackedNodes(
  trackedNodes: TrackedNodeState[],
  toLocalPath: (remotePath: string) => string,
  localFileExists: (localPath: string) => boolean
): TrackedNodeState[] {
  return trackedNodes.filter((tracked) => !localFileExists(toLocalPath(tracked.path)));
}
