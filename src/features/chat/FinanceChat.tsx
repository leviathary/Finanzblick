// Stellt den Finanzchat mit Anmeldung, Datenvorschau und optionaler Spracheingabe bereit.

import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { invoke } from "@tauri-apps/api/core";
import { useDictation } from "./useDictation";
import { t } from "../../i18n";
import { SourcePreview, type Preview } from "./SourcePreview";

type Account = { connected: boolean; loginPending: boolean; model: string };
type Message = { role: "user" | "assistant"; content: string };
type Draft = Preview & { previewId: number; question: string };
const emptyAccount: Account = { connected: false, loginPending: false, model: "gpt-5.6-sol" };
const today = () => { const d = new Date(); return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`; };

export function FinanceChat() {
  const [account, setAccount] = useState<Account>(emptyAccount);
  const [settings, setSettings] = useState(false);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState<"login" | "preview" | "send" | null>(null);
  const [error, setError] = useState("");
  const [from, setFrom] = useState(`${new Date().getFullYear()}-01-01`);
  const [to, setTo] = useState(today);
  const [accountScope, setAccountScope] = useState<"auto" | "all" | "credit_cards">("auto");
  const [fearless, setFearless] = useState(false);
  const [reviewBeforeSend, setReviewBeforeSend] = useState(true);
  const [question, setQuestion] = useState("");
  const [history, setHistory] = useState<Message[]>([]);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [pendingQuestion, setPendingQuestion] = useState<string | null>(null);
  const dictation = useDictation(setQuestion, setError);
  useEffect(() => { if (settings || draft || busy || !account.connected) dictation.cancel(); }, [settings, draft, busy, account.connected]);
  const input = useRef<HTMLTextAreaElement>(null);
  const stream = useRef<HTMLDivElement>(null);
  const followLatest = useRef(true);
  const revision = useRef(0);

  useLayoutEffect(() => {
    const field = input.current;
    if (field) { field.style.height = "0px"; field.style.height = `${Math.min(field.scrollHeight, 160)}px`; }
  }, [question]);

  useLayoutEffect(() => {
    if (followLatest.current && stream.current) stream.current.scrollTop = stream.current.scrollHeight;
  }, [history, pendingQuestion, busy]);

  useEffect(() => {
    if (!busy && !loading && account.connected && !settings && !draft) input.current?.focus();
  }, [busy, loading, account.connected, settings, draft]);
  const mounted = useRef(false);

  useEffect(() => {
    mounted.current = true;
    void invoke<Account>("chatgpt_status").then(value => { if (mounted.current) setAccount(value); }).catch(e => { if (mounted.current) setError(String(e)); }).finally(() => { if (mounted.current) setLoading(false); });
    return () => { mounted.current = false; revision.current++; void invoke("cancel_finance_chat").catch(() => {}); };
  }, []);

  useEffect(() => {
    if (!account.loginPending || busy) return;
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try { const value = await invoke<Account>("chatgpt_status"); if (active) { setAccount(value); if (value.connected) setSettings(false); } }
      catch (e) { if (active) setError(String(e)); }
      if (active) timer = setTimeout(() => { void poll(); }, 2000);
    };
    timer = setTimeout(() => { void poll(); }, 1000);
    return () => { active = false; clearTimeout(timer); };
  }, [account.loginPending, busy]);

  const valid = (id: number) => mounted.current && revision.current === id;
  async function login() {
    const id = ++revision.current; setBusy("login"); setError("");
    try { await invoke("chatgpt_login_start"); const value = await invoke<Account>("chatgpt_status"); if (valid(id)) { setAccount(value); if (value.connected) setSettings(false); } }
    catch (e) { if (valid(id)) setError(String(e)); }
    finally { if (valid(id)) setBusy(null); }
  }
  async function disconnect() {
    ++revision.current; setBusy(null); setDraft(null); setHistory([]); setQuestion(""); setConfirmed(false); setPendingQuestion(null); setFearless(false); setReviewBeforeSend(true);
    await invoke("chatgpt_disconnect"); if (mounted.current) setAccount(emptyAccount);
  }
  function newChat() {
    dictation.cancel();
    ++revision.current; setHistory([]); setDraft(null); setConfirmed(false); setError(""); setQuestion(""); setBusy(null); setPendingQuestion(null);
    followLatest.current = true; input.current?.focus();
    void invoke("cancel_finance_chat").catch(e => setError(String(e)));
  }
  async function prepare() {
    if (!question.trim() || !account.connected || busy || draft || dictation.active || history.length >= 12) return;
    const id = ++revision.current; setBusy("preview"); setError(""); setDraft(null); setConfirmed(false);
    try {
      const value = await invoke<Preview & { previewId: number }>("prepare_finance_chat", { request: { from, to, question, history, includeDetails: fearless, accountScope } });
      if (valid(id)) {
        const scope = JSON.parse(value.payload).data.accountScope;
        if (scope === "credit_cards" && accountScope !== "credit_cards") {
          setHistory([]); setAccountScope("credit_cards");
        }
        const prepared = { ...value, question: question.trim() };
        if (!value.followUp && history.length) setHistory([]);
        if (reviewBeforeSend && !value.followUp) setDraft(prepared);
        else await deliver(prepared);
      }
    } catch (e) { if (valid(id)) setError(String(e)); }
    finally { if (valid(id)) setBusy(null); }
  }
  async function send() {
    if (!draft || !confirmed || busy) return;
    await deliver(draft);
  }
  async function deliver(approved: Draft) {
    followLatest.current = true;
    setPendingQuestion(approved.question); setQuestion("");
    const id = ++revision.current; setBusy("send"); setError(""); setDraft(null); setConfirmed(false);
    try {
      const answer = await invoke<string>("send_finance_chat", { previewId: approved.previewId });
      if (valid(id)) { setHistory(old => [...old, { role: "user", content: approved.question }, { role: "assistant", content: answer }]); setPendingQuestion(null); }
    } catch (e) {
      if (valid(id)) { setPendingQuestion(null); setQuestion(approved.question); setError(String(e)); const status = await invoke<Account>("chatgpt_status").catch(() => emptyAccount); if (valid(id)) setAccount(status); }
    } finally { if (valid(id)) setBusy(null); }
  }
  async function cancel() {
    ++revision.current; await invoke("cancel_finance_chat");
    if (mounted.current) { setPendingQuestion(null); setBusy(null); setDraft(null); setConfirmed(false); setAccount(await invoke<Account>("chatgpt_status").catch(() => emptyAccount)); }
  }

  async function editDraft() {
    ++revision.current;
    setDraft(null); setConfirmed(false);
    await invoke("cancel_finance_chat");
    input.current?.focus();
  }

  return <section className="finance-chat" aria-label={t("Finanzchat")}>
    <header className="chat-header">
      <div><h1>{t("Finanzchat")}</h1><button className="chat-mode-button" onClick={() => setSettings(true)}>
        {accountScope === "credit_cards" && <>{t("Nur Kreditkarten")} · </>}{fearless ? "Fearless" : t("Zusammenfassungen")} · {reviewBeforeSend ? t("Mit Prüfung") : t("Direkt senden")}
      </button></div>
      <div className="chat-header-actions">
        <button className="chat-quiet-button" disabled={!!busy || !!draft} onClick={newChat}><ChatIcon name="plus" /><span>{t("Neuer Chat")}</span></button>
        <button className="chat-info-button" onClick={() => setSettings(true)} aria-haspopup="dialog"><ChatIcon name="info" /><span>{t("Datenschutz & Modell-Info")}</span></button>
      </div>
    </header>

    <div className="chat-stream" ref={stream} role="log" aria-label={t("Nachrichten")} aria-live="polite" onScroll={e => {
      const element = e.currentTarget;
      followLatest.current = element.scrollHeight - element.scrollTop - element.clientHeight < 80;
    }}>
      {loading ? <p className="chat-empty" role="status">{t("Verbindung wird geprüft …")}</p> : !history.length && !pendingQuestion ? <div className="chat-empty">
        <div className="chat-empty-icon"><ChatIcon name="message" /></div>
        <h2>{t("Was möchtest du über deine Finanzen wissen?")}</h2>
        {!account.connected ? <>
          <p>{account.loginPending ? t("Bitte die Anmeldung im Browser abschliessen. Diese Ansicht aktualisiert sich automatisch.") : t("Verbinde dein ChatGPT-Konto, um loszulegen.")}</p>
          <button className="primary-button" disabled={!!busy} onClick={() => { void login(); }}>{busy === "login" ? t("Anmeldung wird geöffnet …") : account.loginPending ? t("Anmeldung erneut öffnen") : t("Mit ChatGPT anmelden")}</button>
        </> : <div className="chat-suggestions">{["Wie unterscheiden sich meine monatlichen Ausgaben?", "Wie hat sich mein Vermögen entwickelt?"].map(example => <button key={example} onClick={() => { setQuestion(t(example)); input.current?.focus(); }}>{t(example)}</button>)}</div>}
      </div> : null}
      {history.map((message, index) => <article key={index} className={`chat-message ${message.role}`} aria-label={message.role === "user" ? t("Du") : "ChatGPT"}>
        {message.role === "user" ? <p>{message.content}</p> : <ChatMarkdown>{message.content}</ChatMarkdown>}
      </article>)}
      {pendingQuestion && <article className="chat-message user" aria-label={t("Du")}><p>{pendingQuestion}</p></article>}
      {busy === "send" && <div className="chat-message assistant chat-typing" role="status"><span aria-hidden="true">•••</span>{t("ChatGPT erstellt die Antwort …")}</div>}
    </div>

    <footer className="chat-composer-area">
      {error && !settings && <p className="chat-error" role="alert">{t(error)}</p>}
      {history.length >= 12 && <p className="chat-note">{t("Bitte für weitere Fragen einen neuen Chat beginnen.")}</p>}
      {!account.connected && history.length > 0 && <button className="chat-quiet-button" disabled={!!busy} onClick={() => { void login(); }}>{t("Mit ChatGPT anmelden")}</button>}
      {busy === "send" && <button className="chat-quiet-button" onClick={() => { void cancel().catch(e => setError(String(e))); }}>{t("Antwort abbrechen und Verbindung beenden")}</button>}
      <form className="chat-composer" onSubmit={e => { e.preventDefault(); void prepare(); }}>
        <textarea ref={input} rows={1} maxLength={4000} readOnly={dictation.active} value={question} aria-label={t("Nachricht")} placeholder={t("Stelle eine Frage zu deinen Finanzen...")} disabled={loading || !account.connected || !!busy || !!draft || history.length >= 12}
          onChange={e => setQuestion(e.target.value)} onKeyDown={e => {
            if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing && e.nativeEvent.keyCode !== 229) { e.preventDefault(); if (!e.repeat) void prepare(); }
          }} />
        <button type="button" className="chat-mic-button" aria-label={t(dictation.active ? "Spracheingabe beenden" : "Frage sprechen (Deutsch)")} title={t(dictation.active ? "Spracheingabe beenden" : "Frage sprechen (Deutsch)")} aria-pressed={dictation.active} disabled={loading || !account.connected || !!busy || !!draft || history.length >= 12}
          onClick={() => { if (dictation.active) dictation.stop(); else void dictation.start(question); }}>
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true">{dictation.active ? <rect x="7" y="7" width="10" height="10" rx="2" fill="currentColor" /> : <><rect x="9" y="2" width="6" height="12" rx="3" /><path d="M5 10v2a7 7 0 0 0 14 0v-2M12 19v3M8 22h8" /></>}</svg>
        </button>
        <button className="chat-send-button" type="submit" aria-label={t("Nachricht senden")} title={t("Nachricht senden")} disabled={dictation.active || !question.trim() || loading || !account.connected || !!busy || !!draft || history.length >= 12}><ChatIcon name="send" /></button>
      </form>
      {dictation.active && <span className="chat-composer-status" role="status">{t(dictation.phase === "loading" ? "Spracherkennung wird geladen …" : dictation.phase === "finishing" ? "Spracheingabe wird abgeschlossen …" : "Ich höre zu … Deutsch · lokal auf deinem Gerät")}</span>}
      {busy === "preview" && <span className="chat-composer-status" role="status">{t("Vorschau wird erstellt …")}</span>}
    </footer>

    {settings && <ChatDialog title={t("Datenschutz & Modell-Info")} onClose={() => setSettings(false)}>
      {error && <p className="chat-error" role="alert">{t(error)}</p>}
      <p className="chat-account-status">{account.connected ? t("Mit ChatGPT verbunden") : t("Nicht verbunden")} · {account.model}</p>
      <fieldset className="chat-sharing-options" disabled={!!busy || !!draft}>
        <legend>{t("Daten & Versand")}</legend>
        <label className="chat-option"><span><strong>{t("Fearless-Modus: Detailtransaktionen")}</strong><small>{t("Einzelne Buchungen mit Beschreibung, Datum, Betrag, Währung und Kategorie übermitteln – für Fragen zu Händlern wie Bolt. Beschreibungen können persönliche Angaben enthalten.")}</small></span><input type="checkbox" role="switch" checked={fearless} onChange={e => { newChat(); setFearless(e.target.checked); }} /></label>
        <label className="chat-option"><span><strong>{t("Vor dem Senden prüfen")}</strong><small>{t("Neue Daten werden vor dem Senden geprüft. Folgefragen verwenden die bereits freigegebenen Daten und werden direkt gesendet.")} {t("Ausgeschaltet: Enter und der Sendepfeil senden die gewählten Daten direkt an OpenAI, ohne weitere Bestätigung.")}</small></span><input type="checkbox" role="switch" checked={reviewBeforeSend} onChange={e => { newChat(); setReviewBeforeSend(e.target.checked); }} /></label>
        <label>{t("Kontenauswahl")}<select aria-label={t("Kontenauswahl")} value={accountScope} onChange={e => { newChat(); setAccountScope(e.target.value as typeof accountScope); }}>
          <option value="auto">{t("Automatisch nach Frage")}</option><option value="credit_cards">{t("Nur Kreditkarten")}</option><option value="all">{t("Alle aktiven Konten")}</option>
        </select></label>
        <p className="chat-note">{t("Kreditkartenfragen werden automatisch auf Kreditkartenkonten begrenzt. Die Begrenzung bleibt für Folgefragen erhalten. Beim Wechsel werden bisherige Nachrichten verworfen.")}</p>
        <div className="chat-period-fields"><label>{t("Von")}<input type="date" value={from} max={to} onChange={e => { newChat(); setFrom(e.target.value); }} /></label><label>{t("Bis")}<input type="date" value={to} min={from} max={today()} onChange={e => { newChat(); setTo(e.target.value); }} /></label></div>
        <p className="chat-note">{t("Der Zeitraum gilt für die freigegebenen Daten. Änderungen starten einen neuen Chat. Diese Optionen gelten nur in dieser Chatansicht.")}</p>
      </fieldset>
      <p>{t("Melde dich im Browser mit deinem ChatGPT-Konto an. Du brauchst einen für Codex freigeschalteten Zugang. Es gelten die Nutzungslimits deines Kontos; ein API-Schlüssel ist nicht nötig.")}</p>
      <p>{t("Das Mikrofon erkennt deutsche Sprache lokal auf deinem Gerät. Aufnahmen werden weder gespeichert noch hochgeladen. Der erkannte Text wird erst beim Absenden mit deiner Frage übermittelt.")}</p>
      <p>{t("Die ChatGPT-Anmeldung wird für dieses Finanzprofil im geschützten Anmeldespeicher deines Geräts behalten. Beim erneuten Öffnen wirst du automatisch verbunden. „ChatGPT abmelden“ entfernt die gespeicherte Anmeldung. Andere Apps bleiben unberührt.")}</p>
      <p>{t("OpenAI erhält deine Frage, den bisherigen Chat und die gewählten Zusammenfassungen oder Detailtransaktionen. Es gelten die Datenverwendungs- und Speicherregeln deines ChatGPT-Kontos.")}</p>
      <p>{fearless ? t("Im Fearless-Modus werden Buchungsbeschreibungen an OpenAI übermittelt. Sie können Namen, Referenzen oder andere persönliche Angaben enthalten. Separate Konto-, Bank-, IBAN- und Adressfelder bleiben ausgeschlossen. Prüfe die Datenvorschau vor der Freigabe.") : t("Im Modus Zusammenfassungen werden keine Buchungstexte, Konto- oder Banknamen und keine eigenen Kategorienamen übertragen. Persönliche Angaben in deinen Fragen werden dennoch mitgesendet.")}</p>
      <p>{t("KI-Antworten können Fehler enthalten. Prüfe wichtige Zahlen anhand der angezeigten Quellen. Bereits gesendete Inhalte werden durch das Löschen des lokalen Chats nicht beim Anbieter gelöscht.")}</p>
      <div className="chat-actions">
        {!account.connected && <button className="primary-button" disabled={!!busy} onClick={() => { void login(); }}>{t("Mit ChatGPT anmelden")}</button>}
        {(account.connected || account.loginPending) && <button className="chat-quiet-button" disabled={!!busy} onClick={() => { void disconnect().catch(e => setError(String(e))); }}>{account.loginPending ? t("Anmeldung abbrechen") : t("ChatGPT abmelden")}</button>}
        <button className="chat-quiet-button" onClick={() => setSettings(false)}>{t("Zurück zum Chat")}</button>
      </div>
    </ChatDialog>}
    {draft && <ChatDialog title={t("Datenfreigabe prüfen")} onClose={() => { void editDraft().catch(e => setError(String(e))); }}>
      <p className="chat-draft-question">{draft.question}</p><SourcePreview preview={draft} />
      <label className="chat-consent"><input type="checkbox" checked={confirmed} onChange={e => setConfirmed(e.target.checked)} /><span>{t("Ich gebe die angezeigten Anfrageinhalte für die Übermittlung an OpenAI frei.")}</span></label>
      <div className="chat-actions"><button className="primary-button" disabled={!confirmed || !!busy} onClick={() => { void send(); }}>{t("Freigeben & an ChatGPT senden")}</button><button className="chat-quiet-button" onClick={() => { void editDraft().catch(e => setError(String(e))); }}>{t("Frage bearbeiten")}</button></div>
    </ChatDialog>}
  </section>;
}

function ChatMarkdown({ children }: { children: string }) {
  return <Markdown remarkPlugins={[remarkGfm]} skipHtml components={{
    // Answers cannot load remote images or turn model text into navigable links.
    a: ({ children }) => <span>{children}</span>,
    img: () => null,
    table: ({ children }) => <div className="chat-table-scroll"><table>{children}</table></div>,
  }}>{children}</Markdown>;
}

function ChatDialog({ title, onClose, children }: { title: string; onClose: () => void; children: ReactNode }) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => { const element = dialog.current; element?.showModal(); return () => element?.close(); }, []);
  return <dialog ref={dialog} className="chat-dialog" aria-label={title} onCancel={e => { e.preventDefault(); onClose(); }} onClick={e => {
    if (e.target !== e.currentTarget) return;
    const bounds = e.currentTarget.getBoundingClientRect();
    if (e.clientX < bounds.left || e.clientX > bounds.right || e.clientY < bounds.top || e.clientY > bounds.bottom) onClose();
  }}><header><h2>{title}</h2><button className="chat-quiet-button" aria-label={t("Schliessen")} onClick={onClose}><ChatIcon name="close" /></button></header>{children}</dialog>;
}

function ChatIcon({ name }: { name: "plus" | "info" | "send" | "close" | "message" }) {
  const paths = {
    plus: <path d="M12 5v14M5 12h14" />,
    info: <><circle cx="12" cy="12" r="9" /><path d="M12 11v6M12 7v1" /></>,
    send: <path d="m6 11 6-6 6 6M12 5v14" />,
    close: <path d="m6 6 12 12M6 18 18 6" />,
    message: <path d="M5 4h14a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H9l-6 4V6a2 2 0 0 1 2-2Z" />,
  };
  return <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">{paths[name]}</svg>;
}
