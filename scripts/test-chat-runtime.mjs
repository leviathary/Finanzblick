// Actual pinned runtime, local mock model: no account or financial data leaves the device.
import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import http from "node:http";
import readline from "node:readline";
import { spawn } from "node:child_process";
import assert from "node:assert/strict";

const root = path.resolve(import.meta.dirname, "..");
const home = await fs.mkdtemp(path.join(os.tmpdir(), "finanzblick-runtime-test-"));
const requests = [];
let pending = new Map(), next = 0, proc;
let complete;
const server = http.createServer(async (req, res) => {
  let body = ""; for await (const chunk of req) body += chunk;
  const parsed = JSON.parse(body); requests.push(parsed);
  res.writeHead(200, { "Content-Type": "text/event-stream" });
  const event = (type, data) => res.write(`event: ${type}\ndata: ${JSON.stringify({ type, ...data })}\n\n`);
  event("response.created", { response: { id: "resp_test" } });
  if (requests.length === 1) {
    const attacks = [
      { type: "function_call", id: "tool_shell", call_id: "call_shell", name: "exec_command", arguments: JSON.stringify({ cmd: "echo FINANZBLICK_TOOL_EXECUTED" }) },
      { type: "custom_tool_call", id: "tool_patch", call_id: "call_patch", name: "apply_patch", input: "*** Begin Patch\n*** Add File: must-not-exist.txt\n+FINANZBLICK_TOOL_EXECUTED\n*** End Patch" },
      { type: "function_call", id: "tool_image", call_id: "call_image", name: "view_image", arguments: JSON.stringify({ path: path.join(home, "private-test.png") }) },
    ];
    attacks.forEach((item, output_index) => { event("response.output_item.added", { output_index, item }); event("response.output_item.done", { output_index, item }); });
    event("response.completed", { response: { id: "resp_test", status: "completed", output: attacks, usage: { input_tokens: 10, output_tokens: 3, total_tokens: 13 } } });
    res.end(); return;
  }
  const item = { type: "message", id: "msg_test", role: "assistant", status: "completed", content: [{ type: "output_text", text: "Synthetic answer." }] };
  event("response.output_item.added", { output_index: 0, item });
  event("response.output_text.delta", { item_id: "msg_test", output_index: 0, content_index: 0, delta: "Synthetic answer." });
  event("response.output_item.done", { output_index: 0, item });
  event("response.completed", { response: { id: "resp_test", status: "completed", output: [item], usage: { input_tokens: 10, output_tokens: 3, total_tokens: 13 } } });
  res.end();
});
await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
try {
  const modelPath = path.join(home, "models.json");
  await fs.copyFile(path.join(root, "src-tauri/chat-runtime/models.json"), modelPath);
  let config = await fs.readFile(path.join(root, "src-tauri/chat-runtime/config.toml"), "utf8");
  config = `model_catalog_json = ${JSON.stringify(modelPath)}\n` + config.replace('model_provider = "openai"', 'model_provider = "local_test"');
  config += `\n[model_providers.local_test]\nname = "Local test"\nbase_url = "http://127.0.0.1:${server.address().port}/v1"\nwire_api = "responses"\nrequires_openai_auth = false\n`;
  await fs.writeFile(path.join(home, "config.toml"), config);
  const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => !/^(CODEX_|OPENAI_|ANTHROPIC_|GEMINI_|GOOGLE_API_KEY)/i.test(key)));
  env.CODEX_HOME = home;
  env.HOME = home; env.USERPROFILE = home;
  proc = spawn(path.join(root, "src-tauri/chat-runtime/bin", process.platform === "win32" ? "codex.exe" : "codex"), ["app-server", "--listen", "stdio://"], { cwd: home, env, stdio: ["pipe", "pipe", "pipe"], windowsHide: true });
  proc.stderr.on("data", data => process.stderr.write(data)); // Synthetic test only.
  const notifications = [];
  readline.createInterface({ input: proc.stdout }).on("line", line => {
    const message = JSON.parse(line);
    if (pending.has(message.id)) { const cb = pending.get(message.id); pending.delete(message.id); message.error ? cb.reject(new Error(JSON.stringify(message.error))) : cb.resolve(message.result); }
    else { notifications.push(message); if (message.method === "turn/completed") complete?.(message.params); }
  });
  const rpc = (method, params) => new Promise((resolve, reject) => {
    const id = ++next; const timer = setTimeout(() => reject(new Error(`Timeout: ${method}`)), 25000);
    pending.set(id, { resolve: value => { clearTimeout(timer); resolve(value); }, reject: error => { clearTimeout(timer); reject(error); } });
    proc.stdin.write(JSON.stringify({ id, method, params }) + "\n");
  });
  await rpc("initialize", { clientInfo: { name: "finanzblick_test", version: "1" }, capabilities: { experimentalApi: false } });
  proc.stdin.write('{"method":"initialized"}\n');
  const effective = await rpc("config/read", { includeLayers: false });
  for (const line of config.split("[features]")[1].split("\n[model_providers")[0].split("\n")) {
    const match = line.trim().match(/^(\w+) = (true|false)$/);
    if (match) assert.equal(effective.config.features[match[1]], match[2] === "true");
  }
  assert.deepEqual(effective.config.mcp_servers, {});
  const started = await rpc("thread/start", { ephemeral: true, sandbox: "read-only", approvalPolicy: "never", baseInstructions: "Answer the synthetic question. No tools.", cwd: home });
  const finished = new Promise((resolve, reject) => { complete = resolve; const timer = setTimeout(() => reject(new Error("Turn timeout")), 25000); timer.unref(); });
  await rpc("turn/start", { threadId: started.thread.id, input: [{ type: "text", text: "Return a synthetic answer.", text_elements: [] }] });
  const result = await finished;
  assert.equal(result.turn.status, "completed", JSON.stringify(result));
  assert.ok(requests.length > 0);
  for (const request of requests) assert.deepEqual(request.tools ?? [], [], "Financial chat must expose ZERO model tools.");
  assert.equal(started.thread.ephemeral, true);
  const outputs = requests.flatMap(request => request.input ?? []).filter(item => item.type?.endsWith("_call_output"));
  assert.equal(outputs.length, 3, "All forced tool calls must receive rejections.");
  for (const output of outputs) assert.match(JSON.stringify(output.output), /unsupported|not found|unknown|unrecognized|not available/i);
  assert.equal(await fs.stat(path.join(home, "must-not-exist.txt")).then(() => true, () => false), false);
  assert.equal(notifications.filter(x => x.id != null && x.method).length, 0);
  assert.ok(notifications.some(x => x.method === "item/agentMessage/delta"));
  assert.equal(await fs.stat(path.join(home, "auth.json")).then(() => true, () => false), false);
  const secondFinished = new Promise((resolve, reject) => { complete = resolve; const timer = setTimeout(() => reject(new Error("Follow-up timeout")), 25000); timer.unref(); });
  const followUp = JSON.stringify({ question: "Explain the previous answer." });
  await rpc("turn/start", { threadId: started.thread.id, input: [{ type: "text", text: followUp, text_elements: [] }] });
  assert.equal((await secondFinished).turn.status, "completed");
  const followUpInput = requests.at(-1).input;
  const userMessages = followUpInput.filter(item => item.role === "user");
  assert.equal(userMessages.length, 2, "The same thread must retain the first question.");
  assert.ok(JSON.stringify(userMessages.at(-1)).includes("Explain the previous answer."));
  assert.equal(followUpInput.filter(item => item.role === "assistant").length > 0, true, "Prior answer remains available.");
  for (const request of requests) assert.deepEqual(request.tools ?? [], []);
  await rpc("thread/unsubscribe", { threadId: started.thread.id });
  let loaded;
  for (let attempt = 0; attempt < 30; attempt++) {
    loaded = await rpc("thread/loaded/list", {});
    if (!loaded.data.includes(started.thread.id)) break;
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  assert.ok(!loaded.data.includes(started.thread.id), "Completed ephemeral thread must be unloaded immediately.");
  console.log("PASS: actual runtime, empty tool set, rejected shell/file/image attacks, ephemeral thread, streamed synthetic answer, same-thread follow-up, no stored auth.");
} finally {
  if (proc) { proc.kill(); await new Promise(resolve => proc.once("close", resolve)); }
  server.closeAllConnections(); await new Promise(resolve => server.close(resolve));
  await fs.rm(home, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
}
