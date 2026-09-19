// Displays provider wealth shares without treating liabilities as positive assets.
import { locale, t } from "../../i18n";
import { DistributionDonut } from "../../shared/charts/DistributionDonut";
import { ProviderLogo } from "../accounts/ProviderLogo";
import "./providerDistribution.css";

type Provider = { provider: string; providerKey: string; balanceMinor: number; accountCount: number; logoDataUrl: string | null };
const colors = ["var(--chart-blue)","var(--accent-positive)","var(--chart-purple)","var(--chart-amber)","#dd6b87","#579ba8","#8b7355","#7889d5"];
export function ProviderDistribution({ providers, total, currency }: { providers: Provider[]; total: number; currency: string }) {
  // Assign colors by identity, independent of balance ranking.
  const keys=providers.map(item=>item.providerKey).sort();
  const sorted=[...providers].sort((a,b)=>b.balanceMinor-a.balanceMinor);
  const color=(key:string)=>{ const index=keys.indexOf(key); return colors[index] ?? `hsl(${Math.round(index*137.5)%360} 55% 55%)`; };
  const positiveTotal=providers.reduce((sum,item)=>sum+Math.max(0,item.balanceMinor),0);
  const money=(value:number)=>new Intl.NumberFormat(locale(),{style:"currency",currency}).format(value/100);
  return <>
    <div className="provider-distribution">
      <DistributionDonut segments={sorted.map(item=>({key:item.providerKey,label:item.provider,color:color(item.providerKey),amountMinor:item.balanceMinor,keys:[item.providerKey]}))} total={total} currency={currency} label={t("Gesamtvermögen")}/>
      <div className="provider-distribution-list">
        {sorted.map(item=><div className="provider-distribution-row" key={item.providerKey}>
          <span className="provider-distribution-dot" style={{background:color(item.providerKey)}} aria-hidden="true"/>
          <ProviderLogo name={item.provider} providerKey={item.providerKey} customLogo={item.logoDataUrl}/>
          <div><strong>{item.provider}</strong><small>{item.accountCount} {item.accountCount===1?t("Konto"):t("Konten")}</small></div>
          <div className="provider-distribution-value"><strong>{money(item.balanceMinor)}</strong><small>{item.balanceMinor<0 ? t("Negativer Saldo") : new Intl.NumberFormat(locale(),{style:"percent",maximumFractionDigits:1}).format(positiveTotal ? item.balanceMinor/positiveTotal : 0)}</small></div>
        </div>)}
      </div>
    </div>
    {providers.some(item=>item.balanceMinor<0) && <p className="provider-distribution-note">{t("Der Ring und die Prozentanteile zeigen nur positive Anbietersalden. Negative Salden sind im Gesamtvermögen enthalten.")}</p>}
    {!positiveTotal && <p className="provider-distribution-note">{t("Keine positiven Anbietersalden vorhanden.")}</p>}
  </>;
}
