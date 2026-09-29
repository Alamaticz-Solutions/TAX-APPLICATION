/**
 * The product's recipe registration for the My Work journey, asserted at
 * module load against the packaged PDS recipe registry so a drifting recipe
 * id, renderer, presentation schema, or capability set fails fast.
 *
 * This mirrors the backend registration in
 * `backend/src/services/ix/orchestration.rs` (my_work_recipe_registration).
 */

import {
  assertPdsIxRecipeRegistration,
  pdsIxRecipeRegistrationSchema,
  type PdsIxRecipeRegistration
} from '@appfw/pds-ix-presentation-contract';

export const NEXUS_MY_WORK_INTENT_KEY = 'pds.ix.intent.attention-stewardship@1';
export const NEXUS_MY_WORK_ARTIFACT_TYPE = 'nexus.ix.my-work-brief@1';
export const NEXUS_MY_WORK_RENDERER_KEY = 'pds.ix.recipe.attention-stewardship@1';

export const myWorkRecipeRegistration: PdsIxRecipeRegistration =
  assertPdsIxRecipeRegistration({
    schemaVersion: pdsIxRecipeRegistrationSchema,
    recipeId: 'attention-stewardship',
    intentKey: NEXUS_MY_WORK_INTENT_KEY,
    artifactType: NEXUS_MY_WORK_ARTIFACT_TYPE,
    contentSchemaVersion: 'pds.ix.presentation@1',
    rendererKey: NEXUS_MY_WORK_RENDERER_KEY,
    requiredCapabilities: [
      'pds.ix.capability.focus-context@1',
      'pds.ix.capability.progressive-presentation@1',
      'pds.ix.capability.evidence-disclosure@1',
      'pds.ix.capability.human-control@1',
      'pds.ix.capability.attention-correction@1'
    ]
  });
