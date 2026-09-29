// Generated from tokens.dtcg.json by generate-pds-native-design-data.mjs.
// Do not hand-edit.
export const pdsNativeDesignData = {
  "schemaVersion": "pds.native.design-data@1",
  "source": {
    "path": "appfw_ui/pds_health/tokens/tokens.dtcg.json",
    "sha256": "sha256:a8f7d627bb89dd94492d8be6cadff83b92f877526ca54cdf99b4a05674d16f0e"
  },
  "defaultSelection": {
    "visualTheme": "apple-like",
    "colorScheme": "light"
  },
  "visualThemes": {
    "apple-like": {
      "light": {
        "identity": {
          "visualTheme": "apple-like",
          "colorScheme": "light"
        },
        "color": {
          "action": "#0077a8",
          "onAction": "#ffffff",
          "canvas": "#f8fdff",
          "surface": "#ffffff",
          "subtle": "#f0f8ff",
          "elevated": "#ffffff",
          "intelligence": "#faf9ff",
          "text": "#20262e",
          "muted": "#59636f",
          "inverse": "#ffffff",
          "border": "rgba(0, 100, 158, 0.17)",
          "accent": "rgba(0, 140, 220, 0.46)",
          "focus": "rgba(0, 119, 168, 0.13)",
          "active": "#009ff5",
          "success": "#08775b",
          "warning": "#855600",
          "danger": "#b4234a"
        },
        "spacing": {
          "4": 4,
          "8": 8,
          "12": 12,
          "16": 16,
          "20": 20,
          "24": 24,
          "32": 32
        },
        "typography": {
          "caption": {
            "fontSize": 12,
            "lineHeight": 18,
            "fontWeight": 500
          },
          "label": {
            "fontSize": 13,
            "lineHeight": 20,
            "fontWeight": 600
          },
          "body": {
            "fontSize": 15,
            "lineHeight": 22,
            "fontWeight": 400
          },
          "title": {
            "fontSize": 16,
            "lineHeight": 24,
            "fontWeight": 600
          },
          "display": {
            "fontSize": 32,
            "lineHeight": 40,
            "fontWeight": 700
          }
        },
        "shape": {
          "small": 6,
          "medium": 8,
          "large": 10,
          "control": 10,
          "panel": 14,
          "pill": 999
        },
        "geometry": {
          "minimumTarget": 44,
          "comfortableTarget": 48
        },
        "motion": {
          "standard": {
            "enabled": true,
            "fastDurationMs": 120,
            "standardDurationMs": 200,
            "slowDurationMs": 300
          },
          "reduced": {
            "enabled": false,
            "fastDurationMs": 0,
            "standardDurationMs": 0,
            "slowDurationMs": 0
          }
        },
        "fontPolicy": {
          "family": "system",
          "allowFontScaling": true,
          "respectBoldText": true,
          "minimumContrastRatio": 4.5
        }
      },
      "dark": {
        "identity": {
          "visualTheme": "apple-like",
          "colorScheme": "dark"
        },
        "color": {
          "action": "#27c9ff",
          "onAction": "#09090b",
          "canvas": "#071019",
          "surface": "rgba(24, 30, 37, 0.82)",
          "subtle": "rgba(42, 48, 58, 0.64)",
          "elevated": "rgba(48, 55, 66, 0.76)",
          "intelligence": "rgba(24, 30, 37, 0.82)",
          "text": "#f4f4f5",
          "muted": "#a1a1aa",
          "inverse": "#09090b",
          "border": "rgba(87, 96, 112, 0.66)",
          "accent": "rgba(39, 201, 255, 0.38)",
          "focus": "rgba(39, 201, 255, 0.2)",
          "active": "#009ff5",
          "success": "#66dfb2",
          "warning": "#ffc45c",
          "danger": "#ff7a95"
        },
        "spacing": {
          "4": 4,
          "8": 8,
          "12": 12,
          "16": 16,
          "20": 20,
          "24": 24,
          "32": 32
        },
        "typography": {
          "caption": {
            "fontSize": 12,
            "lineHeight": 18,
            "fontWeight": 500
          },
          "label": {
            "fontSize": 13,
            "lineHeight": 20,
            "fontWeight": 600
          },
          "body": {
            "fontSize": 15,
            "lineHeight": 22,
            "fontWeight": 400
          },
          "title": {
            "fontSize": 16,
            "lineHeight": 24,
            "fontWeight": 600
          },
          "display": {
            "fontSize": 32,
            "lineHeight": 40,
            "fontWeight": 700
          }
        },
        "shape": {
          "small": 6,
          "medium": 8,
          "large": 10,
          "control": 10,
          "panel": 14,
          "pill": 999
        },
        "geometry": {
          "minimumTarget": 44,
          "comfortableTarget": 48
        },
        "motion": {
          "standard": {
            "enabled": true,
            "fastDurationMs": 120,
            "standardDurationMs": 200,
            "slowDurationMs": 300
          },
          "reduced": {
            "enabled": false,
            "fastDurationMs": 0,
            "standardDurationMs": 0,
            "slowDurationMs": 0
          }
        },
        "fontPolicy": {
          "family": "system",
          "allowFontScaling": true,
          "respectBoldText": true,
          "minimumContrastRatio": 4.5
        }
      }
    }
  },
  "platforms": {
    "native-ios": {
      "visualTheme": "apple-like",
      "qualification": "not-qualified"
    },
    "native-android": {
      "visualTheme": null,
      "qualification": "not-qualified"
    }
  },
  "sources": {
    "color.action": {
      "light": "--pds-color-brand-blue-deep",
      "dark": "--pds-color-brand-blue-bright"
    },
    "color.onAction": {
      "light": "--pds-color-text-on-accent",
      "dark": "--pds-color-text-inverse"
    },
    "color.canvas": "--pds-color-surface-canvas-start",
    "color.surface": "--pds-color-surface-panel",
    "color.subtle": "--pds-color-surface-panel-soft",
    "color.elevated": "--pds-color-surface-elevated",
    "color.intelligence": "--pds-color-intelligence-surface",
    "color.text": "--pds-color-text-default",
    "color.muted": "--pds-color-text-muted",
    "color.inverse": "--pds-color-text-inverse",
    "color.border": "--pds-color-border-default",
    "color.accent": "--pds-color-border-accent",
    "color.focus": "--pds-color-state-focus",
    "color.active": "--pds-color-signal-blue",
    "color.success": "--pds-color-state-success",
    "color.warning": "--pds-color-state-warning",
    "color.danger": "--pds-color-state-danger",
    "spacing": {
      "4": "--pds-space-1",
      "8": "--pds-space-2",
      "12": "--pds-space-3",
      "16": "--pds-space-4",
      "20": "--pds-space-5",
      "24": "--pds-space-6",
      "32": "--pds-space-8"
    },
    "shape": {
      "small": "--pds-radius-sm",
      "medium": "--pds-radius-md",
      "large": "--pds-radius-lg",
      "control": "--pds-radius-control",
      "panel": "--pds-radius-panel",
      "pill": "--pds-radius-pill"
    },
    "typography": {
      "caption": {
        "fontSize": "--pds-font-size-sm",
        "lineHeight": "pds.native.typography.caption.lineHeight",
        "fontWeight": "--pds-font-weight-medium"
      },
      "label": {
        "fontSize": "--pds-font-size-md",
        "lineHeight": "pds.native.typography.label.lineHeight",
        "fontWeight": "--pds-font-weight-semibold"
      },
      "body": {
        "fontSize": "--pds-font-size-base",
        "lineHeight": "pds.native.typography.body.lineHeight",
        "fontWeight": "--pds-font-weight-regular"
      },
      "title": {
        "fontSize": "--pds-font-size-lg",
        "lineHeight": "pds.native.typography.title.lineHeight",
        "fontWeight": "--pds-font-weight-semibold"
      },
      "display": {
        "fontSize": "--pds-font-size-metric",
        "lineHeight": "pds.native.typography.display.lineHeight",
        "fontWeight": "--pds-font-weight-bold"
      }
    },
    "geometry": "pds.native.geometry",
    "motion": {
      "fastDurationMs": "--pds-motion-duration-fast",
      "standardDurationMs": "--pds-motion-duration-standard",
      "slowDurationMs": "--pds-motion-duration-slow",
      "reduced": "pds.native.motion.reduced"
    },
    "fontPolicy": "pds.native.fontPolicy",
    "visualThemes": "pds.native.visualThemes",
    "platforms": "pds.native.platforms"
  },
  "projectionSha256": "sha256:e1c86a0e269fa3be4dce7abfa35fe1033e9ae391daac030ad1dcf756e8d88a4d"
} as const;

export type PdsNativeVisualTheme = keyof typeof pdsNativeDesignData.visualThemes;
export type PdsNativeColorScheme = keyof (typeof pdsNativeDesignData.visualThemes)[PdsNativeVisualTheme];
export type PdsNativeTokenSelection = { readonly visualTheme?: PdsNativeVisualTheme; readonly colorScheme?: PdsNativeColorScheme };
export type PdsNativeTokens = (typeof pdsNativeDesignData.visualThemes)[PdsNativeVisualTheme][PdsNativeColorScheme];

export function pdsNativeTokensFor({ visualTheme = "apple-like", colorScheme = "light" }: PdsNativeTokenSelection = {}): PdsNativeTokens {
  return pdsNativeDesignData.visualThemes[visualTheme][colorScheme];
}
