export const canonicalIxReferenceSpecPath = "docs/specs/ix-eight-vignette-shared-foundation-r1.md";

const historicalPaths = [
  "docs/specs/nexus-pds-design-system-foundation-r1.md",
  "docs/specs/nexus-ix-lifecycle-adapter-r1.md"
];

export function assertCanonicalIxReferenceSpec(relativePath, content) {
  if (relativePath !== canonicalIxReferenceSpecPath) {
    throw new Error(`IX reference checker requires canonical source ${canonicalIxReferenceSpecPath}.`);
  }
  if (typeof content !== "string" || content.length === 0) {
    throw new Error(`IX reference canonical source is missing: ${canonicalIxReferenceSpecPath}.`);
  }
  const historicalPath = historicalPaths.find((candidate) => content.includes(candidate));
  if (historicalPath) {
    throw new Error(`IX reference canonical source reintroduced historical path ${historicalPath}.`);
  }
}
