// The catalog app is data-driven from the same product-neutral manifest the
// component checker validates (reference/catalog.json) and the typed component
// exports (pdsComponentFamilies). Reading the manifest here keeps the
// interactive catalog and the CI-checked static catalog from drifting.
import { pdsComponentFamilies, pdsAgentDecisionGuide } from "@appfw/pds-health-components";
import catalogManifest from "../../../reference/catalog.json";

export type FamilyContract = {
  props: string;
  states: string;
  density: string;
  accessibility: string;
  usage: string;
  verification: string;
};

export type FamilyReadiness = {
  level: string;
  agentUse: readonly string[];
  productBoundary: string;
  evidence: readonly string[];
};

export type CatalogFamily = {
  slug: string;
  name: string;
  purpose: string;
  components: readonly string[];
  contract: FamilyContract;
  readiness: FamilyReadiness;
};

export type AgentRecipe = {
  slug: string;
  intent: string;
  startWith: readonly string[];
  productOwns: string;
  avoid?: string;
  evidence: readonly string[];
};

export type ComponentStatus = "stable" | "beta" | "experimental" | "deprecated";

export type ComponentLifecycle = {
  status: ComponentStatus;
  since: string;
  deprecated?: { since: string; removeBy?: string; replacement?: string; note?: string };
};

export type Versioning = {
  packageVersion: string;
  scheme: string;
  changelog: string;
  statuses: readonly ComponentStatus[];
  deprecationPolicy: string;
};

type Manifest = {
  name: string;
  brand: string;
  purpose: string;
  agentContract: {
    startHere: readonly string[];
    do: readonly string[];
    avoid: readonly string[];
  };
  versioning: Versioning;
  componentLifecycle: Record<string, ComponentLifecycle>;
  families: readonly CatalogFamily[];
  agentDecisionGuide: readonly AgentRecipe[];
};

const manifest = catalogManifest as unknown as Manifest;

export const catalogName = manifest.name;
export const catalogBrand = manifest.brand;
export const catalogPurpose = manifest.purpose;
export const agentContract = manifest.agentContract;
export const families = manifest.families;
export const agentDecisionGuide: readonly AgentRecipe[] = manifest.agentDecisionGuide;
export const versioning = manifest.versioning;
export const packageVersion = manifest.versioning?.packageVersion ?? "0.0.0";

export function lifecycleFor(name: string): ComponentLifecycle | undefined {
  return manifest.componentLifecycle?.[name];
}

// Maturity comes from the typed component source of truth, keyed by family name.
const maturityByName = new Map(pdsComponentFamilies.map((family) => [family.name, family.maturity]));

export function familyMaturity(name: string): string {
  return maturityByName.get(name as never) ?? "foundation";
}

export type ComponentEntry = {
  name: string;
  familySlug: string;
  familyName: string;
};

export const allComponents: readonly ComponentEntry[] = families.flatMap((family) =>
  family.components.map((name) => ({
    name,
    familySlug: family.slug,
    familyName: family.name
  }))
);

export const componentCount = allComponents.length;

// Which agent recipes reference a given component, so each card can point an
// agent at the workflow the component belongs to.
export function recipesForComponent(name: string): readonly AgentRecipe[] {
  return agentDecisionGuide.filter((recipe) => recipe.startWith.includes(name));
}

export function matchesQuery(family: CatalogFamily, component: string, query: string): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  // Match on the component name (precise) or the family name (so a family term
  // like "forms" surfaces the whole family). Free-text purpose is deliberately
  // excluded — matching it casts too wide a net for a component lookup.
  return (
    component.toLowerCase().includes(needle) ||
    family.name.toLowerCase().includes(needle)
  );
}
