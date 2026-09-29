import {
  projectPdsIxRecipeRegistration,
  type PdsIxRecipeProjection,
  type PdsIxRendererProjection
} from "@appfw/pds-ix-presentation-contract";

export type PdsIxNativeProjection = Extract<
  PdsIxRendererProjection,
  "native-ios" | "native-android"
>;

/**
 * Pure registration projection for admission tooling and non-rendering consumers.
 */
export function resolvePdsIxNativeRecipe(
  registration: unknown,
  projection: PdsIxNativeProjection
): PdsIxRecipeProjection {
  if (projection !== "native-ios" && projection !== "native-android") {
    throw new TypeError(`Unsupported native PDS IX renderer projection: ${String(projection)}`);
  }
  return projectPdsIxRecipeRegistration(registration, projection);
}
