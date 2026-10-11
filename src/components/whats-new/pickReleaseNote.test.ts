// Standalone assert check (no JS unit-test runner in this repo). Run with:
//   bun src/components/whats-new/pickReleaseNote.test.ts
import assert from "node:assert";
import { pickReleaseNote } from "./pickReleaseNote";

const stored = {
  version: "0.2.0",
  markdown: "## 0.2.0\n\n- notes from latest.json",
};

// After the update to 0.2.0, the notes kept before installing show once.
assert.deepEqual(pickReleaseNote([stored], "0.2.0", "0.1.0"), stored);
// Already seen: nothing.
assert.equal(pickReleaseNote([stored], "0.2.0", "0.2.0"), null);
// The install did not happen (still 0.1.0): not yet.
assert.equal(pickReleaseNote([stored], "0.1.0", "0.1.0"), null);
// A bundled note of the same version comes first and wins.
const bundled = { version: "0.2.0", markdown: "bundled" };
assert.equal(pickReleaseNote([bundled, stored], "0.2.0", "0.1.0"), bundled);
// The newest eligible note wins over an older one.
const older = { version: "0.1.5", markdown: "older" };
assert.equal(pickReleaseNote([older, stored], "0.2.0", "0.1.0"), stored);

console.log("pickReleaseNote: ok");
