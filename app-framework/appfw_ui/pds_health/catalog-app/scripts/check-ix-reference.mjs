import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";
import {
  assertCanonicalIxReferenceSpec,
  canonicalIxReferenceSpecPath
} from "./ix-reference-spec-source.mjs";

const scriptRoot = dirname(fileURLToPath(import.meta.url));
const catalogRoot = resolve(scriptRoot, "..");
const repoRoot = resolve(catalogRoot, "../../..");

const files = {
  html: resolve(catalogRoot, "ix-reference.html"),
  vite: resolve(catalogRoot, "vite.config.ts"),
  tsconfig: resolve(catalogRoot, "tsconfig.json"),
  catalogApp: resolve(catalogRoot, "src/App.tsx"),
  app: resolve(catalogRoot, "src/ix-reference/IxReferenceApp.tsx"),
  contract: resolve(catalogRoot, "src/ix-reference/contract.ts"),
  fixtures: resolve(catalogRoot, "src/ix-reference/fixtures.ts"),
  run: resolve(catalogRoot, "src/ix-reference/useFixtureRun.ts"),
  analyze: resolve(catalogRoot, "src/ix-reference/AnalyzeWhyRecipe.tsx"),
  conversation: resolve(catalogRoot, "src/ix-reference/ContextualConversationRecipe.tsx"),
  composition: resolve(catalogRoot, "src/ix-reference/AdaptiveCompositionRecipe.tsx"),
  goalPlan: resolve(catalogRoot, "src/ix-reference/WorkingGoalPlanRecipe.tsx"),
  informationLens: resolve(catalogRoot, "src/ix-reference/AdaptiveInformationLensRecipe.tsx"),
  strategy: resolve(catalogRoot, "src/ix-reference/SituationToStrategyRecipe.tsx"),
  attention: resolve(catalogRoot, "src/ix-reference/AttentionStewardshipRecipe.tsx"),
  agents: resolve(catalogRoot, "src/ix-reference/AmbientAgentContinuityRecipe.tsx"),
  components: resolve(catalogRoot, "src/ix-reference/components.tsx"),
  appCss: resolve(catalogRoot, "src/app.css"),
  css: resolve(catalogRoot, "src/ix-reference/ix-reference.css"),
  presentation: resolve(repoRoot, "appfw_ui/pds_health/components/src/intelligence-presentation.tsx"),
  presentationModel: resolve(repoRoot, "appfw_ui/pds_health/components/src/intelligence-presentation-model.ts"),
  presentationContractRuntime: resolve(repoRoot, "appfw_ui/pds_health/ix-presentation-contract/src/index.js"),
  presentationContractTypes: resolve(repoRoot, "appfw_ui/pds_health/ix-presentation-contract/src/index.d.ts"),
  recipeRegistry: resolve(repoRoot, "appfw_ui/pds_health/ix-presentation-contract/registry/pds.ix.recipe-registry.v1.json"),
  catalogManifest: resolve(repoRoot, "appfw_ui/pds_health/reference/catalog.json"),
  nativePackage: resolve(repoRoot, "appfw_ui/pds_health/native-components/package.json"),
  nativeDesignData: resolve(repoRoot, "appfw_ui/pds_health/tokens/pdsNativeDesignData.json"),
  pdsCss: resolve(repoRoot, "appfw_ui/pds_health/components/src/styles.css"),
  spec: resolve(repoRoot, canonicalIxReferenceSpecPath)
};

const content = Object.fromEntries(await Promise.all(
  Object.entries(files).map(async ([key, path]) => [key, await readFile(path, "utf8")])
));
assertCanonicalIxReferenceSpec(canonicalIxReferenceSpecPath, content.spec);
const catalogManifest = JSON.parse(content.catalogManifest);
const recipeRegistry = JSON.parse(content.recipeRegistry);
const nativePackage = JSON.parse(content.nativePackage);
const nativeDesignData = JSON.parse(content.nativeDesignData);

function transpile(source, fileName) {
  return ts.transpileModule(source, {
    fileName,
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022
    }
  }).outputText;
}

const contractModuleUrl = `data:text/javascript;base64,${Buffer.from(
  transpile(content.contract, files.contract)
).toString("base64")}`;
const contractModule = await import(contractModuleUrl);
const fixtureModuleSource = transpile(content.fixtures, files.fixtures).replace(
  'from "./contract"',
  `from "${contractModuleUrl}"`
);
const fixtureModule = await import(
  `data:text/javascript;base64,${Buffer.from(fixtureModuleSource).toString("base64")}`
);

function finalArtifact(schedule) {
  return schedule
    .map(({ event }) => event.type === "fixture_revision_displayed" ? event.artifact : undefined)
    .filter(Boolean)
    .at(-1);
}

function hasUniqueRegionIds(artifact) {
  const regionIds = artifact.regions.map(({ id }) => id);
  return new Set(regionIds).size === regionIds.length;
}

function runSchedule(schedule, preservedArtifact) {
  const firstEvent = schedule[0].event;
  let state = contractModule.beginIxReferencePlaybackState(
    firstEvent.playbackId,
    firstEvent.type === "playback_started" ? firstEvent.intentLabel : "Reference run",
    preservedArtifact
  );
  schedule.forEach(({ event }) => {
    state = contractModule.reduceIxReferencePlaybackEvent(state, event);
  });

  return state;
}

function runScheduleWithPlaybackPause(schedule, preservedArtifact, pauseAfterIndex) {
  const firstEvent = schedule[0].event;
  let state = contractModule.beginIxReferencePlaybackState(
    firstEvent.playbackId,
    firstEvent.type === "playback_started" ? firstEvent.intentLabel : "Reference run",
    preservedArtifact
  );
  let pausedState;

  schedule.forEach(({ event }, index) => {
    if (index === pauseAfterIndex + 1) {
      state = contractModule.pauseIxReferencePlaybackState(state);
      pausedState = state;
      state = contractModule.resumeIxReferencePlaybackState(state);
    }
    state = contractModule.reduceIxReferencePlaybackEvent(state, event);
  });

  if (pauseAfterIndex === schedule.length - 1) {
    state = contractModule.pauseIxReferencePlaybackState(state);
    pausedState = state;
  }

  return { state, pausedState };
}

function withEventIdentity(event, sequence, payload) {
  return {
    schemaVersion: event.schemaVersion,
    eventId: `${event.playbackId}:${sequence}`,
    playbackId: event.playbackId,
    sequence,
    occurredAt: event.occurredAt,
    ...payload
  };
}

function rejects(action, expectedMessage) {
  try {
    action();
    return false;
  } catch (error) {
    return error instanceof Error && error.message.includes(expectedMessage);
  }
}

function accepts(action) {
  try {
    action();
    return true;
  } catch {
    return false;
  }
}

const checks = [];
function check(name, ok, detail) {
  checks.push({ name, ok, detail });
}

const presentationProjectionEntries = [
  "PdsIxPresentation",
  "EvidenceDisclosure",
  "ResolvedContextDisclosure",
  "WorkStatus",
  "ProgressiveResponse",
  "PdsIxRecipePresentation"
];
const projectionIds = ["web-dom", "native-ios", "native-android"];
const projectionLedger = catalogManifest.rendererProjectionLedger;
check(
  "presentation-projection-ledger",
  projectionLedger?.contract === "pds.ix.presentation@1"
    && JSON.stringify(projectionLedger.projections) === JSON.stringify(projectionIds)
    && presentationProjectionEntries.every((entry) => {
      const projections = projectionLedger.entries?.[entry];
      return projections
        && JSON.stringify(Object.keys(projections)) === JSON.stringify(projectionIds)
        && projections["native-ios"].applicability === "supported"
        && projections["native-ios"].readiness === "not-qualified"
        && projections["native-android"].applicability === "supported"
        && projections["native-android"].readiness === "not-qualified";
    }),
  "The six presentation entries must retain closed Web/iOS/Android projection records and truthful native non-qualification."
);
check(
  "native-package-identity-and-design-data",
  nativePackage.name === "@appfw/pds-health-native"
    && nativePackage.exports?.["./design-data"]
    && nativeDesignData.schemaVersion === "pds.native.design-data@1"
    && JSON.stringify(Object.keys(nativeDesignData.visualThemes ?? {})) === JSON.stringify(["apple-like"])
    && nativeDesignData.defaultSelection?.visualTheme === "apple-like"
    && nativeDesignData.defaultSelection?.colorScheme === "light"
    && nativeDesignData.platforms?.["native-ios"]?.visualTheme === "apple-like"
    && nativeDesignData.platforms?.["native-ios"]?.qualification === "not-qualified"
    && nativeDesignData.platforms?.["native-android"]?.visualTheme === null
    && nativeDesignData.platforms?.["native-android"]?.qualification === "not-qualified",
  "The PDS native package must retain one additive Apple-like projection and truthful unqualified iOS/Android platform status without claiming a Material implementation."
);

const eightReferences = [
  "Analyze Why",
  "Contextual Conversation",
  "Adaptive Composition",
  "Working Goal Plan",
  "Adaptive Information Lens",
  "Situation to Strategy",
  "Attention Stewardship",
  "Agents Helping You"
];

const canonicalReferenceIdentity = recipeRegistry.recipes.map(({ id, ordinal, name }) => ({
  id,
  number: ordinal,
  name
}));
const privateReferenceIdentity = fixtureModule.IX_REFERENCES.map(({ id, number, name }) => ({
  id,
  number,
  name
}));
check(
  "canonical-eight-recipe-registry-parity",
  recipeRegistry.schemaVersion === "pds.ix.recipe_registry@1"
    && recipeRegistry.presentationSchemaVersion === "pds.ix.presentation@1"
    && JSON.stringify(recipeRegistry.projections) === JSON.stringify(projectionIds)
    && recipeRegistry.recipes.length === 8
    && JSON.stringify(privateReferenceIdentity) === JSON.stringify(canonicalReferenceIdentity)
    && JSON.stringify(eightReferences) === JSON.stringify(recipeRegistry.recipes.map(({ name }) => name))
    && recipeRegistry.recipes.every((recipe) => (
      recipe.requiredCapabilities.length === 5
      && new Set(recipe.requiredCapabilities).size === 5
      && recipe.requiredCapabilities.every((capability) => recipeRegistry.capabilities.includes(capability))
      && Object.keys(recipe.projections).join("|") === projectionIds.join("|")
    )),
  "Catalog-private fixtures must retain exact ordered ID/name parity with the canonical public eight-recipe registry without becoming public contract types."
);

for (const reference of eightReferences) {
  check(
    `reference-preserved:${reference}`,
    content.fixtures.includes(`name: "${reference}"`)
      && content.spec.toLowerCase().replaceAll("-", " ").includes(reference.toLowerCase()),
    "Reference must exist in both the gallery descriptor and retained gallery proof contract."
  );
}

check(
  "eight-interactive-references",
  (content.fixtures.match(/\n    availability: "interactive"/g) ?? []).length === 8
    && !content.fixtures.includes("preserved-next")
    && content.app.includes("<AmbientAgentContinuityRecipe />"),
  "All eight references must be selectable interactive recipes."
);

check(
  "catalog-entry",
  content.html.includes("/src/ix-reference/main.tsx")
    && content.vite.includes('"ix-reference": path.resolve(appRoot, "ix-reference.html")')
    && content.catalogApp.includes('id: "intelligent-experience"')
    && content.catalogApp.includes('src: "./ix-reference.html"'),
  "Vite must build the IX reference and the normal Design System navigation must expose exactly one entry."
);

check(
  "ix-only-two-row-reference-layout",
  content.catalogApp.includes('design-system-app__reference design-system-app__reference--ix')
    && content.catalogApp.includes(': "design-system-app__reference"')
    && content.appCss.includes(".design-system-app__reference {\n  display: grid;\n  grid-template-rows: minmax(0, 1fr);")
    && content.appCss.includes(".design-system-app__reference--ix {\n  grid-template-rows: auto minmax(0, 1fr);"),
  "Only the IX reference may reserve a notice row; every existing reference iframe retains one-row fill."
);

check(
  "explicit-public-ix-subpaths-and-aliases",
  content.components.includes('from "@appfw/pds-health-components/intelligence-presentation"')
    && content.components.includes('from "@appfw/pds-health-components/intelligence-presentation-model"')
    && !content.components.includes('PdsIxPresentationEnvelope } from "@appfw/pds-health-components"')
    && [
      "@appfw/pds-health-components/intelligence-presentation",
      "@appfw/pds-health-components/intelligence-presentation-model",
      "@appfw/pds-health-components/ix-recipes",
      "@appfw/pds-ix-presentation-contract"
    ].every((specifier) => content.vite.includes(`\"${specifier}\"`)
      && content.tsconfig.includes(`\"${specifier}\"`)),
  "IX catalog source must use explicit public subpaths backed by exact source aliases, never a local package dependency."
);

check(
  "integrated-shell-links-target-top",
  (content.app.match(/target="_top"/g) ?? []).length === 2
    && content.app.includes('aria-label="Return to the full PDS Design System"')
    && content.app.includes('aria-label="Open the full PDS Design System"'),
  "Both links back to the Design System must escape the embedded reference frame instead of nesting the catalog."
);

check(
  "catalog-private-playback-schema",
  content.contract.includes('pds.ix.reference_playback@1'),
  "The deterministic recipes must identify their catalog-private playback protocol."
);

check(
  "catalog-private-playback-non-authority",
  content.contract.includes("deterministic catalog fixture playback only")
    && content.fixtures.includes("Catalog-only display fixtures")
    && content.fixtures.includes("do not reproduce or assert")
    && !content.contract.includes("appfw.ix_event@1")
    && !content.contract.includes("IxRunLifecycle")
    && !content.contract.includes("export type IxEvent")
    && !content.contract.includes("reduceIxEvent"),
  "The gallery player must remain explicitly catalog-only and must not declare shared runtime or authority contracts."
);

check(
  "closed-purpose-shaped-artifact-union",
  [
    "adaptive-composition",
    "working-goal-plan",
    "adaptive-information-lens",
    "situation-to-strategy",
    "attention-stewardship",
    "ambient-agent-continuity"
  ].every((kind) => content.contract.includes(`kind: "${kind}"`))
    && content.contract.includes("referenceRecipe?: IxReferenceRecipeArtifact")
    && content.contract.includes("pds.ix.reference_playback@1"),
  "Purpose-shaped reference payloads must remain a closed catalog-only fixture union."
);

check(
  "revision-starts-at-one",
  content.contract.includes("artifact revision must begin at 1 and advance by one")
    && content.fixtures.includes("revision: 1")
    && !content.fixtures.includes("revision: 0"),
  "Public artifact revision 0 is invalid."
);

check(
  "public-revision-bound-browser-safe",
  contractModule.IX_MAX_PUBLIC_ARTIFACT_REVISION === Number.MAX_SAFE_INTEGER
    && content.contract.includes("IX_MAX_PUBLIC_ARTIFACT_REVISION = 9_007_199_254_740_991")
    && content.contract.includes("Number.isSafeInteger"),
  "The catalog fixture player must enforce the browser-exact revision maximum for gallery playback."
);

const firstPlaybackFinished = content.fixtures.indexOf('type: "playback_finished"');
const firstArtifact = content.fixtures.indexOf('type: "fixture_revision_displayed"');
check(
  "usable-partial-before-terminal",
  firstArtifact >= 0 && firstPlaybackFinished > firstArtifact
    && (content.fixtures.match(/type: "fixture_revision_displayed"/g) ?? []).length >= 5,
  "Useful artifact revisions must appear before terminal completion."
);

check(
  "exact-resume",
  content.run.includes("nextIndexRef.current")
    && content.run.includes("playFrom(nextIndexRef.current, token)"),
  "Resume continues from the next unprocessed fixture event instead of appending an invented update."
);

check(
  "analysis-refresh-preserves-revisions",
  content.analyze.includes('buildAnalyzeWhySchedule("continuity", materializedArtifact)')
    && content.analyze.includes("preservedArtifact: materializedArtifact")
    && !content.analyze.includes('"Analyze again"'),
  "Refreshing analysis must continue the same artifact instead of resetting revision 1."
);

const initialBrief = finalArtifact(fixtureModule.buildAnalyzeWhySchedule("continuity"));
const firstRefresh = finalArtifact(fixtureModule.buildAnalyzeWhySchedule("continuity", initialBrief));
const secondRefresh = finalArtifact(fixtureModule.buildAnalyzeWhySchedule("continuity", firstRefresh));
const roadblocks = finalArtifact(fixtureModule.buildAnalyzeWhySchedule("roadblocks", secondRefresh));
const decisions = finalArtifact(fixtureModule.buildAnalyzeWhySchedule("decisions", roadblocks));
const revisitedRoadblocks = finalArtifact(fixtureModule.buildAnalyzeWhySchedule("roadblocks", decisions));
const schedules = [
  fixtureModule.buildAnalyzeWhySchedule("continuity"),
  fixtureModule.buildAnalyzeWhySchedule("continuity", initialBrief),
  fixtureModule.buildAnalyzeWhySchedule("continuity", firstRefresh),
  fixtureModule.buildAnalyzeWhySchedule("roadblocks", secondRefresh),
  fixtureModule.buildAnalyzeWhySchedule("decisions", roadblocks),
  fixtureModule.buildAnalyzeWhySchedule("roadblocks", decisions)
];

check(
  "artifact-region-integrity",
  accepts(() => contractModule.validateIxArtifact(initialBrief))
    && rejects(
      () => contractModule.validateIxArtifact({
        ...initialBrief,
        regions: [initialBrief.regions[0], { ...initialBrief.regions[0], title: "Duplicate identity" }]
      }),
      "region identifiers must be unique"
    )
    && rejects(
      () => contractModule.validateIxArtifact({
        ...initialBrief,
        changedRegionIds: ["missing-from-this-revision"]
      }),
      "must belong to that artifact revision"
    )
    && rejects(
      () => contractModule.validateIxArtifact({
        ...initialBrief,
        changedRegionIds: [initialBrief.regions[0].id, initialBrief.regions[0].id]
      }),
      "changed-region identifiers must be unique"
    ),
  "Every artifact revision must have unique region IDs and changed-region references local to that exact revision."
);

const completedSchedule = schedules[0];
const completedState = runSchedule(completedSchedule);
const partialSchedule = completedSchedule.map((scheduled, index) => index === completedSchedule.length - 1
  ? {
      ...scheduled,
      event: { ...scheduled.event, outcome: "partial", message: "Useful partial work remains available." }
    }
  : scheduled);
const partialState = runSchedule(partialSchedule);
const cancelledSchedule = fixtureModule.buildCancelledReferenceSchedule();
const cancelledState = runSchedule(cancelledSchedule);
const startEvent = completedSchedule[0].event;
const earlyCancelledSchedule = [
  completedSchedule[0],
  {
    afterMs: 0,
    event: withEventIdentity(startEvent, 2, {
      type: "playback_finished",
      outcome: "cancelled",
      message: "Stopped before context resolution completed."
    })
  }
];
const earlyCancelledState = runSchedule(earlyCancelledSchedule);
const earlyErrorSchedule = [
  completedSchedule[0],
  {
    afterMs: 0,
    event: withEventIdentity(startEvent, 2, {
      type: "playback_error_displayed",
      code: "context_unavailable",
      message: "The permitted context could not be resolved."
    })
  }
];
const earlyErrorState = runSchedule(earlyErrorSchedule);

check(
  "executed-terminal-outcomes",
  completedState.status === "completed"
    && completedState.playbackPhase === "terminal"
    && completedState.artifact?.revision === 3
    && partialState.status === "completed"
    && partialState.phaseLabel === "Partial result ready"
    && partialState.artifact?.revision === 3
    && cancelledState.status === "cancelled"
    && cancelledState.playbackPhase === "terminal"
    && cancelledState.artifact?.revision === 1
    && earlyCancelledState.status === "cancelled"
    && earlyCancelledState.artifact === undefined
    && earlyErrorState.status === "failed"
    && earlyErrorState.playbackPhase === "terminal",
  "Completed, partial, cancelled, and early-error gallery playbacks must remain deterministic and fail visible."
);

const contextEvent = completedSchedule.find(({ event: candidate }) => candidate.type === "context_displayed")?.event;
const phaseEvent = completedSchedule.find(({ event: candidate }) => candidate.type === "display_phase_changed")?.event;
const finishedEvent = completedSchedule.at(-1)?.event;
if (!initialBrief || !contextEvent || !phaseEvent || !finishedEvent) {
  throw new Error("The playback proof requires context, display phase, and finish fixture steps.");
}

const maximumRevision = contractModule.IX_MAX_PUBLIC_ARTIFACT_REVISION;
const maximumRunId = "run-browser-exact-maximum";
const maximumIdentity = { ...startEvent, playbackId: maximumRunId };
const maximumPriorArtifact = {
  ...initialBrief,
  revision: maximumRevision - 1
};
let beforeMaximumState = contractModule.beginIxReferencePlaybackState(
  maximumRunId,
  "Continue exact work",
  maximumPriorArtifact
);
beforeMaximumState = contractModule.reduceIxReferencePlaybackEvent(
  beforeMaximumState,
  withEventIdentity(maximumIdentity, 1, {
    type: "playback_started",
    intentLabel: "Continue exact work"
  })
);
beforeMaximumState = contractModule.reduceIxReferencePlaybackEvent(
  beforeMaximumState,
  withEventIdentity(maximumIdentity, 2, {
    type: "context_displayed",
    context: contextEvent.context
  })
);
const exactMaximumEvent = withEventIdentity(maximumIdentity, 3, {
  type: "fixture_revision_displayed",
  artifact: { ...maximumPriorArtifact, revision: maximumRevision }
});
const maximumState = contractModule.reduceIxReferencePlaybackEvent(beforeMaximumState, exactMaximumEvent);
const maximumJson = JSON.stringify(maximumState.artifact);

check(
  "executed-browser-exact-revision-bound",
  maximumState.artifact?.revision === Number.MAX_SAFE_INTEGER
    && maximumJson.includes('"revision":9007199254740991')
    && JSON.parse(maximumJson).revision === Number.MAX_SAFE_INTEGER
    && rejects(
      () => contractModule.beginIxReferencePlaybackState(
        "run-invalid-zero-preserved",
        "Reject zero",
        { ...initialBrief, revision: 0 }
      ),
      "safe positive integer"
    )
    && rejects(
      () => contractModule.reduceIxReferencePlaybackEvent(
        beforeMaximumState,
        { ...exactMaximumEvent, artifact: { ...maximumPriorArtifact, revision: 0 } }
      ),
      "safe positive integer"
    )
    && rejects(
      () => contractModule.reduceIxReferencePlaybackEvent(
        beforeMaximumState,
        { ...exactMaximumEvent, artifact: { ...maximumPriorArtifact, revision: 1.5 } }
      ),
      "safe positive integer"
    )
    && rejects(
      () => contractModule.reduceIxReferencePlaybackEvent(
        beforeMaximumState,
        {
          ...exactMaximumEvent,
          artifact: { ...maximumPriorArtifact, revision: Number.MAX_SAFE_INTEGER + 1 }
        }
      ),
      "safe positive integer"
    )
    && rejects(
      () => contractModule.reduceIxReferencePlaybackEvent(
        maximumState,
        withEventIdentity(maximumIdentity, 4, {
          type: "fixture_revision_displayed",
          artifact: { ...maximumPriorArtifact, revision: maximumRevision }
        })
      ),
      "cannot be continued"
    )
    && rejects(
      () => contractModule.reduceIxReferencePlaybackEvent(
        contractModule.beginIxReferencePlaybackState(maximumRunId, "Reject sequence"),
        withEventIdentity(maximumIdentity, 0, {
          type: "playback_started",
          intentLabel: "Reject sequence"
        })
      ),
      "safe positive integer"
    )
    && rejects(
      () => contractModule.reduceIxReferencePlaybackEvent(
        contractModule.beginIxReferencePlaybackState(maximumRunId, "Reject sequence"),
        withEventIdentity(maximumIdentity, 1.5, {
          type: "playback_started",
          intentLabel: "Reject sequence"
        })
      ),
      "safe positive integer"
    )
    && rejects(
      () => contractModule.reduceIxReferencePlaybackEvent(
        contractModule.beginIxReferencePlaybackState(maximumRunId, "Reject sequence"),
        withEventIdentity(maximumIdentity, Number.MAX_SAFE_INTEGER + 1, {
          type: "playback_started",
          intentLabel: "Reject sequence"
        })
      ),
      "safe positive integer"
    ),
  "The gallery playback reducer must preserve MAX exactly and reject invalid fixture revisions and sequences."
);

check(
  "executed-playback-order-rejections",
  rejects(
    () => runSchedule([
      completedSchedule[0],
      { afterMs: 0, event: withEventIdentity(startEvent, 2, {
        type: "display_phase_changed",
        phase: phaseEvent.phase,
        label: phaseEvent.label,
        detail: phaseEvent.detail
      }) }
    ]),
    "require resolved fixture context"
  )
    && rejects(
      () => runSchedule([
        completedSchedule[0],
        { afterMs: 0, event: withEventIdentity(startEvent, 2, {
          type: "playback_started",
          intentLabel: "Duplicate start"
        }) }
      ]),
      "must be the first fixture step"
    )
    && rejects(
      () => runSchedule([
        completedSchedule[0],
        { afterMs: 0, event: withEventIdentity(startEvent, 2, {
          type: "context_displayed",
          context: contextEvent.context
        }) },
        { afterMs: 0, event: withEventIdentity(startEvent, 3, {
          type: "context_displayed",
          context: contextEvent.context
        }) }
      ]),
      "must follow playback start"
    )
    && rejects(
      () => runSchedule([
        completedSchedule[0],
        { afterMs: 0, event: withEventIdentity(startEvent, 2, {
          type: "context_displayed",
          context: contextEvent.context
        }) },
        { afterMs: 0, event: withEventIdentity(startEvent, 3, {
          type: "display_phase_changed",
          phase: phaseEvent.phase,
          label: phaseEvent.label,
          detail: phaseEvent.detail
        }) },
        { afterMs: 0, event: withEventIdentity(startEvent, 4, {
          type: "playback_finished",
          outcome: "completed",
          message: finishedEvent.message
        }) }
      ]),
      "requires a useful fixture revision"
    ),
  "The gallery playback reducer must reject display-before-context, duplicate starts, and completed playback without a useful fixture revision."
);

const postTerminalPhase = withEventIdentity(startEvent, completedState.sequence + 1, {
  type: "display_phase_changed",
  phase: phaseEvent.phase,
  label: phaseEvent.label,
  detail: phaseEvent.detail
});
const postCancelledPhase = withEventIdentity(startEvent, cancelledState.sequence + 1, {
  type: "display_phase_changed",
  phase: phaseEvent.phase,
  label: phaseEvent.label,
  detail: phaseEvent.detail
});
const postErrorPhase = withEventIdentity(startEvent, earlyErrorState.sequence + 1, {
  type: "display_phase_changed",
  phase: phaseEvent.phase,
  label: phaseEvent.label,
  detail: phaseEvent.detail
});
check(
  "executed-post-terminal-fence",
  rejects(
    () => contractModule.reduceIxReferencePlaybackEvent(completedState, postTerminalPhase),
    "after the fixture stopped"
  )
    && rejects(
      () => contractModule.reduceIxReferencePlaybackEvent(cancelledState, postCancelledPhase),
      "after the fixture stopped"
    )
    && rejects(
      () => contractModule.reduceIxReferencePlaybackEvent(earlyErrorState, postErrorPhase),
      "after the fixture stopped"
    ),
  "Completed, cancelled, and failed gallery playbacks must reject every later fixture step."
);

const recoveryContext = fixtureModule.RECOVERY_CONTEXT;
const unknownSourceIndex = recoveryContext.sources.findIndex(({ freshness }) => freshness === "unknown");
const falseKnownContext = {
  ...recoveryContext,
  sources: recoveryContext.sources.map((source, index) => index === unknownSourceIndex
    ? { ...source, freshness: "current" }
    : source)
};
const sourceTemplate = recoveryContext.sources[0];
const overBoundSources = Array.from({ length: 33 }, (_, index) => ({
  ...sourceTemplate,
  sourceRef: `bounded-source-${index}`
}));
const duplicateSources = [sourceTemplate, { ...sourceTemplate }];
const futureRefreshSources = recoveryContext.sources.map((source, index) => index === 0
  ? { ...source, refreshedAt: "2026-08-10T09:20:00-07:00" }
  : source);
const subMillisecondFutureContext = {
  ...recoveryContext,
  sourceCount: 1,
  evaluatedAt: "2026-08-09T09:20:00.000000001Z",
  freshness: "mixed",
  sources: [{
    ...recoveryContext.sources[0],
    refreshedAt: "2026-08-09T09:20:00.000000002Z"
  }],
  gaps: []
};
const unknownSourceContext = {
  ...recoveryContext,
  sourceCount: 1,
  freshness: "mixed",
  sources: [recoveryContext.sources[unknownSourceIndex]],
  gaps: []
};

check(
  "executed-provenance-validation",
  accepts(() => contractModule.validateIxResolvedContext(recoveryContext))
    && unknownSourceIndex >= 0
    && recoveryContext.sources[unknownSourceIndex].refreshedAt === undefined
    && accepts(() => contractModule.validateIxResolvedContext({
      ...unknownSourceContext,
      evaluatedAt: "2026-08-09t09:20:00z"
    }))
    && accepts(() => contractModule.validateIxResolvedContext({
      ...unknownSourceContext,
      evaluatedAt: "2026-08-09 09:20:60Z"
    }))
    && accepts(() => contractModule.validateIxResolvedContext({
      ...unknownSourceContext,
      evaluatedAt: "2026-08-09T09:20:00−07:00"
    }))
    && rejects(
      () => contractModule.validateIxResolvedContext(falseKnownContext),
      "must use unknown freshness"
    )
    && rejects(
      () => contractModule.validateIxResolvedContext({
        ...recoveryContext,
        sourceCount: overBoundSources.length,
        sources: overBoundSources
      }),
      "too many source summaries"
    )
    && rejects(
      () => contractModule.validateIxResolvedContext({ ...recoveryContext, sourceCount: 3 }),
      "does not match"
    )
    && rejects(
      () => contractModule.validateIxResolvedContext({
        ...recoveryContext,
        sourceCount: duplicateSources.length,
        sources: duplicateSources
      }),
      "must be unique"
    )
    && rejects(
      () => contractModule.validateIxResolvedContext({
        ...recoveryContext,
        sources: futureRefreshSources
      }),
      "cannot follow context evaluation"
    )
    && rejects(
      () => contractModule.validateIxResolvedContext(subMillisecondFutureContext),
      "cannot follow context evaluation"
    )
    && rejects(
      () => contractModule.validateIxResolvedContext({
        ...recoveryContext,
        sourceCount: 0,
        sources: [],
        freshness: "current"
      }),
      "must use unknown freshness"
    )
    && rejects(
      () => contractModule.validateIxResolvedContext({
        ...recoveryContext,
        freshness: "current"
      }),
      "requires current source summaries"
    )
    && rejects(
      () => contractModule.validateIxResolvedContext({
        ...recoveryContext,
        evaluatedAt: "2026-08-09"
      }),
      "RFC 3339"
    )
    && rejects(
      () => contractModule.validateIxResolvedContext({
        ...recoveryContext,
        sources: recoveryContext.sources.map((source, index) => index === 0
          ? { ...source, sourceRef: "not a safe ref" }
          : source)
      }),
      "unsupported characters"
    )
    && rejects(
      () => contractModule.validateIxResolvedContext({
        ...recoveryContext,
        focusLabel: "   "
      }),
      "bounded plain text"
    )
    && rejects(
      () => contractModule.validateIxResolvedContext({
        ...recoveryContext,
        sources: recoveryContext.sources.map((source, index) => index === 0
          ? { ...source, label: "é".repeat(257) }
          : source)
      }),
      "bounded plain text"
    )
    && rejects(
      () => contractModule.validateIxResolvedContext({
        ...recoveryContext,
        gaps: [" "]
      }),
      "bounded plain text"
    ),
  "Source summaries must execute the catalog fixture text, key, RFC 3339, count, freshness, and aggregate rules."
);

check(
  "repeated-focus-reconciles-stable-regions",
  initialBrief?.revision === 3
    && firstRefresh?.revision === 5
    && secondRefresh?.revision === 7
    && firstRefresh.regions.length === 5
    && secondRefresh.regions.length === 5
    && revisitedRoadblocks.regions.length === decisions.regions.length
    && [initialBrief, firstRefresh, secondRefresh, roadblocks, decisions, revisitedRoadblocks]
      .every(hasUniqueRegionIds),
  "Refreshing or revisiting a focus must replace stable region identities instead of accumulating duplicate sections."
);

const playbackIds = schedules.map((schedule) => schedule[0].event.playbackId);
check(
  "repeated-controls-advance-run-identity",
  new Set(playbackIds).size === playbackIds.length,
  "Every refresh or redirected analysis must receive a distinct playback identity."
);

const firstArtifactIndex = schedules[0].findIndex(({ event }) => event.type === "fixture_revision_displayed");
const stopResumeProof = runScheduleWithPlaybackPause(schedules[0], undefined, firstArtifactIndex);
check(
  "fixture-pause-resume-preserves-partial-artifact",
  stopResumeProof.pausedState?.status === "running"
    && stopResumeProof.pausedState.fixturePlaybackPaused === true
    && stopResumeProof.pausedState.artifact?.revision === 1
    && stopResumeProof.pausedState.artifact.changedRegionIds.length === 0
    && stopResumeProof.state.status === "completed"
    && stopResumeProof.state.artifact?.revision === 3
    && contractModule.resumeIxReferencePlaybackState(cancelledState).status === "cancelled",
  "Pausing deterministic gallery playback must retain partial work, resume remaining fixture steps, and keep an explicit stop final."
);

const editedBody = "Human edit retained across focus changes.";
const humanEditedBrief = {
  ...initialBrief,
  regions: initialBrief.regions.map((region) => region.id === "changed"
    ? { ...region, body: editedBody }
    : region)
};
const redirectedEditedBrief = finalArtifact(
  fixtureModule.buildAnalyzeWhySchedule("roadblocks", humanEditedBrief)
);
check(
  "redirect-preserves-human-edits",
  redirectedEditedBrief.regions.find(({ id }) => id === "changed")?.body === editedBody,
  "Redirected analysis must preserve human-edited regions it did not revise."
);

const preservedRunStart = contractModule.beginIxReferencePlaybackState(
  "preserved-run-motion-proof",
  "Refresh analysis",
  firstRefresh
);
check(
  "preserved-regions-are-static-until-revised",
  preservedRunStart.artifact?.changedRegionIds.length === 0,
  "New playback must not animate prior changed regions before a new fixture revision arrives."
);

check(
  "shared-artifact-loop",
  content.app.includes("workingBrief")
    && content.conversation.includes("Add finding to working brief")
    && content.analyze.includes("Ask a follow-up"),
  "Analyze Why and Contextual Conversation must share one continuing artifact loop."
);

check(
  "shared-artifact-does-not-echo",
  content.analyze.includes("if (!run.state.artifact) return")
    && content.analyze.includes("applyHumanEdits(run.state.artifact, editedBodies)"),
  "A retained parent artifact may be displayed, but only a new fixture revision or human edit may write it back."
);

check(
  "fixture-run-identity-advances",
  content.conversation.includes("runOrdinal.current += 1")
    && content.fixtures.includes("conversation-${normalizedId}-${runOrdinal}"),
  "Repeated contextual questions must not silently reuse one playback identity."
);

const specializedSchedules = [
  ["adaptive-composition", fixtureModule.buildAdaptiveCompositionSchedule("operating-picture")],
  ["adaptive-composition", fixtureModule.buildAdaptiveCompositionSchedule("decisions", ["readiness-signal"])],
  ["working-goal-plan", fixtureModule.buildWorkingGoalPlanSchedule(false)],
  ["working-goal-plan", fixtureModule.buildWorkingGoalPlanSchedule(true)],
  ["adaptive-information-lens", fixtureModule.buildAdaptiveInformationLensSchedule("executive-read")],
  ["adaptive-information-lens", fixtureModule.buildAdaptiveInformationLensSchedule("operator-read")],
  ["situation-to-strategy", fixtureModule.buildSituationToStrategySchedule()],
  ["situation-to-strategy", fixtureModule.buildSituationToStrategySchedule({ category: "source-freshness", text: "What if the source is stale?" })],
  ["attention-stewardship", fixtureModule.buildAttentionStewardshipSchedule(false)],
  ["attention-stewardship", fixtureModule.buildAttentionStewardshipSchedule(true)],
  ["ambient-agent-continuity", fixtureModule.buildAmbientAgentContinuitySchedule()]
];
const specializedStates = specializedSchedules.map(([expectedKind, schedule]) => ({
  expectedKind,
  schedule,
  state: runSchedule(schedule)
}));
check(
  "executed-six-purpose-shaped-recipes",
  specializedStates.every(({ expectedKind, schedule, state }) =>
    state.status === "completed"
    && state.playbackPhase === "terminal"
    && state.artifact?.revision === 2
    && state.artifact?.referenceRecipe?.kind === expectedKind
    && schedule.findIndex(({ event }) => event.type === "fixture_revision_displayed") < schedule.length - 1
  ),
  "Every new recipe and its steering variants must execute meaningful partial and complete revisions."
);

const compositionState = specializedStates[1].state.artifact?.referenceRecipe;
const goalState = specializedStates[3].state.artifact?.referenceRecipe;
const lensState = specializedStates[4].state.artifact?.referenceRecipe;
const strategyState = specializedStates[7].state.artifact?.referenceRecipe;
const calmState = specializedStates[9].state.artifact?.referenceRecipe;
const ambientState = specializedStates[10].state.artifact?.referenceRecipe;
check(
  "distinctive-information-shapes",
  compositionState?.kind === "adaptive-composition"
    && compositionState.blocks[0]?.id === "readiness-signal"
    && compositionState.changeSummary.some((change) => change.includes("selected items"))
    && goalState?.kind === "working-goal-plan"
    && goalState.preservedWork.includes("Owner confirmation remains in progress")
    && !goalState.steps.some(({ state }) => state === "complete")
    && goalState.steps.some(({ state }) => state === "required")
    && lensState?.kind === "adaptive-information-lens"
    && lensState.fullRecordFields.length === 18
    && lensState.fullRecordFields.length > lensState.visibleFields.length
    && strategyState?.kind === "situation-to-strategy"
    && strategyState.counterview
    && strategyState.priorThesis
    && strategyState.revisionChange
    && strategyState.assumptions.length >= 2
    && calmState?.kind === "attention-stewardship"
    && calmState.items.length === 0
    && calmState.emptyMessage?.includes("remains unavailable")
    && ambientState?.kind === "ambient-agent-continuity"
    && ambientState.capabilityName === "Recovery assurance"
    && new Set(ambientState.workstreams.map(({ state }) => state)).size >= 5,
  "Composition, plan, lens, strategy, calm attention, and ambient continuity must retain their distinctive data shapes."
);

check(
  "interactive-steering-and-correction",
  content.composition.includes("Keep in view")
    && content.composition.includes("Recompose with kept items")
    && content.goalPlan.includes("Replan around external handoff")
    && content.goalPlan.includes("Your note")
    && content.goalPlan.includes("Notes retained from the earlier path")
    && content.informationLens.includes("Open full record")
    && content.informationLens.includes("Keep in view")
    && content.strategy.includes("Test this challenge")
    && content.strategy.includes("Probe this assumption")
    && content.attention.includes("Undo last correction")
    && content.attention.includes("Challenge priority")
    && content.attention.includes("Corrected or deferred")
    && content.agents.includes("Preview tuning")
    && content.agents.includes("Apply as revision")
    && content.agents.includes("Cancel preview")
    && content.agents.includes("Rollback tuning")
    && content.agents.includes("30-day deterministic preview")
    && content.agents.includes("Authority is not tunable"),
  "Each reference must expose the correction, steering, or reversible tuning its theory requires."
);

check(
  "ambient-agent-truth-boundary",
  content.agents.includes("no live provider, scheduler, durable worker, background execution")
    && content.fixtures.includes("It cannot approve, fund, assign, or change source systems")
    && content.agents.includes("Observed since you were away")
    && content.agents.includes("Completed during this reveal")
    && content.agents.includes("One accountable capability")
    && ["watching", "working", "completed", "needs-input", "paused", "stopped"]
      .every((state) => content.contract.includes(`\"${state}\"`)),
  "Ambient agents must show useful continuity and every playbackPhase state without claiming live execution or tunable authority."
);

const initialGoalArtifact = finalArtifact(fixtureModule.buildWorkingGoalPlanSchedule(false));
const replannedGoalSchedule = fixtureModule.buildWorkingGoalPlanSchedule(true, 3);
const replannedGoalState = runSchedule(replannedGoalSchedule, initialGoalArtifact);
const replannedGoal = replannedGoalState.artifact?.referenceRecipe;
check(
  "stateful-goal-replan-honesty",
  replannedGoalState.artifact?.revision === 4
    && replannedGoal?.kind === "working-goal-plan"
    && replannedGoal.goal === initialGoalArtifact.referenceRecipe.goal
    && replannedGoal.steps.find(({ id }) => id === "confirm-owner")?.state === "working"
    && !replannedGoal.steps.some(({ state }) => state === "complete")
    && replannedGoal.preservedWork.includes("Owner confirmation remains in progress")
    && replannedGoalSchedule
      .filter(({ event }) => event.type === "fixture_revision_displayed")
      .every(({ event }) => !event.artifact.changedRegionIds.includes("confirm-owner")),
  "A replan must continue exact revisions, change future work only, and never invent completed ownership."
);

const operatorLens = finalArtifact(fixtureModule.buildAdaptiveInformationLensSchedule("operator-read"));
const keptLensSchedule = fixtureModule.buildAdaptiveInformationLensSchedule("executive-read", 3, ["handoff"]);
const keptLensState = runSchedule(keptLensSchedule, operatorLens);
const keptLens = keptLensState.artifact?.referenceRecipe;
check(
  "stateful-information-lens-keeps-field",
  keptLens?.kind === "adaptive-information-lens"
    && keptLens.fullRecordFields.length === 18
    && keptLens.visibleFields[0]?.id === "handoff"
    && keptLens.visibleFields.some(({ id }) => id === "handoff")
    && content.informationLens.includes("Open full record · {recipe.fullRecordFields.length} fields"),
  "A user-kept field must survive a task change while the full 18-field record remains one gesture away."
);

const initialStrategy = finalArtifact(fixtureModule.buildSituationToStrategySchedule());
const challengedStrategySchedule = fixtureModule.buildSituationToStrategySchedule(
  { category: "capacity", text: "Compare available recovery capacity" },
  3,
  { thesis: initialStrategy.referenceRecipe.thesis, reasoning: "Initial ownership thesis" }
);
const challengedStrategyState = runSchedule(challengedStrategySchedule, initialStrategy);
const challengedStrategy = challengedStrategyState.artifact?.referenceRecipe;
check(
  "stateful-strategy-challenge-preserves-prior",
  challengedStrategy?.kind === "situation-to-strategy"
    && challengedStrategy.priorThesis?.thesis === initialStrategy.referenceRecipe.thesis
    && challengedStrategy.challenge?.text === "Compare available recovery capacity"
    && challengedStrategy.revisionChange?.includes("capacity")
    && challengedStrategy.counterview?.includes("capacity")
    && challengedStrategySchedule
      .filter(({ event }) => event.type === "fixture_revision_displayed")
      .every(({ event }) => !event.artifact.changedRegionIds.includes("intervention")),
  "A challenge must preserve the prior thesis and identify only the thesis elements it actually changes."
);

const secondChallengeSchedule = fixtureModule.buildSituationToStrategySchedule(
  { category: "causal-link", text: "What evidence proves ownership caused the decline?" },
  5,
  { thesis: challengedStrategy.thesis, reasoning: challengedStrategy.revisionChange }
);
const secondChallengeState = runSchedule(secondChallengeSchedule, challengedStrategyState.artifact);
const secondChallenge = secondChallengeState.artifact?.referenceRecipe;
check(
  "stateful-consecutive-strategy-challenges",
  secondChallengeState.artifact?.revision === 6
    && secondChallenge?.kind === "situation-to-strategy"
    && secondChallenge.priorThesis?.thesis === challengedStrategy.thesis
    && secondChallenge.priorThesis?.thesis !== initialStrategy.referenceRecipe.thesis
    && secondChallenge.challenge?.category === "causal-link"
    && secondChallenge.challenge?.text === "What evidence proves ownership caused the decline?"
    && secondChallenge.revisionChange?.includes("causal claim"),
  "Bounded challenge categories and free text must survive consecutive exact-revision challenges without resetting the thesis."
);

const calmSchedule = fixtureModule.buildAttentionStewardshipSchedule(true);
const calmRun = runSchedule(calmSchedule);
const calmContext = calmRun.context;
const calmRecipe = calmRun.artifact?.referenceRecipe;
check(
  "stateful-calm-context-is-not-all-clear",
  calmContext?.gaps.length > 0
    && calmRecipe?.kind === "attention-stewardship"
    && calmRecipe.items.length === 0
    && calmRecipe.emptyMessage.includes("remains unavailable")
    && calmContext.focusLabel === "Recovery advisory view · current quarter"
    && calmContext.focusLabel !== fixtureModule.RECOVERY_CONTEXT.focusLabel
    && calmContext.sources.length === 2
    && !calmRecipe.emptyMessage.toLowerCase().includes("nothing needs")
    && !content.attention.includes("You are caught up"),
  "An empty owned-attention set must use its distinct authorized context and keep unresolved detail visible instead of claiming all clear."
);

const completedWorkstream = ambientState.workstreams.find(({ state }) => state === "completed");
const needsInputWorkstream = ambientState.workstreams.find(({ state }) => state === "needs-input");
const watchingWorkstream = ambientState.workstreams.find(({ state }) => state === "watching");
const pausedWorkstream = ambientState.workstreams.find(({ state }) => state === "paused");
const pausedFromWatching = fixtureModule.transitionAmbientWorkstream(watchingWorkstream, "pause");
const resumedWatching = fixtureModule.transitionAmbientWorkstream(pausedFromWatching, "resume");
check(
  "stateful-ambient-control-fence",
  completedWorkstream
    && needsInputWorkstream
    && watchingWorkstream
    && pausedWorkstream
    && fixtureModule.transitionAmbientWorkstream(completedWorkstream, "pause").state === "completed"
    && fixtureModule.transitionAmbientWorkstream(needsInputWorkstream, "stop").state === "needs-input"
    && pausedFromWatching.state === "paused"
    && pausedFromWatching.resumeState === "watching"
    && resumedWatching.state === "watching"
    && fixtureModule.transitionAmbientWorkstream(pausedWorkstream, "resume").state === "working"
    && fixtureModule.transitionAmbientWorkstream(watchingWorkstream, "stop").state === "stopped",
  "Completed and needs-input workstreams must be fenced; pause/resume must restore the exact prior state."
);

const defaultTuningPreview = fixtureModule.previewAmbientTuning({ relevance: "material", cadence: "daily", quiet: true }, ambientState.tuningCorpus);
const expandedTuningPreview = fixtureModule.previewAmbientTuning({ relevance: "operational", cadence: "weekly", quiet: false }, ambientState.tuningCorpus);
check(
  "executed-ambient-tuning-corpus",
  ambientState.tuningCorpus.events.length === 11
    && ambientState.tuningCorpus.windowStart === "2026-07-11T00:00:00-07:00"
    && ambientState.tuningCorpus.windowEnd === "2026-08-09T23:59:59-07:00"
    && ambientState.tuningCorpus.timezone === "America/Los_Angeles"
    && ambientState.tuningCorpus.formula.includes("Filter by relevance")
    && defaultTuningPreview.eventCount === 11
    && defaultTuningPreview.qualified < expandedTuningPreview.qualified
    && defaultTuningPreview.deferred > expandedTuningPreview.deferred
    && content.agents.includes("Inspect the 11-event fixture corpus and formula")
    && content.agents.includes("preview.windowStart")
    && content.agents.includes("preview.timezone"),
  "Tuning preview must execute against an inspectable 11-event corpus with formula, window, and timezone."
);

check(
  "working-goal-note-handler-captures-value",
  content.goalPlan.includes("const note = event.currentTarget.value;")
    && content.goalPlan.includes("setHumanNotes((current) =>")
    && !content.goalPlan.includes("note: event.currentTarget.value"),
  "The React input value must be copied before entering the functional state updater."
);

const authorizedSources = new Map(fixtureModule.RECOVERY_CONTEXT.sources.map((source) => [source.sourceRef, source]));
const traceIsInspectable = (trace) => trace.sources.length > 0
  && trace.sources.every((source) => {
    if (source.kind === "authorized-context") {
      const resolved = authorizedSources.get(source.sourceRef);
      return resolved
        && source.label === resolved.label
        && source.freshness === resolved.freshness
        && source.refreshedAt === resolved.refreshedAt;
    }
    return source.kind === "deterministic-fixture"
      && source.sourceRef.startsWith("fixture:")
      && source.freshness === "unknown"
      && typeof source.observedAt === "string"
      && source.observedAt.length > 0;
  })
  && trace.derivation;
check(
  "claim-level-trace-on-six-recipes",
  compositionState.blocks.every(({ trace }) => traceIsInspectable(trace))
    && goalState.steps.every(({ trace }) => traceIsInspectable(trace))
    && lensState.fullRecordFields.every(({ trace }) => traceIsInspectable(trace))
    && traceIsInspectable(strategyState.thesisTrace)
    && strategyState.interventionTrace.derivation
    && strategyState.assumptions.every(({ trace }) => traceIsInspectable(trace))
    && specializedStates[8].state.artifact.referenceRecipe.items.every(({ trace }) => traceIsInspectable(trace))
    && ambientState.sinceAway.every(({ trace }) => traceIsInspectable(trace))
    && ambientState.workstreams.every(({ trace }) => traceIsInspectable(trace))
    && [content.composition, content.goalPlan, content.informationLens, content.strategy, content.attention, content.agents]
      .every((source) => source.includes("<ClaimTrace"))
    && content.components.includes("Inspect source and derivation")
    && content.components.includes("#ix-source-${source.sourceRef}")
    && content.components.includes("claimSourceFreshness"),
  "Every purpose-shaped recipe must carry and render claim-level source, freshness, and derivation."
);

const strategyThesisSources = strategyState.thesisTrace.sources;
const interventionSources = strategyState.interventionTrace.sources;
check(
  "compound-claim-preserves-per-source-freshness",
  strategyThesisSources.length === 2
    && strategyThesisSources[0].sourceRef === "recovery-exercise-register"
    && strategyThesisSources[0].refreshedAt === "2026-08-09T09:12:00-07:00"
    && strategyThesisSources[1].sourceRef === "recovery-ownership-roster"
    && strategyThesisSources[1].refreshedAt === "2026-08-08T16:30:00-07:00"
    && interventionSources.length === 2
    && interventionSources[1].sourceRef === "external-handoff-status"
    && interventionSources[1].freshness === "unknown"
    && interventionSources[1].refreshedAt === undefined
    && !content.contract.includes("refreshed: string")
    && !content.components.includes("trace.refreshed")
    && content.components.includes("trace.sources.map((source)"),
  "Compound claims must retain each source's own ledger freshness and timestamp instead of collapsing them into one ambiguous value."
);

const compositionDecisionSources = compositionState.blocks
  .find(({ id }) => id === "decision-move")
  ?.trace.sources;
check(
  "composition-decision-resolves-authorized-source-freshness",
  compositionDecisionSources?.length === 2
    && compositionDecisionSources[0].sourceRef === "recovery-ownership-roster"
    && compositionDecisionSources[0].kind === "authorized-context"
    && compositionDecisionSources[0].freshness === "current"
    && compositionDecisionSources[0].refreshedAt === "2026-08-08T16:30:00-07:00"
    && compositionDecisionSources[1].sourceRef === "continuity-scorecard"
    && compositionDecisionSources[1].kind === "authorized-context"
    && compositionDecisionSources[1].freshness === "stale"
    && compositionDecisionSources[1].refreshedAt === "2026-07-17T17:00:00-07:00",
  "Adaptive Composition's decision move must resolve roster and scorecard shorthand to their exact authorized ledger metadata."
);

check(
  "full-navigation-labels-and-synthetic-exception",
  content.css.includes("grid-template-columns: repeat(4, minmax(0, 1fr))")
    && content.css.includes("white-space: normal")
    && content.catalogApp.includes("Synthetic reference exception")
    && content.catalogApp.includes("not reusable product copy")
    && content.spec.includes("### Synthetic Reference Exception")
    && content.spec.includes("does not validate, simulate, or claim an")
    && content.spec.includes("application intelligence runtime"),
  "Desktop and mobile navigation must expose full labels, and fixture nouns must carry a narrow, explicit doctrine exception."
);

check(
  "human-revision-announcements",
  content.components.includes("revisionNote")
    && content.components.includes("changeSummary")
    && content.components.includes("singleTerminalPunctuation")
    && content.components.includes("replace(/[.!?]+$/u")
    && !content.components.includes("changedRegionIds.join"),
  "Live announcements must use human labels and change summaries, with exactly one terminal punctuation mark, instead of internal artifact identifiers."
);

check(
  "focus-preserving-state-transitions",
  content.agents.includes("pendingFocusRef")
    && content.agents.includes("ambient-workstream-control-${workstream.id}")
    && content.agents.includes("Tuning preview cancelled. No settings changed.")
    && content.attention.includes("attention-restore-${item.id}")
    && content.attention.includes("Restore to active attention")
    && content.attention.includes('role="status" aria-live="polite" aria-atomic="true"'),
  "Ambient controls, tuning transitions, and Attention corrections must retain keyboard focus and announce the new state plainly."
);

check(
  "inspectable-claims",
  content.presentation.includes("model.summary")
    && content.components.includes('summary: "Where did this come from?"')
    && content.components.includes("How calculated")
    && content.components.includes("Challenge this"),
  "Source, freshness, calculation, limits, and challenge must be reachable at the claim."
);

check(
  "unknown-context-freshness-is-visible",
  content.components.includes("Refresh time unknown")
    && content.components.includes("contextFreshnessValue")
    && content.components.includes("summary: `View ${resolved.sourceCount} sources`"),
  "Unknown refresh time and the bounded source ledger must remain visible rather than being coerced to stale."
);

check(
  "meaningful-liveness",
  content.fixtures.includes("Finding what changed")
    && content.fixtures.includes("Checking the number and the weak assumption")
    && !content.fixtures.includes("Thinking…"),
  "Visible phases name actual work rather than generic model activity."
);

check(
  "motion-is-bounded",
  content.components.includes("model={{")
    && content.components.includes("active,")
    && content.analyze.includes('active={run.state.status === "running"}')
    && content.conversation.includes('active={run.state.status === "running"}')
    && content.pdsCss.includes('.pds-progressive-response[data-active="true"]')
    && content.pdsCss.includes("prefers-reduced-motion: reduce")
    && content.pdsCss.includes("animation: pds-work-label-shimmer"),
  "Shimmer must be gated by active gallery playback and have a reduced-motion equivalent."
);

check(
  "artifact-revision-announced",
  content.presentation.includes('className="pds-assistive-announcement" role="status" aria-live="polite" aria-atomic="true"')
    && content.components.includes("artifact.title} revision ${artifact.revision}"),
  "Each usable artifact revision must have a concise assistive-technology announcement."
);

check(
  "react-free-serializable-presentation-model",
  content.presentationModel.includes('from "@appfw/pds-ix-presentation-contract"')
    && content.presentationContractTypes.includes("export type ProgressiveResponsePresentationModel")
    && content.presentationContractTypes.includes("readonly ProgressiveResponseRegionModel[]")
    && !/from ["']react|ReactNode|HTMLElement|onClick/.test(content.presentationContractTypes)
    && !/from ["']react|document|window|onClick/.test(content.presentationContractRuntime),
  "The Web adapter must consume the standalone channel-neutral contract, whose runtime and types stay plain serializable data with no React, DOM, or callback values."
);

check(
  "mobile-freshness-remains-visible",
  content.pdsCss.includes(".pds-resolved-context__meta .pds-freshness-indicator")
    && content.pdsCss.includes("width: 100%")
    && !content.pdsCss.includes(".pds-resolved-context__meta .pds-freshness-indicator {\n    display: none"),
  "Narrow layouts must reflow source freshness instead of hiding it."
);

const implementationText = [
  content.app,
  content.contract,
  content.fixtures,
  content.run,
  content.analyze,
  content.conversation,
  content.composition,
  content.goalPlan,
  content.informationLens,
  content.strategy,
  content.attention,
  content.agents,
  content.components,
  content.presentation,
  content.presentationModel
].join("\n");
for (const unsafe of ["dangerouslySetInnerHTML", ".innerHTML", "eval(", "GEMINI_API_KEY", "AWS_SECRET_ACCESS_KEY"]) {
  check(
    `unsafe-pattern-absent:${unsafe}`,
    !implementationText.includes(unsafe),
    "The reference must not contain executable model UI, secret handling, or unsafe HTML insertion."
  );
}

const browserBaseUrl = process.argv
  .find((argument) => argument.startsWith("--browser-base-url="))
  ?.slice("--browser-base-url=".length)
  .replace(/\/$/, "");
let browserProof = "not_run";
let browserProofDetail = "Physical browser, mobile, reduced-motion, source-link, and integrated-shell proof was not run. Pass --browser-base-url=<url> to execute it.";
if (browserBaseUrl) {
  let browser;
  let noteBrowserOk = false;
  let focusBrowserOk = false;
  let punctuationBrowserOk = false;
  let sourceLinkBrowserOk = false;
  let mobileBrowserOk = false;
  let reducedMotionBrowserOk = false;
  let shellEscapeBrowserOk = false;
  let browserDetail = "The browser regression did not complete.";
  try {
    const { chromium } = await import("@playwright/test");
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(error.message));
    await page.goto(`${browserBaseUrl}/ix-reference.html`, { waitUntil: "networkidle" });
    await page.getByRole("button", { name: /Working Goal Plan/ }).click();
    await page.getByRole("button", { name: "Build a working plan" }).click();
    const replan = page.getByRole("button", { name: "Replan around external handoff" });
    await replan.waitFor({ state: "visible" });
    const note = page.getByLabel("Your note").nth(2);
    await note.fill("Retain this dependency note through replanning.");
    const valueAfterInput = await note.inputValue();
    const galleryMounted = await page.locator("main.ix-gallery").isVisible();
    const replanStillPresent = await replan.isVisible();
    await replan.click();
    await page.getByText("Notes retained from the earlier path").waitFor({ state: "visible" });
    const retainedNoteVisible = await page.getByText("Retain this dependency note through replanning.").isVisible();
    noteBrowserOk = valueAfterInput === "Retain this dependency note through replanning."
      && galleryMounted
      && replanStillPresent
      && retainedNoteVisible;

    const revisionAnnouncements = await page
      .locator('p.ix-sr-only[role="status"]')
      .filter({ hasText: / revision / })
      .allTextContents();
    punctuationBrowserOk = revisionAnnouncements.length > 0
      && revisionAnnouncements.every((announcement) => {
        const normalized = announcement.trim();
        return /[.!?]$/u.test(normalized) && !/[.!?]{2,}/u.test(normalized);
      });

    await page.getByRole("button", { name: /Agents Helping You/ }).click();
    await page.getByRole("button", { name: "Review capability updates" }).click();
    await page.getByRole("heading", { name: "Completed during this reveal" }).waitFor({ state: "visible" });
    const pauseWorkstream = page.getByRole("button", { name: "Pause", exact: true }).first();
    await pauseWorkstream.click();
    await page.waitForFunction(() => document.activeElement instanceof HTMLButtonElement
      && document.activeElement.textContent?.trim().startsWith("Resume"));
    const ambientFocus = await page.evaluate(() => ({
      tag: document.activeElement?.tagName,
      label: document.activeElement?.textContent?.trim()
    }));

    await page.getByRole("button", { name: /Attention Stewardship/ }).click();
    await page.getByRole("button", { name: "What needs my attention?" }).click();
    const correctAttention = page.getByRole("button", { name: "Already covered" }).first();
    await correctAttention.waitFor({ state: "visible" });
    await correctAttention.click();
    await page.waitForFunction(() => document.activeElement instanceof HTMLButtonElement
      && document.activeElement.textContent?.trim() === "Restore to active attention");
    const attentionFocus = await page.evaluate(() => ({
      tag: document.activeElement?.tagName,
      label: document.activeElement?.textContent?.trim()
    }));
    const correctedActions = await page.locator(".ix-corrected-attention article").first().getByRole("button").count();
    focusBrowserOk = ambientFocus.tag === "BUTTON"
      && ambientFocus.label?.startsWith("Resume") === true
      && attentionFocus.tag === "BUTTON"
      && attentionFocus.label === "Restore to active attention"
      && correctedActions === 1;

    const claimDisclosure = page.locator(".ix-corrected-attention details.ix-claim-trace").first();
    await claimDisclosure.locator("summary").click();
    const authorizedSourceLink = claimDisclosure.locator('a[href^="#ix-source-"]').first();
    const sourceTargetSelector = await authorizedSourceLink.getAttribute("href");
    if (sourceTargetSelector) {
      await authorizedSourceLink.click();
      const sourceTarget = page.locator(sourceTargetSelector);
      sourceLinkBrowserOk = await sourceTarget.isVisible()
        && await sourceTarget.evaluate((target) => target.closest("details")?.open === true)
        && new URL(page.url()).hash === sourceTargetSelector;
    }

    await page.setViewportSize({ width: 390, height: 844 });
    const mobileState = await page.evaluate(() => {
      const documentElement = document.documentElement;
      const body = document.body;
      const nav = document.querySelector(".ix-reference-nav");
      const gallery = document.querySelector("main.ix-gallery");
      const contextMeta = document.querySelector(".pds-resolved-context__meta");
      const labels = [...document.querySelectorAll(".ix-reference-nav__item strong")];
      const contextRect = contextMeta?.getBoundingClientRect();
      const galleryRect = gallery?.getBoundingClientRect();
      return {
        labels: labels.map((label) => label.textContent?.trim()),
        labelsUnclipped: labels.every((label) => label.scrollWidth <= label.clientWidth + 1),
        documentOverflow: Math.max(documentElement.scrollWidth, body.scrollWidth) - documentElement.clientWidth,
        navScroller: Boolean(nav
          && nav.scrollWidth > nav.clientWidth + 1
          && ["auto", "scroll"].includes(getComputedStyle(nav).overflowX)),
        freshnessVisible: Boolean(contextMeta
          && contextRect
          && contextRect.width > 0
          && contextRect.height > 0
          && getComputedStyle(contextMeta).visibility !== "hidden"
          && contextMeta.textContent?.includes("resolved sources")),
        galleryFits: Boolean(galleryRect
          && galleryRect.left >= -1
          && galleryRect.right <= documentElement.clientWidth + 1)
      };
    });
    mobileBrowserOk = mobileState.labels.join("|") === eightReferences.join("|")
      && mobileState.labelsUnclipped
      && mobileState.documentOverflow <= 1
      && mobileState.navScroller
      && mobileState.freshnessVisible
      && mobileState.galleryFits;

    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.getByRole("button", { name: "What needs my attention?" }).click();
    const activeMotion = page.locator(
      '.pds-work-status[data-active="true"] .pds-work-status__signal, '
        + '.pds-work-status[data-active="true"] .pds-work-status__copy strong'
    );
    await activeMotion.first().waitFor({ state: "visible" });
    const motionStyles = await activeMotion.evaluateAll((elements) => elements.map((element) => {
      const style = getComputedStyle(element);
      const seconds = style.animationDuration.split(",").map((duration) => duration.trim().endsWith("ms")
        ? Number.parseFloat(duration) / 1000
        : Number.parseFloat(duration));
      const iterations = style.animationIterationCount.split(",").map((count) => count.trim() === "infinite"
        ? Number.POSITIVE_INFINITY
        : Number.parseFloat(count));
      return {
        maxDuration: Math.max(...seconds),
        maxIterations: Math.max(...iterations),
        isShimmerLabel: element.matches(".pds-work-status__copy strong"),
        backgroundImage: style.backgroundImage
      };
    }));
    reducedMotionBrowserOk = motionStyles.length > 0
      && motionStyles.every(({ maxDuration, maxIterations }) => maxDuration <= 0.001 && maxIterations <= 1)
      && motionStyles.filter(({ isShimmerLabel }) => isShimmerLabel)
        .every(({ backgroundImage }) => backgroundImage === "none");

    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto(`${browserBaseUrl}/?view=intelligent-experience`, { waitUntil: "networkidle" });
    const ixFrame = page.frameLocator('iframe[title="Intelligent Experience reference experience"]');
    const returnLink = ixFrame.getByRole("link", { name: "Return to the full PDS Design System" });
    const footerLink = ixFrame.getByRole("link", { name: "Open the full PDS Design System" });
    const returnTarget = await returnLink.getAttribute("target");
    const footerTarget = await footerLink.getAttribute("target");
    await Promise.all([
      page.waitForURL((url) => url.pathname === "/" && url.search === ""),
      returnLink.click()
    ]);
    shellEscapeBrowserOk = returnTarget === "_top"
      && footerTarget === "_top"
      && page.frames().length === 1
      && await page.locator("iframe").count() === 0;

    const pageErrorFree = pageErrors.length === 0;
    noteBrowserOk = noteBrowserOk && pageErrorFree;
    focusBrowserOk = focusBrowserOk && pageErrorFree;
    punctuationBrowserOk = punctuationBrowserOk && pageErrorFree;
    sourceLinkBrowserOk = sourceLinkBrowserOk && pageErrorFree;
    mobileBrowserOk = mobileBrowserOk && pageErrorFree;
    reducedMotionBrowserOk = reducedMotionBrowserOk && pageErrorFree;
    shellEscapeBrowserOk = shellEscapeBrowserOk && pageErrorFree;
    const browserOk = noteBrowserOk
      && focusBrowserOk
      && punctuationBrowserOk
      && sourceLinkBrowserOk
      && mobileBrowserOk
      && reducedMotionBrowserOk
      && shellEscapeBrowserOk;
    browserDetail = browserOk
      ? "A physical browser passed note, focus, punctuation, source-link, mobile reflow, reduced-motion, and integrated-shell escape proof."
      : `Browser state mismatch (note=${noteBrowserOk}, focus=${focusBrowserOk}, punctuation=${punctuationBrowserOk}, sourceLink=${sourceLinkBrowserOk}, mobile=${mobileBrowserOk}, reducedMotion=${reducedMotionBrowserOk}, shellEscape=${shellEscapeBrowserOk}); page errors: ${pageErrors.join(" | ") || "none"}.`;
  } catch (error) {
    browserDetail = error instanceof Error ? error.stack ?? error.message : String(error);
  } finally {
    await browser?.close();
  }
  check("browser-working-goal-note-regression", noteBrowserOk, browserDetail);
  check("browser-state-change-focus-continuity", focusBrowserOk, browserDetail);
  check("browser-revision-announcement-punctuation", punctuationBrowserOk, browserDetail);
  check("browser-claim-source-link-resolves", sourceLinkBrowserOk, browserDetail);
  check("browser-mobile-reflow-and-freshness", mobileBrowserOk, browserDetail);
  check("browser-reduced-motion-is-static", reducedMotionBrowserOk, browserDetail);
  check("browser-integrated-shell-escape", shellEscapeBrowserOk, browserDetail);
  browserProof = [
    noteBrowserOk,
    focusBrowserOk,
    punctuationBrowserOk,
    sourceLinkBrowserOk,
    mobileBrowserOk,
    reducedMotionBrowserOk,
    shellEscapeBrowserOk
  ].every(Boolean) ? "passed" : "failed";
  browserProofDetail = browserDetail;
}

const failed = checks.filter((candidate) => !candidate.ok);
const result = {
  ok: failed.length === 0 && browserProof !== "failed",
  profile: "pds.ix_reference_gallery_proof@1",
  proofScope: browserProof === "not_run" ? "source_and_semantic_only" : "source_semantic_and_browser",
  browserProof,
  browserProofDetail,
  referenceCount: eightReferences.length,
  interactiveCount: 8,
  checks,
  failed: failed.map(({ name, detail }) => ({ name, detail }))
};

process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
if (!result.ok) process.exitCode = 1;
