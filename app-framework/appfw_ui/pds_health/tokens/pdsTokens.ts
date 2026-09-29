type DesignTokenTree = {
  readonly [key: string]: string | DesignTokenTree;
};

export const pdsTokens = {
  color: {
    brand: {
      gray: "#545860",
      slate: "#53585f",
      blue: "#00a9eb",
      blueBright: "#27c9ff",
      blueDeep: "#0077a8",
      blueDeeper: "#00638f",
      teal: "#00b8b0",
      violet: "#6f5cff"
    },
    signal: {
      blue: "#009ff5",
      teal: "#00d1c2",
      green: "#00c58e",
      violet: "#7658fa",
      coral: "#f44f77",
      amber: "#ffb020"
    },
    text: {
      default: "light-dark(#20262e, #f4f4f5)",
      muted: "light-dark(#59636f, #a1a1aa)",
      quiet: "light-dark(#64727f, #71717a)",
      inverse: "light-dark(#ffffff, #09090b)",
      onAccent: "#ffffff"
    },
    surface: {
      canvasStart: "light-dark(#f8fdff, #071019)",
      canvasMid: "light-dark(#fbfeff, #0f1a24)",
      canvasEnd: "light-dark(#ffffff, #17142a)",
      page: "light-dark(#fbfeff, #09090b)",
      panel: "light-dark(#ffffff, rgba(24, 30, 37, 0.82))",
      panelSoft: "light-dark(#f0f8ff, rgba(42, 48, 58, 0.64))",
      sidebar: "light-dark(#fbfdff, rgba(16, 22, 29, 0.88))",
      sidebarSoft: "light-dark(#edf7ff, rgba(29, 36, 46, 0.7))",
      elevated: "light-dark(#ffffff, rgba(48, 55, 66, 0.76))",
      glass: "light-dark(rgba(255, 255, 255, 0.72), rgba(21, 27, 35, 0.68))",
      glassOpaque: "light-dark(rgba(255, 255, 255, 0.97), rgba(21, 27, 35, 0.97))",
      track: "light-dark(rgba(2, 68, 108, 0.07), rgba(2, 6, 12, 0.42))"
    },
    border: {
      default: "light-dark(rgba(0, 100, 158, 0.17), rgba(87, 96, 112, 0.66))",
      accent: "light-dark(rgba(0, 140, 220, 0.46), rgba(39, 201, 255, 0.38))",
      highlight: "light-dark(rgba(255, 255, 255, 0.96), rgba(255, 255, 255, 0.08))",
      highlightSoft: "light-dark(rgba(255, 255, 255, 0.35), rgba(255, 255, 255, 0.04))",
      hairline: "light-dark(rgba(0, 100, 158, 0.1), rgba(148, 166, 190, 0.16))"
    },
    state: {
      accentSoft: "light-dark(#e8f8ff, rgba(39, 201, 255, 0.18))",
      danger: "light-dark(#b4234a, #ff7a95)",
      dangerBg: "light-dark(#fff3f6, rgba(244, 79, 119, 0.18))",
      dangerSoft: "light-dark(#fff8fa, rgba(244, 79, 119, 0.1))",
      success: "light-dark(#08775b, #66dfb2)",
      successBg: "light-dark(#e4fbf2, rgba(0, 197, 142, 0.18))",
      successSoft: "light-dark(#effcf7, rgba(0, 197, 142, 0.12))",
      teal: "light-dark(#00786f, #53e3d5)",
      gold: "#ffb020",
      warning: "light-dark(#855600, #ffc45c)",
      focus: "light-dark(rgba(0, 119, 168, 0.13), rgba(39, 201, 255, 0.2))"
    },
    glow: {
      accent: "light-dark(rgba(0, 159, 245, 0.14), rgba(39, 201, 255, 0.24))",
      violet: "light-dark(rgba(111, 92, 255, 0.08), rgba(111, 92, 255, 0.2))",
      warm: "light-dark(rgba(255, 138, 91, 0.08), rgba(255, 122, 89, 0.1))"
    },
    spotlight: "light-dark(rgba(0, 159, 245, 0.09), rgba(39, 201, 255, 0.16))",
    spotlightViolet: "light-dark(rgba(111, 92, 255, 0.06), rgba(111, 92, 255, 0.18))",
    spotlightWarm: "light-dark(rgba(255, 138, 91, 0.08), rgba(255, 122, 89, 0.1))",
    intelligence: {
      surface: "light-dark(#faf9ff, rgba(24, 30, 37, 0.82))",
      border: "light-dark(rgba(96, 77, 214, 0.24), rgba(111, 92, 255, 0.4))"
    },
    backdropScrim: "light-dark(rgba(9, 9, 11, 0.42), rgba(0, 0, 0, 0.62))"
  },
  gradient: {
    canvas:
      "linear-gradient(180deg, var(--pds-color-surface-canvas-start) 0%, var(--pds-color-surface-canvas-mid) 46%, var(--pds-color-surface-canvas-end) 100%)",
    panel:
      "linear-gradient(180deg, light-dark(#ffffff, rgba(38, 45, 56, 0.86)) 0%, var(--pds-color-surface-panel) 100%)",
    sidebar:
      "linear-gradient(180deg, light-dark(#edf7ff, rgba(20, 27, 36, 0.94)) 0%, var(--pds-color-surface-sidebar) 62%, var(--pds-color-surface-sidebar-soft) 100%)",
    control: "linear-gradient(180deg, var(--pds-color-surface-elevated) 0%, var(--pds-color-surface-elevated) 100%)",
    accent:
      "linear-gradient(135deg, var(--pds-color-brand-blue-deeper) 0%, var(--pds-color-brand-blue-deep) 58%, #007eae 100%)",
    borderAccent:
      "linear-gradient(135deg, var(--pds-color-brand-blue) 0%, var(--pds-color-brand-violet) 48%, var(--pds-color-brand-teal) 100%)",
    intelligence: "linear-gradient(135deg, var(--pds-color-brand-blue-deep) 0%, var(--pds-color-brand-violet) 100%)",
    progress:
      "linear-gradient(90deg, var(--pds-color-signal-blue) 0%, color-mix(in oklch, var(--pds-color-signal-blue) 72%, var(--pds-color-signal-teal)) 100%)"
  },
  font: {
    familySans:
      '"InterVariable", "Inter Fallback: Segoe UI", "Inter Fallback: Arial", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Arial, sans-serif',
    familyMono:
      '"Geist Mono Variable", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
    familyDisplay:
      '"Poppins", "InterVariable", "Inter Fallback: Segoe UI", "Inter Fallback: Arial", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Arial, sans-serif',
    featureSans: '"cv01" 1, "ss03" 1, "calt" 1, "liga" 1',
    opticalSizing: "auto",
    size: {
      xs: "11px",
      sm: "12px",
      md: "13px",
      base: "15px",
      lg: "16px",
      metric: "32px"
    },
    weight: {
      regular: "400",
      medium: "500",
      semibold: "600",
      bold: "700",
      metric: "600",
      display: "700"
    },
    variantMetric: "lining-nums tabular-nums",
    navigation: {
      labelSize: "14px",
      labelLineHeight: "20px",
      labelWeight: "500"
    }
  },
  space: {
    "1": "4px",
    "2": "8px",
    "3": "12px",
    "4": "16px",
    "5": "20px",
    "6": "24px",
    "8": "32px"
  },
  radius: {
    sm: "6px",
    md: "8px",
    lg: "10px",
    pill: "999px",
    control: "10px",
    panel: "14px"
  },
  shadow: {
    card: "0 18px 42px light-dark(rgba(0, 86, 138, 0.08), rgba(0, 0, 0, 0.26))",
    control:
      "0 0 0 1px light-dark(rgba(0, 119, 168, 0.18), rgba(39, 201, 255, 0.18)), 0 12px 28px var(--pds-color-glow-accent)",
    panel: "0 24px 64px light-dark(rgba(0, 86, 138, 0.11), rgba(0, 0, 0, 0.42))",
    thumb: "0 1px 2px light-dark(rgba(4, 44, 83, 0.16), rgba(0, 0, 0, 0.42)), 0 3px 9px light-dark(rgba(0, 86, 138, 0.12), rgba(0, 0, 0, 0.3))",
    track: "inset 0 1px 2px light-dark(rgba(4, 44, 83, 0.09), rgba(0, 0, 0, 0.35))",
    specular: "inset 0 1px 1px var(--pds-color-border-highlight), inset 0 -1px 1px var(--pds-color-border-highlight-soft)"
  },
  backdrop: {
    panel: "blur(18px) saturate(1.18)",
    glass: "blur(16px) saturate(1.8)"
  },
  motion: {
    duration: {
      fast: "120ms",
      standard: "200ms",
      slow: "300ms"
    },
    easing: {
      standard: "cubic-bezier(0.2, 0, 0.38, 0.9)",
      enter: "cubic-bezier(0.16, 1, 0.3, 1)",
      exit: "cubic-bezier(0.2, 0, 1, 0.9)",
      spring: "cubic-bezier(0.22, 1, 0.36, 1)"
    },
    scale: {
      press: "0.97",
      enter: "0.96"
    },
    distanceEnter: "8px",
    fluid: "cubic-bezier(0.16, 1, 0.3, 1)"
  }
} as const satisfies DesignTokenTree;

export const pdsVisualThemeIds = ["apple-like", "material-like"] as const;

export const pdsVisualThemes = {
  "apple-like": {
    label: "Apple-like",
    default: true,
    character: "luminous translucent surfaces, fine borders, restrained glow, and depth"
  },
  "material-like": {
    label: "Material-like",
    default: false,
    character: "PDS-seeded Material 3 tonal roles, expressive shape, state layers, and discrete elevation"
  }
} as const satisfies Record<(typeof pdsVisualThemeIds)[number], {
  readonly label: string;
  readonly default: boolean;
  readonly character: string;
}>;

export const pdsMaterial3Theme = {
  sourceColor: "#00a9eb",
  tokenContract: "Google Material 3 v0.192",
  colorRoles: {
    primary: "var(--pds-m3-color-primary)",
    onPrimary: "var(--pds-m3-color-on-primary)",
    primaryContainer: "var(--pds-m3-color-primary-container)",
    onPrimaryContainer: "var(--pds-m3-color-on-primary-container)",
    secondary: "var(--pds-m3-color-secondary)",
    onSecondary: "var(--pds-m3-color-on-secondary)",
    secondaryContainer: "var(--pds-m3-color-secondary-container)",
    onSecondaryContainer: "var(--pds-m3-color-on-secondary-container)",
    tertiary: "var(--pds-m3-color-tertiary)",
    onTertiary: "var(--pds-m3-color-on-tertiary)",
    tertiaryContainer: "var(--pds-m3-color-tertiary-container)",
    onTertiaryContainer: "var(--pds-m3-color-on-tertiary-container)",
    error: "var(--pds-m3-color-error)",
    onError: "var(--pds-m3-color-on-error)",
    errorContainer: "var(--pds-m3-color-error-container)",
    onErrorContainer: "var(--pds-m3-color-on-error-container)",
    surface: "var(--pds-m3-color-surface)",
    onSurface: "var(--pds-m3-color-on-surface)",
    surfaceContainerLowest: "var(--pds-m3-color-surface-container-lowest)",
    surfaceContainerLow: "var(--pds-m3-color-surface-container-low)",
    surfaceContainer: "var(--pds-m3-color-surface-container)",
    surfaceContainerHigh: "var(--pds-m3-color-surface-container-high)",
    surfaceContainerHighest: "var(--pds-m3-color-surface-container-highest)",
    outline: "var(--pds-m3-color-outline)",
    outlineVariant: "var(--pds-m3-color-outline-variant)"
  },
  shape: {
    extraSmall: "var(--pds-m3-shape-extra-small)",
    small: "var(--pds-m3-shape-small)",
    medium: "var(--pds-m3-shape-medium)",
    large: "var(--pds-m3-shape-large)",
    extraLarge: "var(--pds-m3-shape-extra-large)",
    full: "var(--pds-m3-shape-full)"
  },
  stateLayer: {
    hover: "var(--pds-m3-state-hover-opacity)",
    focus: "var(--pds-m3-state-focus-opacity)",
    pressed: "var(--pds-m3-state-pressed-opacity)",
    dragged: "var(--pds-m3-state-dragged-opacity)"
  },
  elevation: {
    level0: "var(--pds-m3-elevation-level0)",
    level1: "var(--pds-m3-elevation-level1)",
    level2: "var(--pds-m3-elevation-level2)",
    level3: "var(--pds-m3-elevation-level3)",
    level4: "var(--pds-m3-elevation-level4)",
    level5: "var(--pds-m3-elevation-level5)"
  }
} as const;

export const pdsTokenCssVars = {
  colorBrandGray: "var(--pds-color-brand-gray)",
  colorBrandSlate: "var(--pds-color-brand-slate)",
  colorBrandBlue: "var(--pds-color-brand-blue)",
  colorBrandBlueBright: "var(--pds-color-brand-blue-bright)",
  colorBrandBlueDeep: "var(--pds-color-brand-blue-deep)",
  colorBrandBlueDeeper: "var(--pds-color-brand-blue-deeper)",
  colorBrandTeal: "var(--pds-color-brand-teal)",
  colorBrandViolet: "var(--pds-color-brand-violet)",
  colorSignalBlue: "var(--pds-color-signal-blue)",
  colorSignalTeal: "var(--pds-color-signal-teal)",
  colorSignalGreen: "var(--pds-color-signal-green)",
  colorSignalViolet: "var(--pds-color-signal-violet)",
  colorSignalCoral: "var(--pds-color-signal-coral)",
  colorSignalAmber: "var(--pds-color-signal-amber)",
  colorTextDefault: "var(--pds-color-text-default)",
  colorTextMuted: "var(--pds-color-text-muted)",
  colorTextQuiet: "var(--pds-color-text-quiet)",
  colorTextInverse: "var(--pds-color-text-inverse)",
  colorTextOnAccent: "var(--pds-color-text-on-accent)",
  colorSurfaceCanvasStart: "var(--pds-color-surface-canvas-start)",
  colorSurfaceCanvasMid: "var(--pds-color-surface-canvas-mid)",
  colorSurfaceCanvasEnd: "var(--pds-color-surface-canvas-end)",
  colorSurfacePage: "var(--pds-color-surface-page)",
  colorSurfacePanel: "var(--pds-color-surface-panel)",
  colorSurfacePanelSoft: "var(--pds-color-surface-panel-soft)",
  colorSurfaceSidebar: "var(--pds-color-surface-sidebar)",
  colorSurfaceSidebarSoft: "var(--pds-color-surface-sidebar-soft)",
  colorSurfaceElevated: "var(--pds-color-surface-elevated)",
  colorBorderDefault: "var(--pds-color-border-default)",
  colorBorderAccent: "var(--pds-color-border-accent)",
  colorBorderHighlight: "var(--pds-color-border-highlight)",
  colorStateAccentSoft: "var(--pds-color-state-accent-soft)",
  colorStateDanger: "var(--pds-color-state-danger)",
  colorStateDangerBg: "var(--pds-color-state-danger-bg)",
  colorStateDangerSoft: "var(--pds-color-state-danger-soft)",
  colorStateSuccess: "var(--pds-color-state-success)",
  colorStateSuccessBg: "var(--pds-color-state-success-bg)",
  colorStateSuccessSoft: "var(--pds-color-state-success-soft)",
  colorStateTeal: "var(--pds-color-state-teal)",
  colorStateGold: "var(--pds-color-state-gold)",
  colorStateWarning: "var(--pds-color-state-warning)",
  colorStateFocus: "var(--pds-color-state-focus)",
  colorGlowAccent: "var(--pds-color-glow-accent)",
  colorGlowViolet: "var(--pds-color-glow-violet)",
  colorGlowWarm: "var(--pds-color-glow-warm)",
  colorSpotlight: "var(--pds-color-spotlight)",
  colorSpotlightViolet: "var(--pds-color-spotlight-violet)",
  colorSpotlightWarm: "var(--pds-color-spotlight-warm)",
  colorIntelligenceSurface: "var(--pds-color-intelligence-surface)",
  colorIntelligenceBorder: "var(--pds-color-intelligence-border)",
  colorBackdropScrim: "var(--pds-color-backdrop-scrim)",
  gradientCanvas: "var(--pds-gradient-canvas)",
  gradientPanel: "var(--pds-gradient-panel)",
  gradientSidebar: "var(--pds-gradient-sidebar)",
  gradientControl: "var(--pds-gradient-control)",
  gradientAccent: "var(--pds-gradient-accent)",
  gradientBorderAccent: "var(--pds-gradient-border-accent)",
  gradientIntelligence: "var(--pds-gradient-intelligence)",
  gradientProgress: "var(--pds-gradient-progress)",
  fontFamilySans: "var(--pds-font-family-sans)",
  fontFamilyMono: "var(--pds-font-family-mono)",
  fontSans: "var(--pds-font-sans)",
  fontMono: "var(--pds-font-mono)",
  fontFeatureSans: "var(--pds-font-feature-sans)",
  fontOpticalSizing: "var(--pds-font-optical-sizing)",
  fontSizeMetric: "var(--pds-font-size-metric)",
  fontWeightMetric: "var(--pds-font-weight-metric)",
  fontVariantMetric: "var(--pds-font-variant-metric)",
  radiusMd: "var(--pds-radius-md)",
  shadowPanel: "var(--pds-shadow-panel)",
  backdropPanel: "var(--pds-backdrop-panel)",
  motionDurationFast: "var(--pds-motion-duration-fast)",
  motionDurationStandard: "var(--pds-motion-duration-standard)",
  motionDurationSlow: "var(--pds-motion-duration-slow)",
  motionDurationFieldLabel: "var(--pds-motion-duration-field-label)",
  motionEasingStandard: "var(--pds-motion-easing-standard)",
  motionEasingEnter: "var(--pds-motion-easing-enter)",
  motionEasingExit: "var(--pds-motion-easing-exit)",
  motionEasingFieldLabel: "var(--pds-motion-easing-field-label)",
  motionScalePress: "var(--pds-motion-scale-press)",
  motionScaleEnter: "var(--pds-motion-scale-enter)",
  motionDistanceEnter: "var(--pds-motion-distance-enter)",
  motionFluid: "var(--pds-motion-fluid)"
} as const;

export type PdsTokens = typeof pdsTokens;
export type PdsTokenCssVar = keyof typeof pdsTokenCssVars;
export type PdsVisualTheme = (typeof pdsVisualThemeIds)[number];
