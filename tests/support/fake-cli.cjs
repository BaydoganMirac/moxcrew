const { chmod, writeFile } = require("node:fs/promises");

async function writeFakeCli(basePath, source) {
  if (process.platform === "win32") {
    const script = `${basePath}.cjs`;
    const command = `${basePath}.cmd`;
    const escape = (value) => value.replaceAll("%", "%%");
    await writeFile(script, source, "utf8");
    await writeFile(
      command,
      `@echo off\r\n"${escape(process.execPath)}" "${escape(script)}" %*\r\n`,
      "utf8",
    );
    return command;
  }
  await writeFile(basePath, source, "utf8");
  await chmod(basePath, 0o700);
  return basePath;
}

module.exports = { writeFakeCli };
