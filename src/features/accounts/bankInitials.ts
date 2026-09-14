const institutions: Array<[string[], string]> = [
  [["ubs", "union bank of switzerland"], "UBS"],
  [["swissquote"], "SQ"],
  [["raiffeisen"], "RF"],
  [["postfinance", "post finance"], "PF"],
  [["vontobel", "fontobel"], "VT"],
  [["zurcher kantonalbank", "zurich cantonal bank", "zkb"], "ZKB"],
  [["yuh"], "YUH"],
  [["bank cler", "cler"], "BC"],
  [["banque cantonale vaudoise", "waadtlander kantonalbank", "bcv"], "BCV"],
  [["banque cantonale de geneve", "genfer kantonalbank", "bcge"], "BCGE"],
  [["basler kantonalbank", "bkb"], "BKB"],
  [["julius bar", "julius baer", "juliusbar"], "JB"],
  [["lombard odier"], "LO"],
  [["luzerner kantonalbank", "lukb"], "LUKB"],
  [["migros bank", "migrosbank", "migros"], "MB"],
];

function normalize(value: string) {
  return value.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase().replace(/[^a-z0-9]+/g, " ").trim();
}

export function bankInitials(name: string, providerKey: string): string {
  for (const value of [name, providerKey]) {
    const candidate = ` ${normalize(value)} `;
    const known = institutions.find(([aliases]) => aliases.some(alias => candidate.includes(` ${alias} `)));
    if (known) return known[1];
  }
  const words = name.trim().split(/\s+/).filter(Boolean);
  if (words.length > 1) return words.slice(0, 2).map(word => Array.from(word)[0]).join("").toUpperCase();
  return Array.from(words[0] ?? "?").slice(0, 2).join("").toUpperCase();
}
