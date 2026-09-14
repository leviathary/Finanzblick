// Local links from a chat snapshot open the same period in the source reports.
export function reportPeriod() {
  const query = new URLSearchParams(window.location.hash.split("?")[1] ?? "");
  const from = query.get("from") ?? "";
  const to = query.get("to") ?? "";
  const valid = (value: string) => /^\d{4}-\d{2}-\d{2}$/.test(value)
    && !Number.isNaN(Date.parse(`${value}T12:00:00Z`))
    && new Date(`${value}T12:00:00Z`).toISOString().slice(0, 10) === value;
  return valid(from) && valid(to) && from <= to ? { from, to } : null;
}
