export function monthRange(month: string): { from: string; to: string } | null {
  if (!/^[1-9]\d{3}-(0[1-9]|1[0-2])$/.test(month)) return null;
  const [year, number] = month.split("-").map(Number);
  const days = new Date(Date.UTC(year, number, 0)).getUTCDate();
  return { from: `${month}-01`, to: `${month}-${days}` };
}

export function shiftMonth(month: string, offset: -1 | 1): string {
  if (!monthRange(month)) return "";
  const [year, number] = month.split("-").map(Number);
  const date = new Date(Date.UTC(year, number - 1 + offset, 1));
  const shifted = `${date.getUTCFullYear()}-${String(date.getUTCMonth() + 1).padStart(2, "0")}`;
  return monthRange(shifted) ? shifted : "";
}
