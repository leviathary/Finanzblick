// Groups expense categories for the shared donut and synchronizes selection with the list.
import { categoryName, t } from "../../i18n";
import { DistributionDonut, type DonutSegment } from "../../shared/charts/DistributionDonut";

type Category = { key: string; label: string; color: string; amountMinor: number };
export function splitPositiveCategories<T extends Category>(categories: T[]) {
  const positive = categories.filter(item => item.amountMinor > 0).sort((a,b) => b.amountMinor - a.amountMinor);
  return { primary: positive.slice(0,8), remaining: positive.slice(8) };
}
export function buildCategoryDonutSegments(categories: Category[], remainingExpanded: boolean): DonutSegment[] {
  const { primary, remaining } = splitPositiveCategories(categories);
  const segments: DonutSegment[] = primary.map(item => ({...item, label: categoryName(item.key,item.label), keys:[item.key]}));
  if (remaining.length) segments.push({key:"__remaining",label:t("Weitere Kategorien"),color:"var(--text-secondary)",amountMinor:remaining.reduce((sum,item)=>sum+item.amountMinor,0),keys:remaining.map(item=>item.key),expanded:remainingExpanded});
  return segments;
}
export function CategoryDonut({ categories, selected, onSelect, total, label, remainingExpanded }: {
  categories: Category[]; selected: string[]; onSelect: (keys: string[]) => void; total: number; label: string; remainingExpanded: boolean;
}) {
  const { primary } = splitPositiveCategories(categories);
  const segments = buildCategoryDonutSegments(categories, remainingExpanded);
  if (!primary.length) return null;
  return <div className="category-donut">
    <DistributionDonut segments={segments} selected={selected} onSelect={onSelect} total={total} label={label} currency="CHF"/>
    {categories.some(item=>item.amountMinor<0) && <p>{t("Der Ring zeigt positive Kategoriesummen. Negative Summen bleiben in der Liste und im Gesamtbetrag berücksichtigt.")}</p>}
  </div>;
}
