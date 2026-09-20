// Vergibt stabile, semantisch geeignete Diagrammfarben anhand der Datenidentität.

const distributionPalette = [
  "var(--chart-distribution-1)",
  "var(--chart-distribution-2)",
  "var(--chart-distribution-3)",
  "var(--chart-distribution-4)",
  "var(--chart-distribution-5)",
  "var(--chart-distribution-6)",
  "var(--chart-distribution-7)",
  "var(--chart-distribution-8)",
];

export function distributionColor(key: string, keys: string[]): string {
  const index = keys.indexOf(key);
  return distributionPalette[index] ?? `hsl(${Math.round(index * 137.5) % 360} 48% 45%)`;
}
