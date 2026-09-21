// Sammelt Lizenzinformationen der Abhängigkeiten für die Auslieferung.

import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const target = process.argv[2] ?? execFileSync("rustc", ["-vV"], { encoding: "utf8" }).match(/^host: (.+)$/m)?.[1];
if (!target) throw new Error("Rust target could not be determined.");
const metadata = JSON.parse(execFileSync("cargo", ["metadata", "--manifest-path", "src-tauri/Cargo.toml", "--locked", "--offline", "--format-version", "1", "--filter-platform", target], { cwd: root, encoding: "utf8", maxBuffer: 32 * 1024 * 1024 }));
const reachable = new Set(metadata.resolve.nodes.map(node => node.id));
const packages = metadata.packages.filter(item => item.source && reachable.has(item.id)).map(item => ({
  ecosystem: "Cargo", name: item.name, version: item.version, license: item.license,
  directory: path.dirname(item.manifest_path), licenseFile: item.license_file,
  source: `https://crates.io/api/v1/crates/${item.name}/${item.version}/download`,
  authors: item.authors,
}));
const supplements = JSON.parse(await fs.readFile(path.join(root, "licenses/upstream/sources.json"), "utf8"));
const lock = JSON.parse(await fs.readFile(path.join(root, "package-lock.json"), "utf8"));
for (const [location, item] of Object.entries(lock.packages)) {
  if (!location || item.dev || item.optional) continue;
  const directory = path.join(root, location);
  const manifest = JSON.parse(await fs.readFile(path.join(directory, "package.json"), "utf8"));
  packages.push({ ecosystem: "npm", name: manifest.name, version: manifest.version, license: manifest.license, directory, source: item.resolved });
}

// This development dependency is deliberately shipped as a native runtime.
packages.push({ ecosystem: "Bundled runtime", name: "OpenAI Codex", version: "0.154.0", license: "Apache-2.0", directory: path.join(root, "src-tauri/chat-runtime"), source: "https://github.com/openai/codex/releases/tag/rust-v0.154.0" });

packages.push({ ecosystem: "Bundled speech model", name: "Vosk German small", version: "0.15", license: "Apache-2.0", directory: path.join(root, "public/speech"), source: "https://alphacephei.com/vosk/models" });

async function licenseFiles(directory, relative = "") {
  const found = [];
  for (const entry of await fs.readdir(path.join(directory, relative), { withFileTypes: true })) {
    const name = path.join(relative, entry.name);
    if (entry.isDirectory() && ![".git", "node_modules", "target"].includes(entry.name)) {
      found.push(...await licenseFiles(directory, name));
    } else if (entry.isFile() && /^(licen[cs]e|copying|copyright|notice)([._-]|$)/i.test(entry.name)) {
      found.push(name);
    }
  }
  return found;
}

const sections = [];
const index = [];
const missing = [];
for (const item of packages.sort((first, second) => `${first.ecosystem}/${first.name}/${first.version}`.localeCompare(`${second.ecosystem}/${second.name}/${second.version}`))) {
  const files = await licenseFiles(item.directory);
  if (item.licenseFile && !files.includes(item.licenseFile)) files.push(item.licenseFile);
  const texts = [];
  for (const filename of [...new Set(files)].sort()) {
    const content = await fs.readFile(path.resolve(item.directory, filename), "utf8");
    texts.push(`--- ${filename.split(path.sep).join("/")} ---\n${content.trim()}\n`);
  }
  for (const supplement of supplements[`${item.name}@${item.version}`] ?? []) {
    texts.push(`--- Upstream supplement: ${supplement.source} ---\n${(await fs.readFile(path.join(root, supplement.file), "utf8")).trim()}\n`);
  }
  if (!texts.length && ["adobe-cmap-parser@0.4.1", "pdf-extract@0.12.0", "type1-encoding-parser@0.1.1"].includes(`${item.name}@${item.version}`) && item.license === "MIT") {
    const standard = (await fs.readFile(path.join(root, "LICENSE"), "utf8")).split("Permission is hereby granted")[1];
    texts.push(`Upstream declares MIT in Cargo.toml but supplies no standalone license file.\nOriginal author attribution: ${item.authors.join(", ")}\nStandard MIT permission and disclaimer follow; no copyright year is inferred.\n\nPermission is hereby granted${standard}`);
  }
  if (!texts.length && item.name === "selectors" && item.license === "MPL-2.0") {
    const cssparser = packages.find(candidate => candidate.name === "cssparser");
    const standard = await fs.readFile(path.join(cssparser.directory, "LICENSE"), "utf8");
    const sourceHeader = (await fs.readFile(path.join(item.directory, "lib.rs"), "utf8")).split("*/")[0] + "*/";
    texts.push(`Upstream source license header:\n${sourceHeader}\n\nStandard MPL-2.0 text (also distributed by cssparser):\n${standard}`);
  }
  if (!texts.length || !item.license) missing.push(`${item.ecosystem}: ${item.name} ${item.version}`);
  index.push(`| ${item.ecosystem} | ${item.name} | ${item.version} | ${item.license ?? "UNKNOWN"} | [Source](${item.source}) |`);
  sections.push(`${"=".repeat(80)}\n${item.ecosystem}: ${item.name} ${item.version}\nDeclared license: ${item.license}\nSource: ${item.source}\n\n${texts.join("\n")}`);
}
if (missing.length) throw new Error(`Missing license declarations or files:\n${missing.join("\n")}`);
const title = "Third-party licenses for Saldonaut";
await fs.writeFile(path.join(root, "THIRD_PARTY_LICENSES.txt"), `${title}\nTarget: ${target}\n\nOriginal license and notice files follow. Each dependency retains its own terms.\nThe inventory includes build dependencies and nested native-library notices.\nSource links identify the exact upstream packages, including MPL-covered sources.\nSaldonaut's MIT license does not replace these licenses.\n\n${sections.join("\n").replaceAll("\r\n", "\n").replace(/[\t ]+$/gm, "").trimEnd()}\n`);
await fs.writeFile(path.join(root, "DEPENDENCIES.md"), `# Third-party dependencies\n\nTarget: \`${target}\`. Generated with \`npm run licenses\` from the locked Cargo dependencies\nand installed production npm packages. Includes Cargo build dependencies; this is\na conservative inventory, not a claim that every listed package is linked at runtime.\n\nFull original license and copyright notices, including bundled native sources,\nare in [THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt). Upstream source links\nprovide the exact versions; third-party code is not relicensed under Saldonaut's MIT license.\nRegenerate for each release target after dependency changes.\n\n| Ecosystem | Package | Version | Declared license | Source archive |\n| --- | --- | --- | --- | --- |\n${index.join("\n")}\n`);
console.log(`Collected licenses for ${packages.length} packages (${target}).`);
