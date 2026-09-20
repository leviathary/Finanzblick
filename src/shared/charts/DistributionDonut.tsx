// Shared distribution ring with accessible hover/focus details and optional selection.
import { useState } from "react";
import { locale } from "../../i18n";
import "./distributionDonut.css";

export type DonutSegment = { key: string; label: string; color: string; amountMinor: number; keys: string[]; expanded?: boolean };
export function DistributionDonut({ segments, selected = [], onSelect, total, label, currency }: {
  segments: DonutSegment[]; selected?: string[]; onSelect?: (keys: string[]) => void;
  total: number; label: string; currency: string;
}) {
  const [hover, setHover] = useState<string | null>(null);
  const positive = segments.filter(item=>item.amountMinor>0);
  const sum = positive.reduce((value,item)=>value+item.amountMinor,0);
  const active = positive.find(item=>item.key===hover);
  const money = (value:number) => new Intl.NumberFormat(locale(),{style:"currency",currency}).format(value/100);
  const percent = (value:number) => new Intl.NumberFormat(locale(),{style:"percent",minimumFractionDigits:1,maximumFractionDigits:1}).format(value/sum);
  let offset = 0;
  return <div className="distribution-donut">
    <svg viewBox="0 0 240 240" aria-label={label}>
      {!sum && <circle cx="120" cy="120" r="94" fill="none" stroke="var(--border-subtle)" strokeWidth="28"/>}
      {positive.map(item=>{
        const share=item.amountMinor/sum*100, start=offset; offset+=share;
        const pressed=item.keys.every(key=>selected.includes(key));
        const expandable=item.expanded !== undefined;
        const description=`${item.label}: ${money(item.amountMinor)} (${percent(item.amountMinor)})`;
        return <circle key={item.key} cx="120" cy="120" r="94" pathLength="100" fill="none" stroke={item.color}
          strokeWidth={pressed || hover===item.key ? 35 : 28} strokeDasharray={`${share} ${100-share}`} strokeDashoffset={-start} transform="rotate(-90 120 120)"
          role={onSelect ? "button" : "img"} tabIndex={0} aria-pressed={onSelect && !expandable ? pressed : undefined} aria-expanded={expandable ? item.expanded : undefined} aria-label={description}
          style={{opacity:selected.length && !item.keys.some(key=>selected.includes(key)) ? 0.4 : 1,cursor:onSelect ? "pointer" : "default"}}
          onMouseEnter={()=>setHover(item.key)} onMouseLeave={()=>setHover(null)} onFocus={()=>setHover(item.key)} onBlur={()=>setHover(null)}
          onClick={()=>onSelect?.(item.keys)} onKeyDown={event=>{if(onSelect && (event.key==='Enter'||event.key===' ')){event.preventDefault();onSelect(item.keys);}}}>
          <title>{description}</title>
        </circle>;
      })}
    </svg>
    <div className="distribution-donut-center" aria-live="polite"><span>{active?.label ?? label}</span><strong>{money(active?.amountMinor ?? total)}</strong>{active && <small>{percent(active.amountMinor)}</small>}</div>
  </div>;
}
