import { bankInitials } from "./bankInitials";

export function ProviderLogo({ name, providerKey, customLogo }: { name: string; providerKey: string; customLogo?: string | null }) {
  return <span className="provider-mark provider-logo" aria-hidden="true">
    {customLogo ? <img src={customLogo} alt=""/> : <span>{bankInitials(name, providerKey)}</span>}
  </span>;
}
