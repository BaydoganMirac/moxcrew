import { decodeGeminiHook } from "../../lib/providers/gemini.mjs";
import { writeGeminiInboxSignal } from "../../lib/runtime/gemini-inbox.mjs";
import { realpath } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const MAX_BYTES = 16_384;

async function readInput() {
  let input = "";
  for await (const chunk of process.stdin) {
    input += chunk;
    if (Buffer.byteLength(input) > MAX_BYTES) throw new TypeError();
  }
  return input;
}

try {
  const signal = decodeGeminiHook(await readInput());
  if (signal) {
    const sourceRoot = resolve(dirname(await realpath(fileURLToPath(import.meta.url))), "../..");
    await writeGeminiInboxSignal(signal, {
      dataDir: process.env.PATCHFLEET_DATA_DIR ?? join(homedir(), ".patchfleet"),
      sourceRoot,
    });
  }
} catch {
  // Observation is fail-open and never exposes the native hook payload.
}
process.stdout.write("{}");
