#!/usr/bin/env node
import { randomUUID } from "node:crypto";
import { readdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const nextRoot = join(root, ".next");
const target = join(nextRoot, "required-server-files.json");
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

await Promise.all([
  "cache",
  "diagnostics",
  "trace",
  "turbopack",
  "types",
].map((entry) => rm(join(nextRoot, entry), { recursive: true, force: true })));

async function removeSourceMaps(directory) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) await removeSourceMaps(path);
    else if (entry.isFile() && entry.name.endsWith(".map")) await rm(path);
  }
}

await removeSourceMaps(nextRoot);
