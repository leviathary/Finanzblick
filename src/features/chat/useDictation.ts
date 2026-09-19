// Verbindet Mikrofoneingabe mit lokaler Spracherkennung und verwaltet deren Ressourcen.

import { useEffect, useRef, useState } from 'react';
import type { VoskClient as Model, KaldiRecognizer } from '@lichess-org/vosk-browser';
import workerUrl from '@lichess-org/vosk-browser/dist/vosk.worker.js?url';
import wasmUrl from '@lichess-org/vosk-browser/dist/vosk.wasm?url';

type Session = {
  model?: Model; recognizer?: KaldiRecognizer; stream?: MediaStream;
  context?: AudioContext; source?: MediaStreamAudioSourceNode; processor?: ScriptProcessorNode;
  timer?: ReturnType<typeof setTimeout>; deadline?: ReturnType<typeof setTimeout>;
};

function destroyModel(model?: Model) {
  if (!model) return;
  // Pinned VoskClient 0.0.3 only queues terminate(), which can fail during loading.
  // Terminating its dedicated Worker also reclaims WASM memory and pending audio.
  const worker: unknown = Reflect.get(model, "worker");
  if (worker instanceof Worker) worker.terminate();
  else model.terminate();
}

// Audio stays inside this WebView and its local recognition worker. No audio is stored.
export function useDictation(setText: (text: string) => void, reportError: (text: string) => void) {
  const [phase, setPhase] = useState<'idle' | 'loading' | 'listening' | 'finishing'>('idle');
  const active = useRef<Session | null>(null);

  function release(s: Session) {
    clearTimeout(s.timer); clearTimeout(s.deadline);
    if (s.processor) { s.processor.onaudioprocess = null; s.processor.disconnect(); }
    s.source?.disconnect();
    s.stream?.getTracks().forEach(track => track.stop());
    if (s.context && s.context.state !== 'closed') void s.context.close().catch(() => {});
  }
  function cancel() {
    const s = active.current;
    active.current = null;
    if (s) { release(s); destroyModel(s.model); }
    setPhase('idle');
  }
  useEffect(() => () => {
    const s = active.current; active.current = null;
    if (s) { release(s); destroyModel(s.model); }
  }, []);

  function stop() {
    const s = active.current;
    if (!s) return;
    if (!s.recognizer) { cancel(); return; }
    release(s); setPhase('finishing');
    s.recognizer.retrieveFinalResult();
    s.deadline = setTimeout(cancel, 5000);
  }

  async function start(initial: string) {
    if (active.current) return;
    const s: Session = {};
    active.current = s; setPhase('loading'); reportError('');
    let committed = initial.trimEnd();
    let finishing = false;
    let stage: "microphone" | "recognition" = "microphone";
    const current = () => active.current === s;
    const update = (tail: string) => setText([committed, tail].filter(Boolean).join(' ').slice(0, 4000));
    try {
      if (!navigator.mediaDevices?.getUserMedia) throw new Error('unsupported');
      // Create/resume within the click gesture, including Safari's user activation requirement.
      s.context = new AudioContext();
      await s.context.resume();
      if (!current()) return;
      const stream = await navigator.mediaDevices.getUserMedia({ audio: { channelCount: 1, echoCancellation: true, noiseSuppression: true }, video: false });
      if (!current()) { stream.getTracks().forEach(track => track.stop()); return; }
      s.stream = stream;
      stage = "recognition";
      const { VoskClient } = await import('@lichess-org/vosk-browser');
      if (!current()) return;
      s.model = new VoskClient({ modelUrl: new URL('/speech/de.tar.gz', window.location.href).href, workerUrl, wasmUrl: new URL(wasmUrl, window.location.href).href, logLevel: -1 });
      s.deadline = setTimeout(() => { if (current()) { reportError('Das Sprachmodell konnte nicht geladen werden. Bitte erneut versuchen.'); cancel(); } }, 45000);
      s.model.on('error', () => { if (current()) { reportError('Die Spracheingabe wurde unterbrochen. Bitte erneut versuchen.'); cancel(); } });
      s.model.on('load', message => {
        if (!current()) return;
        clearTimeout(s.deadline);
        if (message.event !== 'load' || !message.result || !s.context || !s.model) { cancel(); return; }
        const recognizer = new s.model.KaldiRecognizer(s.context.sampleRate);
        s.recognizer = recognizer;
        recognizer.on('result', message => {
          if (!current() || message.event !== 'result') return;
          committed = [committed, message.result.text].filter(Boolean).join(' ').slice(0, 4000);
          update('');
          if (finishing) cancel();
        });
        recognizer.on('partialresult', message => { if (current() && message.event === 'partialresult') update(message.result.partial); });
        recognizer.on('error', () => { if (current()) { reportError('Die Spracheingabe wurde unterbrochen. Bitte erneut versuchen.'); cancel(); } });
        s.source = s.context.createMediaStreamSource(stream);
        s.processor = s.context.createScriptProcessor(4096, 1, 1);
        s.processor.onaudioprocess = event => { if (current()) recognizer.acceptWaveform(event.inputBuffer); };
        // The processor emits silence, so no microphone feedback reaches the speakers.
        s.source.connect(s.processor); s.processor.connect(s.context.destination);
        // Mark finalization before stopping capture, including the automatic time limit.
        const finish = () => { finishing = true; stop(); };
        stream.getAudioTracks().forEach(track => { track.onended = () => { if (current()) finish(); }; });
        s.timer = setTimeout(finish, 120000);
        // The public stop action also needs to mark the final result.
        finalizer.current = finish;
        setPhase('listening');
      });
    } catch (error) {
      if (!current()) return;
      const denied = error instanceof DOMException && (error.name === 'NotAllowedError' || error.name === 'PermissionDeniedError');
      reportError(stage === "recognition"
        ? 'Die Spracherkennung konnte nicht geladen werden. Bitte die App neu öffnen und erneut versuchen.'
        : denied ? 'Bitte den Mikrofonzugriff in den Geräteinstellungen erlauben.' : 'Das Mikrofon ist nicht verfügbar. Bitte Anschluss und Berechtigung prüfen.');
      cancel();
    }
  }
  const finalizer = useRef<(() => void) | null>(null);
  return { phase, active: phase !== 'idle', start, cancel, stop: () => { if (phase === 'listening') finalizer.current?.(); else cancel(); } };
}
