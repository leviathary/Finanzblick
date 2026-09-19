// Bereitet die plattformspezifische Chat-Laufzeit für die Paketierung vor.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createHash } from "node:crypto";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const triples = { "win32-x64": "x86_64-pc-windows-msvc", "win32-arm64": "aarch64-pc-windows-msvc", "darwin-x64": "x86_64-apple-darwin", "darwin-arm64": "aarch64-apple-darwin", "linux-x64": "x86_64-unknown-linux-musl", "linux-arm64": "aarch64-unknown-linux-musl" };
const platform = `${process.platform}-${process.arch}`;
const triple = triples[platform];
if (!triple) throw new Error(`Unsupported chat runtime platform: ${platform}`);
const pkg = JSON.parse(fs.readFileSync(path.join(root, "node_modules/@openai/codex/package.json"), "utf8"));
if (pkg.version !== "0.154.0") throw new Error("Re-audit the chat runtime before changing its version.");
const name = process.platform === "win32" ? "codex.exe" : "codex";
const source = path.join(root, `node_modules/@openai/codex-${platform}/vendor`, triple, "bin", name);
const target = path.join(root, "src-tauri/chat-runtime/bin");
fs.mkdirSync(target, { recursive: true });
const sha256 = createHash("sha256").update(fs.readFileSync(source)).digest("hex");
const staged = path.join(target, name);
const stagedHash = fs.existsSync(staged) ? createHash("sha256").update(fs.readFileSync(staged)).digest("hex") : null;
// Windows locks a running executable. An unchanged runtime need not be replaced.
if (stagedHash !== sha256) fs.copyFileSync(source, staged);
if (process.platform !== "win32") fs.chmodSync(path.join(target, name), 0o755);
fs.writeFileSync(path.join(target, "manifest.json"), JSON.stringify({ version: pkg.version, platform, executable: name, sha256 }, null, 2) + "\n");
console.log(`Chat runtime ${pkg.version} staged for ${platform}.`);
