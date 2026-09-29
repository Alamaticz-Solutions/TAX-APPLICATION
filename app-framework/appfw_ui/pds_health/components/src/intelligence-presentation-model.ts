/**
 * Web-package adapter for the canonical React-free presentation contract.
 *
 * The source of truth lives in the standalone
 * `@appfw/pds-ix-presentation-contract` package. This web package re-exports
 * the contract so existing web consumers keep a stable subpath while native
 * renderers can consume the same model without acquiring React DOM or CSS.
 */
export {
  assertPdsIxPresentation,
  pdsIxPresentationSchema,
  validatePdsIxPresentation
} from "@appfw/pds-ix-presentation-contract";

export type {
  EvidenceDisclosureItemModel,
  EvidenceDisclosureModel,
  PdsIxPresentationEnvelope,
  PdsIxPresentationIdentity,
  PdsIxPresentationValidation,
  ProgressiveResponsePresentationModel,
  ProgressiveResponseRegionModel,
  ResolvedContextPresentationModel,
  WorkStatusPresentationModel
} from "@appfw/pds-ix-presentation-contract";
