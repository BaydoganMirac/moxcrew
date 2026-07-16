#!/usr/bin/env node
import { randomUUID } from "node:crypto";
import { readFile, rename, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const target = join(root, ".next", "required-server-files.json");
const manifest = JSON.parse(await readFile(target, "utf8"));
const locations = [
  manifest.appDir,
  manifest.config?.outputFileTracingRoot,
  manifest.config?.turbopack?.root,
];
if (locations.some((location) => location !== root)) {
  throw new Error("Next.js build locations did not match the package root");
}
manifest.appDir = ".";
manifest.config.outputFileTracingRoot = ".";
manifest.config.turbopack.root = ".";
const temporary = `${target}.${randomUUID()}.tmp`;
await writeFile(temporary, `${JSON.stringify(manifest)}\n`, { flag: "wx", mode: 0o600 });
await rename(temporary, target);
