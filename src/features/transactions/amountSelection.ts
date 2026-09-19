// Erweitert oder verkleinert die Buchungsauswahl über einen zusammenhängenden Bereich.

export function selectAmountRange(base: number[], ordered: number[], start: number, end: number, selecting: boolean): number[] {
  const first = ordered.indexOf(start);
  const last = ordered.indexOf(end);
  if (first < 0 || last < 0) return base;
  const range = ordered.slice(Math.min(first, last), Math.max(first, last) + 1);
  return selecting ? [...new Set([...base, ...range])] : base.filter(id => !range.includes(id));
}
