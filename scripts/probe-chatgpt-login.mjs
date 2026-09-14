// Isolated protocol probe: no existing login, financial data, or model request.
// Usage: node scripts/probe-chatgpt-login.mjs [path-to-codex-executable]
import { spawn } from "node:child_process";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import readline from "node:readline";
import assert from "node:assert/strict";

const temporaryRoot = await fs.realpath(os.tmpdir());
const probeHome = await fs.mkdtemp(path.join(temporaryRoot, "finanzblick-codex-probe-"));
const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => !/^(CODEX_|OPENAI_|ANTHROPIC_|GEMINI_|GOOGLE_API_KEY)/i.test(key)));
env.CODEX_HOME = probeHome;
let child;
let closed;
const waiting = new Map();
let sequence = 0;
try {
  await fs.writeFile(path.join(probeHome, "config.toml"), 'cli_auth_credentials_store = "file"\n[analytics]\nenabled = false\n');
  child = spawn(process.argv[2] ? path.resolve(process.argv[2]) : "codex", ["app-server", "--listen", "stdio://"], {
    cwd: probeHome, env, stdio: ["pipe", "pipe", "pipe"], windowsHide: true,
  });
  closed = new Promise(resolve => child.once("close", resolve));
  const failAll = error => { for (const item of waiting.values()) item.reject(error); waiting.clear(); };
  child.on("error", () => failAll(new Error("Codex executable could not be started.")));
  child.on("exit", () => failAll(new Error("Codex exited before completing the protocol probe.")));
  child.stderr.resume(); // Do not persist diagnostics or authentication material.
  const lines = readline.createInterface({ input: child.stdout });
  lines.on("line", line => {
    let message;
    try { message = JSON.parse(line); } catch { return; }
    const pending = waiting.get(message.id);
    if (pending) {
      waiting.delete(message.id);
      if (message.error) pending.reject(new Error(`Protocol rejected ${pending.method} (code ${Number(message.error.code)}).`));
      else pending.resolve(message.result);
    } else if (message.id != null && message.method) {
      // A probe never approves tools, file access, or any other server-initiated action.
      child.stdin.write(JSON.stringify({ id: message.id, error: { code: -32601, message: "Not supported by isolated login probe" } }) + "\n");
    }
  });
  const rpc = (method, params) => new Promise((resolve, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => { waiting.delete(id); reject(new Error(`Timeout during ${method}.`)); }, 15000);
    waiting.set(id, { method, resolve: value => { clearTimeout(timer); resolve(value); }, reject: error => { clearTimeout(timer); reject(error); } });
    child.stdin.write(JSON.stringify({ id, method, params }) + "\n");
  });
  const initialized = await rpc("initialize", { clientInfo: { name: "finanzblick_login_probe", title: "Finanzblick login probe", version: "0.1.0" }, capabilities: { experimentalApi: false } });
  assert.ok(initialized);
  child.stdin.write(JSON.stringify({ method: "initialized" }) + "\n");
  const account = await rpc("account/read", { refreshToken: false });
  assert.equal(account.account, null, "The isolated probe must not reuse a personal account.");
  const login = await rpc("account/login/start", { type: "chatgpt" });
  assert.equal(login.type, "chatgpt");
  assert.ok(login.loginId);
  const url = new URL(login.authUrl);
  assert.equal(url.protocol, "https:");
  assert.ok(["auth.openai.com", "chatgpt.com"].includes(url.hostname));
  const cancelled = await rpc("account/login/cancel", { loginId: login.loginId });
  assert.ok(cancelled);
  const after = await rpc("account/read", { refreshToken: false });
  assert.equal(after.account, null);
  console.log(JSON.stringify({
    protocol: "stdio", initialization: "passed", isolatedAccount: "signed out",
    chatgptLoginStart: "passed", authorizationHost: url.hostname, loginCancel: "passed",
    browserOpened: false, modelRequests: 0, financialDataSent: false,
    limitation: "Full user sign-in, subscription inference and application sandboxing remain untested.",
  }, null, 2));
} finally {
  if (child) {
    child.stdin.destroy();
    child.kill();
    await closed;
  }
  // Delete only the exact temporary directory created above, never a computed user home.
  const resolved = await fs.realpath(probeHome);
  if (path.dirname(resolved) !== temporaryRoot || !path.basename(resolved).startsWith("finanzblick-codex-probe-")) {
    throw new Error("Unexpected probe directory; refusing cleanup.");
  }
  await fs.rm(resolved, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
}
