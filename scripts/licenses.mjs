import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

// setup-python and standard Windows installs provide python.exe; Unix uses python3.
const python = process.platform === "win32" ? "python" : "python3";
const script = fileURLToPath(new URL("./licenses.py", import.meta.url));
const result = spawnSync(python, [script, ...process.argv.slice(2)], { stdio: "inherit" });
if (result.error) {
  console.error("License verification requires Python 3.11 or newer:", result.error.message);
}
process.exit(result.status ?? 1);
