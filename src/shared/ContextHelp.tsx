// Stellt kurze, kontextbezogene Begriffserklärungen als einheitlichen Info-Auslöser bereit.

import { type ReactNode, useId } from "react";

interface ContextHelpProps {
  label: string;
  children: ReactNode;
  className?: string;
}

export function ContextHelp({ label, children, className = "" }: ContextHelpProps) {
  const tooltipId = useId();

  return <button
    type="button"
    className={`context-help ${className}`.trim()}
    aria-label={label}
    aria-describedby={tooltipId}
  >
    <span className="context-help-icon" aria-hidden="true">i</span>
    <span className="context-help-tooltip" id={tooltipId} role="tooltip">{children}</span>
  </button>;
}
