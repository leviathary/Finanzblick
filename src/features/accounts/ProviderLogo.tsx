import { bankLogo } from "./bankLogos";

export function ProviderLogo({ name, providerKey, customLogo }: { name: string; providerKey: string; customLogo?: string | null }) {
  const logo = customLogo || bankLogo(name, providerKey);
  return <span className={`provider-mark provider-logo ${providerKey}`} aria-hidden="true">
    {logo ? <img src={logo} alt=""/> : <span>{monogram(name)}</span>}
  </span>;
}

function monogram(name: string) {
  const normalized = name.toLowerCase();
  if (normalized.includes("ubs")) return "UBS";
  if (normalized.includes("swissquote")) return "SQ";
  return name.trim().slice(0, 1).toUpperCase();
}
