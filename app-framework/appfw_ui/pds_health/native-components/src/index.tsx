import {
  validatePdsIxPresentation,
  type EvidenceDisclosureModel,
  type PdsIxPresentationEnvelope,
  type PdsIxRecipeProjection,
  type PdsIxRecipeRegistration,
  type ProgressiveResponsePresentationModel,
  type ProgressiveResponseRegionModel,
  type ResolvedContextPresentationModel,
  type WorkStatusPresentationModel
} from "@appfw/pds-ix-presentation-contract";
import {
  resolvePdsIxNativeRecipe,
  type PdsIxNativeProjection
} from "./ix-recipe-projection.js";
import {
  pdsNativeDesignData,
  pdsNativeTokensFor,
  type PdsNativeColorScheme,
  type PdsNativeVisualTheme
} from "./design-data.js";
import { useEffect, useMemo, useState } from "react";
import {
  AccessibilityInfo,
  Pressable,
  StyleSheet,
  Text,
  View,
  type TextStyle,
  type ViewStyle
} from "react-native";

export { pdsNativeDesignData };
export type { PdsNativeColorScheme, PdsNativeVisualTheme };
export { resolvePdsIxNativeRecipe } from "./ix-recipe-projection.js";
export type { PdsIxNativeProjection } from "./ix-recipe-projection.js";

type NativePresentationProps = {
  colorScheme?: PdsNativeColorScheme;
  visualTheme?: PdsNativeVisualTheme;
};

export type EvidenceDisclosureProps = NativePresentationProps & {
  model: EvidenceDisclosureModel;
  initiallyExpanded?: boolean;
  onSourcePress?: (sourceRef: string) => void;
};

export type ResolvedContextDisclosureProps = NativePresentationProps & {
  model: ResolvedContextPresentationModel;
  onAction?: () => void;
  onSourcePress?: (sourceRef: string) => void;
};

export type WorkStatusProps = NativePresentationProps & {
  model: WorkStatusPresentationModel;
  onAction?: () => void;
};

export type ProgressiveResponseProps = NativePresentationProps & {
  model: ProgressiveResponsePresentationModel;
  onRegionAction?: (region: ProgressiveResponseRegionModel) => void;
  onRegionEdit?: (region: ProgressiveResponseRegionModel) => void;
  onSourcePress?: (sourceRef: string) => void;
};

export type PdsIxPresentationProps = NativePresentationProps & {
  presentation: unknown;
  onContextAction?: () => void;
  onStatusAction?: () => void;
  onRegionAction?: (region: ProgressiveResponseRegionModel) => void;
  onRegionEdit?: (region: ProgressiveResponseRegionModel) => void;
  onSourcePress?: (sourceRef: string) => void;
};

export type PdsIxRecipePresentationProps = NativePresentationProps & {
  registration: unknown;
  presentation: unknown;
  projection: PdsIxNativeProjection;
  onContextAction?: (registration: PdsIxRecipeRegistration) => void;
  onStatusAction?: (registration: PdsIxRecipeRegistration) => void;
  onRegionAction?: (
    region: ProgressiveResponseRegionModel,
    registration: PdsIxRecipeRegistration
  ) => void;
  onRegionEdit?: (
    region: ProgressiveResponseRegionModel,
    registration: PdsIxRecipeRegistration
  ) => void;
  onSourcePress?: (sourceRef: string, registration: PdsIxRecipeRegistration) => void;
};

function Action({ label, onPress, styles }: {
  label?: string;
  onPress?: () => void;
  styles: ReturnType<typeof stylesFor>;
}) {
  if (!label || !onPress) return null;
  return (
    <Pressable
      accessibilityLabel={label}
      accessibilityRole="button"
      hitSlop={styles.hitSlop}
      onPress={onPress}
      style={styles.action}
    >
      <Text {...styles.textProps} style={styles.actionText}>{label}</Text>
    </Pressable>
  );
}

export function EvidenceDisclosure({
  model,
  colorScheme = "light",
  visualTheme,
  initiallyExpanded = false,
  onSourcePress
}: EvidenceDisclosureProps) {
  const styles = stylesFor(colorScheme, visualTheme);
  const [expanded, setExpanded] = useState(initiallyExpanded);
  return (
    <View style={styles.evidence}>
      <Pressable
        accessibilityLabel={model.summary}
        accessibilityRole="button"
        accessibilityState={{ expanded }}
        hitSlop={styles.hitSlop}
        onPress={() => setExpanded((value) => !value)}
        style={styles.disclosureButton}
      >
        <Text {...styles.textProps} style={styles.evidenceSummary}>{model.summary}</Text>
        <Text {...styles.textProps} accessibilityElementsHidden style={styles.disclosureIndicator}>
          {expanded ? "−" : "+"}
        </Text>
      </Pressable>
      {expanded ? model.items.map((item, index) => {
        const itemBody = (
          <>
            <Text {...styles.textProps} style={styles.label}>{item.label}</Text>
            <Text {...styles.textProps} selectable style={styles.value}>{item.value}</Text>
          </>
        );
        return item.sourceRef && onSourcePress ? (
          <Pressable
            accessibilityLabel={`${item.label}: ${item.value}. Open source reference.`}
            accessibilityRole="link"
            hitSlop={styles.hitSlop}
            key={item.id ?? item.sourceRef}
            onPress={() => onSourcePress(item.sourceRef!)}
            style={styles.sourceAction}
            testID={`pds-source-${item.sourceRef}`}
          >
            {itemBody}
          </Pressable>
        ) : (
          <View
            accessibilityLabel={`${item.label}: ${item.value}.${item.sourceRef ? ` Source reference: ${item.sourceRef}.` : ""}`}
            key={item.id ?? item.sourceRef ?? `${item.label}-${index}`}
            style={styles.evidenceItem}
            testID={item.sourceRef ? `pds-source-${item.sourceRef}` : undefined}
          >
            {itemBody}
          </View>
        );
      }) : null}
    </View>
  );
}

export function ResolvedContextDisclosure({
  model,
  colorScheme = "light",
  visualTheme,
  onAction,
  onSourcePress
}: ResolvedContextDisclosureProps) {
  const styles = stylesFor(colorScheme, visualTheme);
  return (
    <View accessibilityLabel="Context used for this work" style={styles.panel}>
      <Text {...styles.textProps} style={styles.eyebrow}>{model.eyebrow}</Text>
      <Text {...styles.textProps} accessibilityRole="header" style={styles.title}>{model.title}</Text>
      {model.detail ? <Text {...styles.textProps} style={styles.body}>{model.detail}</Text> : null}
      {model.metaLabel ? <Text {...styles.textProps} style={styles.meta}>{model.metaLabel}</Text> : null}
      {model.gaps.length > 0 ? (
        <View accessibilityLabel="Context gaps" style={styles.gaps}>
          <Text {...styles.textProps} accessibilityRole="header" style={styles.sectionTitle}>Context gaps</Text>
          {model.gaps.map((gap) => <Text {...styles.textProps} key={gap} style={styles.body}>• {gap}</Text>)}
        </View>
      ) : null}
      <EvidenceDisclosure
        colorScheme={colorScheme}
        visualTheme={visualTheme}
        model={model.evidence}
        onSourcePress={onSourcePress}
      />
      <Action label={model.actionLabel} onPress={onAction} styles={styles} />
    </View>
  );
}

export function WorkStatus({ model, colorScheme = "light", visualTheme, onAction }: WorkStatusProps) {
  const styles = stylesFor(colorScheme, visualTheme);
  return (
    <View
      accessibilityLabel={`${model.label}${model.detail ? `. ${model.detail}` : ""}`}
      accessibilityRole="summary"
      style={[styles.workStatus, model.active ? styles.workStatusActive : null]}
    >
      <View style={[styles.statusDot, model.active ? styles.statusDotActive : null]} />
      <View style={styles.flex}>
        <Text {...styles.textProps} style={styles.sectionTitle}>{model.label}</Text>
        {model.detail ? <Text {...styles.textProps} style={styles.meta}>{model.detail}</Text> : null}
      </View>
      <Action label={model.actionLabel} onPress={onAction} styles={styles} />
    </View>
  );
}

export function ProgressiveResponse({
  model,
  colorScheme = "light",
  visualTheme,
  onRegionAction,
  onRegionEdit,
  onSourcePress
}: ProgressiveResponseProps) {
  const styles = stylesFor(colorScheme, visualTheme);
  return (
    <View style={styles.response}>
      <Text {...styles.textProps} style={styles.eyebrow}>{model.eyebrow}</Text>
      <Text {...styles.textProps} accessibilityRole="header" style={styles.title}>{model.title}</Text>
      {model.metaLabel ? <Text {...styles.textProps} style={styles.meta}>{model.metaLabel}</Text> : null}
      {model.regions.length === 0 ? (
        <Text {...styles.textProps} accessibilityRole="summary" style={styles.empty}>{model.emptyState}</Text>
      ) : model.regions.map((region) => (
        <View
          accessibilityLabel={`${region.label ?? "Response region"}. Status: ${region.status}.`}
          key={region.id}
          style={[styles.region, region.changed ? styles.changedRegion : null]}
          testID={`pds-region-${region.id}`}
        >
          <View style={styles.regionHeader}>
            <Text {...styles.textProps} style={styles.label}>{region.label ?? "Response"}</Text>
            <Text {...styles.textProps} accessibilityLabel={`Status: ${region.status}.`} style={styles.statusBadge}>
              {region.status}
            </Text>
          </View>
          <Text {...styles.textProps} accessibilityRole="header" style={styles.sectionTitle}>{region.title}</Text>
          <Text {...styles.textProps} style={styles.body}>{region.body}</Text>
          {region.whyItMatters ? (
            <Text {...styles.textProps} style={styles.why}>
              <Text {...styles.textProps} style={styles.label}>Why this matters: </Text>{region.whyItMatters}
            </Text>
          ) : null}
          {region.evidence ? (
            <EvidenceDisclosure
              colorScheme={colorScheme}
              visualTheme={visualTheme}
              model={region.evidence}
              onSourcePress={onSourcePress}
            />
          ) : null}
          <Action
            label={region.actionLabel}
            onPress={onRegionAction ? () => onRegionAction(region) : undefined}
            styles={styles}
          />
          <Action
            label={region.editable ? "Edit this section" : undefined}
            onPress={onRegionEdit ? () => onRegionEdit(region) : undefined}
            styles={styles}
          />
        </View>
      ))}
      {model.footerText ? <Text {...styles.textProps} style={styles.meta}>{model.footerText}</Text> : null}
    </View>
  );
}

function ValidPresentation({
  presentation,
  colorScheme,
  visualTheme,
  onContextAction,
  onStatusAction,
  onRegionAction,
  onRegionEdit,
  onSourcePress
}: Omit<PdsIxPresentationProps, "presentation"> & { presentation: PdsIxPresentationEnvelope }) {
  const styles = stylesFor(colorScheme, visualTheme);
  const revisionIdentity = `${presentation.identity.presentationId}:${presentation.identity.revision}`;
  useEffect(() => {
    AccessibilityInfo.announceForAccessibility(presentation.announcement);
  }, [revisionIdentity]);
  return (
    <View
      accessibilityLabel={`Intelligence presentation revision ${presentation.identity.revision}`}
      style={styles.composition}
      testID={`pds-presentation-${presentation.identity.presentationId}`}
    >
      {presentation.context ? (
        <ResolvedContextDisclosure
          colorScheme={colorScheme}
          visualTheme={visualTheme}
          model={presentation.context}
          onAction={onContextAction}
          onSourcePress={onSourcePress}
        />
      ) : null}
      {presentation.workStatus ? (
        <WorkStatus colorScheme={colorScheme} visualTheme={visualTheme} model={presentation.workStatus} onAction={onStatusAction} />
      ) : null}
      <ProgressiveResponse
        colorScheme={colorScheme}
        visualTheme={visualTheme}
        model={presentation.response}
        onRegionAction={onRegionAction}
        onRegionEdit={onRegionEdit}
        onSourcePress={onSourcePress}
      />
    </View>
  );
}

export function PdsIxPresentation(props: PdsIxPresentationProps) {
  const validation = useMemo(() => validatePdsIxPresentation(props.presentation), [props.presentation]);
  const styles = stylesFor(props.colorScheme, props.visualTheme);
  if (!validation.ok) {
    return (
      <View accessibilityLabel="Intelligence presentation unavailable" style={styles.fallback} testID="pds-presentation-invalid">
        <Text {...styles.textProps} accessibilityRole="alert" style={styles.sectionTitle}>
          This intelligence presentation is unavailable.
        </Text>
        <Text {...styles.textProps} style={styles.body}>Try again after the presentation data is refreshed.</Text>
      </View>
    );
  }
  return <ValidPresentation {...props} presentation={validation.value} />;
}

/**
 * Channel-appropriate native composition for any registered PDS IX recipe.
 * Product data, copy, actions, policy, and lifecycle remain caller-owned.
 */
export function PdsIxRecipePresentation({
  registration,
  presentation,
  projection,
  colorScheme,
  visualTheme,
  onContextAction,
  onStatusAction,
  onRegionAction,
  onRegionEdit,
  onSourcePress
}: PdsIxRecipePresentationProps) {
  let projected: PdsIxRecipeProjection;
  try {
    projected = resolvePdsIxNativeRecipe(registration, projection);
  } catch {
    return (
      <PdsIxPresentation
        colorScheme={colorScheme}
        presentation={null}
        visualTheme={visualTheme}
      />
    );
  }
  const exactRegistration = projected.registration;
  return (
    <PdsIxPresentation
      colorScheme={colorScheme}
      onContextAction={
        onContextAction ? () => onContextAction(exactRegistration) : undefined
      }
      onRegionAction={
        onRegionAction
          ? (region) => onRegionAction(region, exactRegistration)
          : undefined
      }
      onRegionEdit={
        onRegionEdit
          ? (region) => onRegionEdit(region, exactRegistration)
          : undefined
      }
      onSourcePress={
        onSourcePress
          ? (sourceRef) => onSourcePress(sourceRef, exactRegistration)
          : undefined
      }
      onStatusAction={
        onStatusAction ? () => onStatusAction(exactRegistration) : undefined
      }
      presentation={presentation}
      visualTheme={visualTheme}
    />
  );
}

const styleCache = new Map<string, ReturnType<typeof createStyles>>();

function stylesFor(scheme: PdsNativeColorScheme = "light", visualTheme: PdsNativeVisualTheme = "apple-like") {
  const cacheKey = `${visualTheme}:${scheme}`;
  const cached = styleCache.get(cacheKey);
  if (cached) return cached;
  const created = createStyles(scheme, visualTheme);
  styleCache.set(cacheKey, created);
  return created;
}

function fontWeight(value: number): TextStyle["fontWeight"] {
  return String(value) as TextStyle["fontWeight"];
}

function createStyles(scheme: PdsNativeColorScheme, visualTheme: PdsNativeVisualTheme) {
  const tokens = pdsNativeTokensFor({ visualTheme, colorScheme: scheme });
  const panel: ViewStyle = {
    backgroundColor: tokens.color.surface,
    borderColor: tokens.color.border,
    borderRadius: tokens.shape.panel,
    borderWidth: 1,
    padding: tokens.spacing["16"]
  };
  const textProps = textPropsFor(tokens);
  return {
    ...StyleSheet.create({
      action: { alignItems: "center", alignSelf: "flex-start", backgroundColor: tokens.color.action, borderRadius: tokens.shape.control, justifyContent: "center", marginTop: tokens.spacing["8"], minHeight: tokens.geometry.minimumTarget, minWidth: tokens.geometry.minimumTarget, paddingHorizontal: tokens.spacing["16"] },
      actionText: { color: tokens.color.onAction, fontSize: tokens.typography.label.fontSize, fontWeight: fontWeight(tokens.typography.label.fontWeight), lineHeight: tokens.typography.label.lineHeight },
      body: { color: tokens.color.text, fontSize: tokens.typography.body.fontSize, fontWeight: fontWeight(tokens.typography.body.fontWeight), lineHeight: tokens.typography.body.lineHeight },
      changedRegion: { borderColor: tokens.color.accent },
      composition: { backgroundColor: tokens.color.canvas, gap: tokens.spacing["16"], padding: tokens.spacing["20"] },
      disclosureButton: { alignItems: "center", flexDirection: "row", justifyContent: "space-between", minHeight: tokens.geometry.minimumTarget },
      disclosureIndicator: { color: tokens.color.action, fontSize: tokens.typography.title.fontSize, lineHeight: tokens.typography.title.lineHeight },
      empty: { color: tokens.color.muted, fontSize: tokens.typography.body.fontSize, paddingVertical: tokens.spacing["16"] },
      evidence: { borderTopColor: tokens.color.border, borderTopWidth: 1, gap: tokens.spacing["8"], marginTop: tokens.spacing["12"], paddingTop: tokens.spacing["12"] },
      evidenceItem: { gap: tokens.spacing["4"] },
      evidenceSummary: { color: tokens.color.action, fontSize: tokens.typography.label.fontSize, fontWeight: fontWeight(tokens.typography.label.fontWeight), lineHeight: tokens.typography.label.lineHeight },
      eyebrow: { color: tokens.color.action, fontSize: tokens.typography.caption.fontSize, fontWeight: fontWeight(tokens.typography.label.fontWeight), letterSpacing: 0.6, lineHeight: tokens.typography.caption.lineHeight, textTransform: "uppercase" },
      fallback: { ...panel, backgroundColor: tokens.color.subtle, gap: tokens.spacing["8"] },
      flex: { flex: 1 },
      gaps: { backgroundColor: tokens.color.intelligence, borderRadius: tokens.shape.control, gap: tokens.spacing["4"], padding: tokens.spacing["12"] },
      label: { color: tokens.color.muted, fontSize: tokens.typography.caption.fontSize, fontWeight: fontWeight(tokens.typography.label.fontWeight), lineHeight: tokens.typography.caption.lineHeight },
      meta: { color: tokens.color.muted, fontSize: tokens.typography.caption.fontSize, lineHeight: tokens.typography.caption.lineHeight },
      panel,
      region: { ...panel, gap: tokens.spacing["8"] },
      regionHeader: { alignItems: "center", flexDirection: "row", justifyContent: "space-between" },
      response: { gap: tokens.spacing["12"] },
      sectionTitle: { color: tokens.color.text, fontSize: tokens.typography.title.fontSize, fontWeight: fontWeight(tokens.typography.title.fontWeight), lineHeight: tokens.typography.title.lineHeight },
      sourceAction: { borderRadius: tokens.shape.control, gap: tokens.spacing["4"], minHeight: tokens.geometry.minimumTarget, paddingVertical: tokens.spacing["4"] },
      statusBadge: { color: tokens.color.action, fontSize: tokens.typography.caption.fontSize, fontWeight: fontWeight(tokens.typography.label.fontWeight), lineHeight: tokens.typography.caption.lineHeight, textTransform: "uppercase" },
      statusDot: { backgroundColor: tokens.color.muted, borderRadius: tokens.shape.pill, height: tokens.spacing["12"], width: tokens.spacing["12"] },
      statusDotActive: { backgroundColor: tokens.color.active },
      title: { color: tokens.color.text, fontSize: tokens.typography.display.fontSize, fontWeight: fontWeight(tokens.typography.display.fontWeight), lineHeight: tokens.typography.display.lineHeight },
      value: { color: tokens.color.text, fontSize: tokens.typography.body.fontSize, lineHeight: tokens.typography.body.lineHeight },
      why: { color: tokens.color.text, fontSize: tokens.typography.body.fontSize, lineHeight: tokens.typography.body.lineHeight },
      workStatus: { alignItems: "center", borderColor: tokens.color.border, borderRadius: tokens.shape.control, borderWidth: 1, flexDirection: "row", gap: tokens.spacing["12"], padding: tokens.spacing["12"] },
      workStatusActive: { borderColor: tokens.color.active }
    }),
    hitSlop: { bottom: tokens.spacing["4"], left: tokens.spacing["4"], right: tokens.spacing["4"], top: tokens.spacing["4"] },
    textProps
  };
}

function textPropsFor(tokens: ReturnType<typeof pdsNativeTokensFor>) {
  return {
    allowFontScaling: tokens.fontPolicy.allowFontScaling
  } as const;
}
