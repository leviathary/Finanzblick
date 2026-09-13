export function filenameFilter(query: string, regex: boolean): { matches: (name: string) => boolean; error: string | null } {
  if (!query) return { matches: () => true, error: null };
  try {
    // Ordinary searches match a substring; only * and ? have special meaning.
    const pattern = regex ? query : query.split("*").map(part => part.split("?").map(text => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join(".")).join(".*");
    const expression = new RegExp(pattern, "i");
    return { matches: name => expression.test(name), error: null };
  } catch {
    return { matches: () => false, error: "Ungültiger regulärer Ausdruck. Bitte prüfe deine Eingabe." };
  }
}
