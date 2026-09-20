// Rendert die lokale, durchsuchbare Hilfe mit direkter Navigation zu einzelnen Artikeln.

import { useEffect, useMemo, useRef, useState } from "react";
import { t } from "../../i18n";
import { getHelpArticles } from "./helpContent";
import "./help.css";

function articleFromHash() {
  const match = window.location.hash.match(/^#help(?:\/([^?]+))?/);
  return match?.[1]?.split("/")[0] ?? "start";
}

export function Help() {
  const articles = getHelpArticles();
  const [activeId, setActiveId] = useState(articleFromHash);
  const [query, setQuery] = useState("");
  const headingRef = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    const sync = () => setActiveId(articleFromHash());
    window.addEventListener("hashchange", sync);
    return () => window.removeEventListener("hashchange", sync);
  }, []);

  const normalizedQuery = query.trim().toLocaleLowerCase();
  const visibleArticles = useMemo(() => {
    if (!normalizedQuery) return articles;
    return articles.filter(article => [
      article.title,
      article.summary,
      ...article.sections.flatMap(section => [
        section.title,
        ...(section.paragraphs ?? []),
        ...(section.steps ?? []),
        ...(section.bullets ?? []),
        section.note ?? "",
      ]),
    ].join(" ").toLocaleLowerCase().includes(normalizedQuery));
  }, [articles, normalizedQuery]);

  const activeArticle = articles.find(article => article.id === activeId) ?? articles[0];

  useEffect(() => {
    headingRef.current?.focus();
  }, [activeArticle.id]);

  return <section className="help-page">
    <div className="overview-heading help-heading">
      <div>
        <p className="eyebrow">Finanzblick</p>
        <h1>{t("Hilfe")}</h1>
        <p className="intro">{t("Anleitungen und Antworten zu den wichtigsten Funktionen – lokal und jederzeit verfügbar.")}</p>
      </div>
    </div>
    <div className="help-layout">
      <aside className="dashboard-card help-index">
        <label htmlFor="help-search">{t("Hilfethemen durchsuchen")}</label>
        <input id="help-search" type="search" value={query} onChange={event => setQuery(event.target.value)} placeholder={t("Thema oder Begriff eingeben")} />
        <nav aria-label={t("Hilfethemen")}>
          {visibleArticles.map(article => <a key={article.id} href={`#help/${article.id}`} aria-current={activeArticle.id === article.id ? "page" : undefined}>
            <strong>{article.title}</strong>
            <span>{article.summary}</span>
          </a>)}
        </nav>
        {visibleArticles.length === 0 && <p className="help-empty" role="status">{t("Keine passenden Hilfethemen gefunden.")}</p>}
      </aside>
      <article className="dashboard-card help-article" aria-labelledby="help-article-title">
        <p className="eyebrow">{t("Hilfe")}</p>
        <h2 id="help-article-title" ref={headingRef} tabIndex={-1}>{activeArticle.title}</h2>
        <p className="help-summary">{activeArticle.summary}</p>
        {activeArticle.sections.map(section => <section key={section.title}>
          <h3>{section.title}</h3>
          {section.paragraphs?.map(paragraph => <p key={paragraph}>{paragraph}</p>)}
          {section.steps && <ol>{section.steps.map(step => <li key={step}>{step}</li>)}</ol>}
          {section.bullets && <ul>{section.bullets.map(bullet => <li key={bullet}>{bullet}</li>)}</ul>}
          {section.note && <aside className="help-note"><strong>{t("Hinweis")}</strong><p>{section.note}</p></aside>}
        </section>)}
      </article>
    </div>
  </section>;
}
