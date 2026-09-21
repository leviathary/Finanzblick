// Steuert die Build- und Paketierungsschritte für eine Release-Ausgabe.

import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
if (process.env.RUSTFLAGS && !process.env.CARGO_ENCODED_RUSTFLAGS) {
  throw new Error("Use CARGO_ENCODED_RUSTFLAGS instead of RUSTFLAGS for release builds.");
}
const perlCheck = spawnSync("perl", ["-MIPC::Cmd", "-MLocale::Maketext::Simple", "-e", "1"], {
  encoding: "utf8",
});
if (perlCheck.error || perlCheck.status !== 0) {
  const detail = perlCheck.error?.message ?? perlCheck.stderr.trim();
  const recommendation = process.platform === "win32"
    ? "Install or activate full Strawberry Perl and put perl\\bin and c\\bin before Git for Windows on PATH."
    : "Install a complete native Perl distribution and put it on PATH.";
  throw new Error(`Release builds require Perl with IPC::Cmd and Locale::Maketext::Simple. ${recommendation}\n${detail}`);
}
// Vendored native libraries can embed their build directory independently of
// rustc's path remapping. Keep the default release target outside personal paths
// on every supported platform so those strings remain distributable.
const targetDirectory = path.resolve(process.env.CARGO_TARGET_DIR ?? (
  process.platform === "win32"
    ? path.join(path.parse(root).root, "build", "saldonaut-release")
    : path.join(os.tmpdir(), "saldonaut-release")
));
const flags = (process.env.CARGO_ENCODED_RUSTFLAGS ?? "").split("\x1f").filter(Boolean);
const prefixes = [
  [os.homedir(), "/build/user"],
  [process.env.CARGO_HOME ?? path.join(os.homedir(), ".cargo"), "/build/cargo"],
  [execFileSync("rustc", ["--print", "sysroot"], { encoding: "utf8" }).trim(), "/build/rust"],
  [root, "/build/saldonaut"],
];
for (const [source, destination] of prefixes) {
  for (const variant of new Set([source, source.replaceAll("\\", "/")])) {
    flags.push(`--remap-path-prefix=${variant}=${destination}`);
  }
}
const env = {
  ...process.env,
  CARGO_ENCODED_RUSTFLAGS: flags.join("\x1f"),
  CARGO_TARGET_DIR: targetDirectory,
};
delete env.RUSTFLAGS;
const run = (command, args) => {
  const result = spawnSync(command, args, { cwd: root, env, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
};
run(process.execPath, [path.join(root, "scripts/collect-licenses.mjs")]);
run(process.execPath, [path.join(root, "node_modules/@tauri-apps/cli/tauri.js"), "build", ...process.argv.slice(2)]);
const binary = path.join(targetDirectory, "release", process.platform === "win32" ? "saldonaut.exe" : "saldonaut");
const content = fs.readFileSync(binary);
for (const [prefix] of prefixes) {
  for (const variant of new Set([prefix, prefix.replaceAll("\\", "/")])) {
    for (const encoding of ["utf8", "utf16le"]) {
      if (content.includes(Buffer.from(variant, encoding))) {
        throw new Error("Release contains a personal build path. Do not distribute the generated bundle.");
      }
    }
  }
}
console.log("Release executable checked: no configured personal build prefixes found.");
