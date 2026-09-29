import { Badge, PageHeader, Surface } from '@appfw/pds-health-components';

const PDS_HEALTH_COMPONENTS_NAME = '@appfw/pds-health-components';
const PDS_HEALTH_COMPONENTS_VERSION = '0.12.0';
const PDS_HEALTH_COMPONENTS_SHA256 =
  'b5c655533e9b935df92dc348b05b1966226bb6532a323c37ccf77ea12272b9e0';
const PDS_IX_PRESENTATION_CONTRACT_NAME = '@appfw/pds-ix-presentation-contract';
const PDS_IX_PRESENTATION_CONTRACT_VERSION = '0.2.0';
const PDS_IX_PRESENTATION_CONTRACT_SHA256 =
  '3a0a3404e2186e87302af019c1eca5dee5a80de9d10e98383f3fdcfcae884364';

export function AboutPage() {
  return (
    <>
      <PageHeader
        eyebrow="Package consume"
        title="About PDS Nexus Web"
        subtitle="Visible B_IX-family consume identities for this frontend."
      />
      <Surface
        title="Consumed packages"
        subtitle="Receipted archives are vendored under frontend/vendor and installed through file: pins."
      >
        <dl>
          <div>
            <dt>PDS Health components</dt>
            <dd>
              <strong>{PDS_HEALTH_COMPONENTS_NAME}</strong>
              <span>{PDS_HEALTH_COMPONENTS_VERSION}</span>
              <span>{PDS_HEALTH_COMPONENTS_SHA256}</span>
            </dd>
          </div>
          <div>
            <dt>IX presentation contract</dt>
            <dd>
              <strong>{PDS_IX_PRESENTATION_CONTRACT_NAME}</strong>
              <span>{PDS_IX_PRESENTATION_CONTRACT_VERSION}</span>
              <span>{PDS_IX_PRESENTATION_CONTRACT_SHA256}</span>
            </dd>
          </div>
        </dl>
        <p>
          This page reuses the receipted App Framework / PDS packages. It does
          not introduce a Nexus-prefixed design-system or presentation-contract
          fork, an appfw_ui source alias, the held 0.9.0 archive, or native 0.2.0.
        </p>
        <div aria-label="Consume posture">
          <Badge tone="accent">B_IX family</Badge>
          <Badge tone="neutral">0.12.0 + 0.2.0</Badge>
          <Badge tone="warning">0.9.0 not installed</Badge>
        </div>
      </Surface>
    </>
  );
}
