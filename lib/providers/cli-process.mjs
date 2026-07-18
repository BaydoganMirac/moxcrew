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
  const { timeout, ...execOptions } = options;
  let timedOut = false;
  let timer;
  const child = exec(windowsCommandLine(command, args), execOptions, (error, stdout, stderr) => {
    clearTimeout(timer);
    if (error && timedOut) {
      error.killed = true;
      error.signal = "SIGTERM";
    }
    if (
      error &&
      !timedOut &&
      error.code === 1 &&
      /(?:is not recognized as an internal or external command|The system cannot find the path specified)/i.test(stderr)
    ) {
      error.code = "ENOENT";
    }
    callback(error, stdout, stderr);
  });
  if (Number.isFinite(timeout) && timeout > 0) {
    timer = setTimeout(() => {
      timedOut = true;
      terminateCli(child);
    }, timeout);
    timer.unref();
  }
  return child;
}

export function spawnCli(command, args, options) {
  if (process.platform !== "win32") return spawn(command, args, options);
  return spawn(windowsCommandLine(command, args), { ...options, shell: true });
}

export function terminateCli(child, signal = "SIGTERM") {
  if (process.platform !== "win32" || !Number.isSafeInteger(child.pid)) {
    child.kill(signal);
    return;
  }
  const killer = spawn("taskkill.exe", ["/pid", String(child.pid), "/T", "/F"], {
    stdio: "ignore",
    windowsHide: true,
  });
  killer.on("error", () => undefined);
  killer.unref();
}
