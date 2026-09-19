// Groups expense categories for the shared donut and synchronizes selection with the list.
import { categoryName, t } from "../../i18n";
import { DistributionDonut } from "../../shared/charts/DistributionDonut";

type Category = { key: string; label: string; color: string; amountMinor: number };
export function CategoryDonut({ categories, selected, onSelect, total, label }: {
  categories: Category[]; selected: string[]; onSelect: (keys: string[]) => void; total: number; label: string;
}) {
  const positive = categories.filter(item => item.amountMinor > 0).sort((a,b) => b.amountMinor - a.amountMinor);
  const segments = positive.slice(0,8).map(item => ({...item, label: categoryName(item.key,item.label), keys:[item.key]}));
  const rest = positive.slice(8);
  if (rest.length) segments.push({key:"__remaining",label:t("Weitere Kategorien"),color:"var(--text-secondary)",amountMinor:rest.reduce((sum,item)=>sum+item.amountMinor,0),keys:rest.map(item=>item.key)});
  if (!positive.length) return null;
  return <div className="category-donut">
    <DistributionDonut segments={segments} selected={selected} onSelect={onSelect} total={total} label={label} currency="CHF"/>
    {categories.some(item=>item.amountMinor<0) && <p>{t("Der Ring zeigt positive Kategoriesummen. Negative Summen bleiben in der Liste und im Gesamtbetrag berücksichtigt.")}</p>}
  </div>;
}
