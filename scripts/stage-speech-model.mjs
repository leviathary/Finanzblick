// Bereitet das lokale Spracherkennungsmodell für die Paketierung vor und prüft dessen Prüfsumme.

import { createHash } from 'node:crypto';
import { readFile, mkdir, writeFile } from 'node:fs/promises';

// Build-time download only. Dictation never contacts a remote server.
const target = new URL('../public/speech/de.tar.gz', import.meta.url);
const expected = 'dc500ec4e8176b68ab12037ce6b19de6ce8c23f916d90b6be86f7b22633dd430';
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
let existing;
try { existing = await readFile(target); } catch (e) { if (e.code !== 'ENOENT') throw e; }
if (!existing || hash(existing) !== expected) {
  const response = await fetch('https://ccoreilly.github.io/vosk-browser/models/vosk-model-small-de-0.15.tar.gz', { signal: AbortSignal.timeout(120000) });
  if (!response.ok) throw new Error(`Speech model download failed: ${response.status}`);
  const bytes = Buffer.from(await response.arrayBuffer());
  if (hash(bytes) !== expected) throw new Error('Speech model checksum mismatch');
  await mkdir(new URL('../public/speech/', import.meta.url), { recursive: true });
  await writeFile(target, bytes);
}
console.log('Verified local German speech model.');
