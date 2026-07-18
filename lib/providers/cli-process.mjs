import { exec, execFile, spawn } from "node:child_process";

function windowsCommandLine(command, args) {
  const quote = (value) => {
    const text = String(value);
    if (/[\0\r\n"]/.test(text)) throw new TypeError("invalid CLI command");
    return `"${text.replaceAll("%", "%%")}"`;
  };
  return [command, ...args].map(quote).join(" ");
}

export function execCli(command, args, options, callback) {
  if (process.platform !== "win32") return execFile(command, args, options, callback);
  return exec(windowsCommandLine(command, args), options, (error, stdout, stderr) => {
    if (
      error &&
      error.code === 1 &&
      /(?:is not recognized as an internal or external command|The system cannot find the path specified)/i.test(stderr)
    ) {
      error.code = "ENOENT";
    }
    callback(error, stdout, stderr);
  });
}

export function spawnCli(command, args, options) {
  if (process.platform !== "win32") return spawn(command, args, options);
  return spawn(windowsCommandLine(command, args), { ...options, shell: true });
}
