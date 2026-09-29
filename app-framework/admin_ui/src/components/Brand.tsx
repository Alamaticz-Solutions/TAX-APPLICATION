import pdsLogoUrl from "../assets/pdsh-logo-nav-2.svg";

export function PdsLogo({ compact = false }: { compact?: boolean }) {
  return (
    <div className={`pds-logo ${compact ? "compact" : ""}`}>
      <img alt="PDS Health" src={pdsLogoUrl} />
    </div>
  );
}
