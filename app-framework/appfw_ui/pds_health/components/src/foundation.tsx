import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type AnchorHTMLAttributes,
  type ButtonHTMLAttributes,
  type HTMLAttributes,
  type ReactNode,
  type RefObject,
  type SVGAttributes
} from "react";
import OverflowTooltip from "./overflow-tooltip";
import { composeClassNames, type PdsDensity, type PdsSize } from "./types";

export type PdsHealthLogoProps = Omit<
  SVGAttributes<SVGSVGElement>,
  "aria-label" | "children" | "role" | "viewBox"
> & {
  /** The horizontal wordmark is preferred; the mark is for constrained chrome. */
  variant?: "wordmark" | "mark";
  /** Removes the logo from the accessibility tree when an adjacent label names it. */
  decorative?: boolean;
  /** Accessible name for a meaningful logo. */
  label?: string;
};

const PDS_HEALTH_MARK_PATHS = [
  ["ink", "M24.82,11.32c.1,0,.19.08.23.23.05.35.34,2.02.54,3.13.2,1.09,2.08,1.23,2.35-.14.04-.21.08-.48.13-.79l1.15-9.27c.19-1.42.25-2.7-.44-3.34C27.75.22,25.96,0,24.82,0c-1.16,0-2.94.21-3.97,1.15-.68.64-.63,1.92-.44,3.34l1.16,9.27c.05.31.09.58.13.79.25,1.37,2.14,1.23,2.34.14.21-1.11.5-2.78.55-3.13.02-.13.09-.23.23-.23"],
  ["ink", "M38.26,24.84c0,.11-.09.21-.24.23-.36.05-2.01.35-3.13.56-1.09.21-1.22,2.11.15,2.35.21.03.48.08.78.13l9.24,1.14c1.41.17,2.7.23,3.33-.47.93-1.02,1.13-2.81,1.13-3.96s-.23-2.95-1.17-3.97c-.64-.7-1.91-.62-3.33-.45l-9.24,1.2c-.3.05-.58.09-.79.13-1.37.26-1.22,2.15-.13,2.35,1.11.2,2.78.48,3.13.54.13.03.24.08.24.23"],
  ["ink", "M11.28,24.83c0,.1.08.21.23.23.36.05,2.02.34,3.13.56,1.09.21,1.23,2.11-.15,2.35-.21.03-.47.08-.78.12l-9.24,1.14c-1.41.17-2.7.23-3.33-.47C.21,27.73,0,25.94,0,24.8c0-1.17.23-2.95,1.17-3.97.64-.7,1.92-.62,3.33-.44l9.24,1.21c.31.05.58.09.78.13,1.37.27,1.22,2.15.13,2.35-1.11.2-2.78.48-3.13.54-.14.03-.23.09-.23.23"],
  ["ink", "M15.24,15.23c.07-.07.2-.09.32,0,.29.21,1.67,1.19,2.6,1.82.91.63,2.35-.61,1.56-1.76-.12-.17-.28-.39-.46-.64l-5.72-7.38c-.87-1.11-1.74-2.08-2.68-2.03-1.39.07-2.79,1.17-3.61,1.99-.82.83-1.92,2.24-1.98,3.62-.05.95.91,1.81,2.04,2.68l7.37,5.73c.25.18.46.34.64.46,1.15.8,2.39-.64,1.76-1.56-.64-.93-1.62-2.32-1.83-2.6-.08-.11-.1-.23,0-.33"],
  ["accent", "M34.32,15.23c-.07-.07-.19-.09-.32.01-.28.21-1.67,1.19-2.6,1.82-.92.63-2.35-.62-1.56-1.76.12-.17.28-.39.46-.64l5.73-7.38c.87-1.12,1.74-2.08,2.68-2.03,1.38.07,2.78,1.18,3.6,1.99.82.83,1.93,2.24,1.99,3.62.04.94-.93,1.81-2.04,2.68l-7.37,5.73c-.25.18-.46.34-.64.46-1.15.8-2.39-.64-1.76-1.56.65-.93,1.62-2.32,1.83-2.6.09-.12.11-.23,0-.33"],
  ["ink", "M24.81,38.24c.1,0,.19-.08.23-.23.05-.36.34-2.03.55-3.14.21-1.09,2.08-1.23,2.35.15.04.21.08.47.13.78l1.15,9.27c.18,1.42.25,2.7-.44,3.35-1.03.93-2.81,1.15-3.96,1.15s-2.94-.22-3.97-1.15c-.68-.64-.63-1.93-.44-3.35l1.17-9.27c.05-.31.09-.57.13-.78.26-1.37,2.14-1.23,2.35-.15.21,1.12.49,2.78.55,3.14.02.13.09.23.23.23"],
  ["ink", "M15.24,34.34c.08.07.2.09.32-.01.29-.21,1.66-1.19,2.6-1.83.91-.62,2.35.62,1.56,1.76-.13.18-.28.39-.46.65l-5.72,7.38c-.87,1.11-1.74,2.07-2.68,2.03-1.38-.07-2.79-1.19-3.61-1.99-.82-.83-1.92-2.23-1.98-3.62-.05-.94.91-1.81,2.04-2.68l7.37-5.73c.25-.19.46-.34.64-.46,1.15-.8,2.39.64,1.76,1.56-.64.92-1.62,2.31-1.83,2.6-.08.12-.1.24,0,.33"],
  ["ink", "M34.32,34.34c-.07.07-.19.09-.32-.01-.28-.21-1.67-1.19-2.6-1.83-.92-.62-2.35.62-1.56,1.77.12.18.28.4.46.65l5.72,7.38c.87,1.12,1.74,2.08,2.68,2.03,1.38-.07,2.78-1.18,3.6-1.99.81-.83,1.93-2.23,1.99-3.62.04-.94-.92-1.81-2.04-2.68l-7.37-5.73c-.25-.19-.46-.34-.64-.46-1.15-.8-2.39.64-1.76,1.56.65.92,1.62,2.31,1.83,2.6.09.12.11.24,0,.33"]
] as const;

const PDS_HEALTH_WORDMARK_PATHS = [
  ["ink", "M64.71,11.45c0-.21.17-.37.37-.37h10.42c6.37,0,10.4,3.64,10.4,9.15v.08c0,6.14-4.93,9.35-10.95,9.35h-5.07c-.21,0-.37.17-.37.37v8.06c0,.21-.17.37-.37.37h-4.08c-.21,0-.37-.17-.37-.37V11.46h.01ZM75.1,25.32c3.64,0,5.9-2.03,5.9-4.89v-.08c0-3.21-2.31-4.89-5.9-4.89h-5.22c-.21,0-.37.17-.37.37v9.12c0,.21.17.37.37.37h5.22Z"],
  ["ink", "M88.68,11.45c0-.21.17-.37.37-.37h9.84c8.6,0,14.55,5.9,14.55,13.61v.08c0,7.71-5.94,13.69-14.55,13.69h-9.84c-.21,0-.37-.17-.37-.37V11.45ZM98.88,34.08c5.75,0,9.5-3.87,9.5-9.23v-.08c0-5.36-3.76-9.31-9.5-9.31h-5.03c-.21,0-.37.17-.37.37v17.88c0,.21.17.37.37.37h5.03Z"],
  ["ink", "M115.12,34.19l2.42-2.88c.13-.15.36-.17.52-.04,2.52,2.12,5.08,3.32,8.36,3.32,2.98,0,4.85-1.37,4.85-3.44v-.08c0-1.95-1.09-3.01-6.18-4.19-5.82-1.4-9.11-3.13-9.11-8.18v-.08c0-4.69,3.91-7.94,9.35-7.94,3.86,0,6.94,1.13,9.65,3.22.16.12.19.35.07.51l-2.16,3.05c-.12.17-.35.2-.52.08-2.37-1.7-4.73-2.6-7.13-2.6-2.82,0-4.46,1.44-4.46,3.25v.08c0,2.11,1.25,3.05,6.49,4.3,5.78,1.4,8.8,3.48,8.8,8.02v.08c0,5.12-4.03,8.18-9.77,8.18-4.07,0-7.91-1.37-11.15-4.13-.15-.13-.17-.37-.04-.52h0Z"],
  ["ink", "M151.77,11.09h4.08c.21,0,.37.17.37.37v10.64c0,.21.17.37.37.37h12.32c.21,0,.37-.17.37-.37v-10.64c0-.21.17-.37.37-.37h4.08c.21,0,.37.17.37.37v26.64c0,.21-.17.37-.37.37h-4.08c-.21,0-.37-.17-.37-.37v-10.8c0-.21-.17-.37-.37-.37h-12.32c-.21,0-.37.17-.37.37v10.8c0,.21-.17.37-.37.37h-4.08c-.21,0-.37-.17-.37-.37V11.46c0-.21.17-.37.37-.37Z"],
  ["ink", "M177.29,28.21v-.44c0-5.65,4.51-10.42,10.16-10.39,6.53.03,9.83,5.18,9.83,11.11,0,.32-.02.65-.05.99-.01.19-.18.34-.37.34h-14.39c-.23,0-.41.21-.36.44.64,3.09,2.95,4.8,5.86,4.8,2.19,0,3.78-.78,5.35-2.23.14-.13.35-.14.5-.01l2.25,2c.15.13.17.37.03.52-1.93,2.19-4.57,3.59-8.21,3.59-5.98,0-10.6-4.34-10.6-10.71h0ZM192.16,26.69c.23,0,.4-.2.37-.42-.43-2.85-2.21-5.02-5.19-5.02-2.76,0-4.74,2.03-5.29,5-.04.23.14.43.36.43h9.74Z"],
  ["ink", "M213.43,38.09v-1.25c0-.32-.39-.49-.62-.26-1.39,1.35-3.37,2.31-6.06,2.31-3.91,0-7.36-2.23-7.36-6.37v-.08c0-4.58,3.56-6.77,8.37-6.77,2.5,0,4.11.35,5.71.86v-.39c0-2.86-1.8-4.42-5.08-4.42-2.16,0-3.8.44-5.54,1.14-.2.08-.42-.02-.49-.22l-1.05-3.1c-.06-.18.03-.38.21-.46,2.16-.94,4.34-1.58,7.55-1.58,6.1,0,9.07,3.21,9.07,8.72v11.87c0,.21-.17.37-.37.37h-3.96c-.21,0-.37-.17-.37-.37h0ZM213.54,30.08c0-.16-.09-.3-.24-.35-1.19-.42-2.77-.74-4.49-.74-2.98,0-4.73,1.21-4.73,3.21v.08c0,1.95,1.76,3.05,3.99,3.05,3.13,0,5.47-1.76,5.47-4.34v-.91h0Z"],
  ["ink", "M222.25,9.91h4c.21,0,.37.17.37.37v27.81c0,.21-.17.37-.37.37h-4c-.21,0-.37-.17-.37-.37V10.28c0-.21.17-.37.37-.37Z"],
  ["ink", "M232.07,32.75v-10.5c0-.21-.17-.37-.37-.37h-1.88c-.21,0-.37-.17-.37-.37v-3.33c0-.21.17-.37.37-.37h1.88c.21,0,.37-.17.37-.37v-4.93c0-.21.17-.37.37-.37h4c.21,0,.37.17.37.37v4.93c0,.21.17.37.37.37h4.82c.21,0,.37.17.37.37v3.33c0,.21-.17.37-.37.37h-4.82c-.21,0-.37.17-.37.37v9.76c0,1.84.94,2.58,2.54,2.58.87,0,1.66-.16,2.44-.48.24-.1.5.09.5.34v3.07c0,.14-.07.27-.19.33-1.13.6-2.41.95-4.07.95-3.48,0-5.94-1.52-5.94-6.06h-.01Z"],
  ["ink", "M245.79,9.91h4c.21,0,.37.17.37.37v9.63c0,.35.43.5.66.23,1.29-1.56,3.08-2.76,5.8-2.76,4.65,0,7.36,3.13,7.36,7.94v12.77c0,.21-.17.37-.37.37h-4c-.21,0-.37-.17-.37-.37v-11.36c0-3.21-1.6-5.04-4.42-5.04s-4.65,1.91-4.65,5.12v11.28c0,.21-.17.37-.37.37h-4c-.21,0-.37-.17-.37-.37V10.28c0-.21.17-.37.37-.37Z"],
  ["ink", "M263.14,11.68h0c0-1.02.82-1.87,1.85-1.87s1.85.83,1.85,1.85h0c0,1.02-.82,1.87-1.85,1.87s-1.85-.83-1.85-1.85ZM266.61,11.67h0c0-.91-.7-1.66-1.63-1.66s-1.64.76-1.64,1.66h0c0,.91.7,1.66,1.64,1.66s1.63-.76,1.63-1.66ZM264.22,10.67h.9c.44,0,.78.2.78.64,0,.3-.17.51-.42.6l.49.69h-.56l-.41-.6h-.3v.6h-.47v-1.92h0ZM265.09,11.6c.21,0,.33-.11.33-.27,0-.17-.13-.27-.33-.27h-.4v.53s.4,0,.4,0Z"]
] as const;

/** Approved PDS Health identity artwork, preserved as responsive vector paths. */
export function PdsHealthLogo({
  variant = "wordmark",
  decorative = false,
  label = "PDS Health",
  className,
  ...props
}: PdsHealthLogoProps) {
  const paths = variant === "wordmark"
    ? [...PDS_HEALTH_MARK_PATHS, ...PDS_HEALTH_WORDMARK_PATHS]
    : PDS_HEALTH_MARK_PATHS;

  return (
    <svg
      {...props}
      className={composeClassNames("pds-health-logo", className)}
      data-variant={variant}
      viewBox={variant === "wordmark" ? "0 0 266.83 49.56" : "0 0 49.56 49.56"}
      preserveAspectRatio="xMinYMid meet"
      role={decorative ? undefined : "img"}
      aria-label={decorative ? undefined : label}
      aria-hidden={decorative || undefined}
      focusable="false"
    >
      {paths.map(([tone, d], index) => (
        <path
          key={`${tone}-${index}`}
          className={`pds-health-logo__${tone}`}
          d={d}
        />
      ))}
    </svg>
  );
}

export type IconSlotProps = HTMLAttributes<HTMLSpanElement> & {
  children: ReactNode;
  label?: string;
  size?: PdsSize;
};

export function IconSlot({
  children,
  label,
  size = "md",
  className,
  ...props
}: IconSlotProps) {
  return (
    <span
      {...props}
      className={composeClassNames("pds-icon-slot", className)}
      data-size={size}
      role={label ? "img" : undefined}
      aria-label={label}
      aria-hidden={label ? undefined : true}
    >
      {children}
    </span>
  );
}

type NavigationItemSharedProps = {
  label: ReactNode;
  icon?: ReactNode;
  description?: ReactNode;
  trailing?: ReactNode;
  current?: boolean;
  disabled?: boolean;
  density?: PdsDensity;
  labelBehavior?: "wrap" | "truncate";
  overflowTooltip?: ReactNode | false;
  className?: string;
};

export type NavigationItemLinkProps = NavigationItemSharedProps &
  Omit<
    AnchorHTMLAttributes<HTMLAnchorElement>,
    "aria-current" | "children" | "className" | "href"
  > & {
    href: string;
  };

export type NavigationItemButtonProps = NavigationItemSharedProps &
  Omit<
    ButtonHTMLAttributes<HTMLButtonElement>,
    "children" | "className" | "disabled"
  > & {
    href?: never;
  };

export type NavigationItemProps =
  | NavigationItemLinkProps
  | NavigationItemButtonProps;

function NavigationItemContent({
  icon,
  label,
  description,
  trailing,
  labelRef
}: Pick<
  NavigationItemSharedProps,
  "description" | "icon" | "label" | "trailing"
> & {
  labelRef: RefObject<HTMLSpanElement>;
}) {
  return (
    <>
      <span className="pds-navigation-item__icon" aria-hidden="true">
        {icon ? <IconSlot>{icon}</IconSlot> : null}
      </span>
      <span className="pds-navigation-item__copy">
        <span ref={labelRef} className="pds-navigation-item__label">
          {label}
        </span>
        {description ? (
          <span className="pds-navigation-item__description">{description}</span>
        ) : null}
      </span>
      {trailing ? (
        <span className="pds-navigation-item__trailing">{trailing}</span>
      ) : null}
    </>
  );
}

export function NavigationItem(props: NavigationItemProps) {
  const {
    icon,
    label,
    description,
    trailing,
    current = false,
    disabled = false,
    density = "comfortable",
    labelBehavior = "wrap",
    overflowTooltip = label,
    className,
    ...nativeProps
  } = props;
  const labelRef = useRef<HTMLSpanElement>(null);
  const content = (
    <NavigationItemContent
      icon={icon}
      label={label}
      description={description}
      trailing={trailing}
      labelRef={labelRef}
    />
  );
  const sharedProps = {
    className: composeClassNames("pds-navigation-item", className),
    "data-current": current || undefined,
    "data-density": density,
    "data-label-behavior": labelBehavior,
    "data-disabled": disabled || undefined
  };

  if ("href" in nativeProps && typeof nativeProps.href === "string") {
    const {
      href,
      onClick,
      tabIndex,
      ...anchorProps
    } = nativeProps as Omit<
      NavigationItemLinkProps,
      keyof NavigationItemSharedProps
    >;
    const link = (
      <a
        {...anchorProps}
        {...sharedProps}
        href={href}
        aria-current={current ? "page" : undefined}
        aria-disabled={disabled || undefined}
        tabIndex={disabled ? -1 : tabIndex}
        onClick={(event) => {
          if (disabled) {
            event.preventDefault();
            return;
          }
          onClick?.(event);
        }}
      >
        {content}
      </a>
    );
    return (
      <OverflowTooltip
        content={overflowTooltip === false ? label : overflowTooltip}
        overflowTargetRef={labelRef}
        disabled={overflowTooltip === false}
      >
        {link}
      </OverflowTooltip>
    );
  }

  const {
    href: _href,
    type,
    ...buttonProps
  } = nativeProps as Omit<
    NavigationItemButtonProps,
    keyof NavigationItemSharedProps
  >;
  const button = (
    <button
      {...buttonProps}
      {...sharedProps}
      type={type ?? "button"}
      disabled={disabled}
      aria-current={current ? "page" : undefined}
    >
      {content}
    </button>
  );
  return (
    <OverflowTooltip
      content={overflowTooltip === false ? label : overflowTooltip}
      overflowTargetRef={labelRef}
      disabled={overflowTooltip === false}
    >
      {button}
    </OverflowTooltip>
  );
}

function initialsForName(name: string) {
  const parts = name
    .trim()
    .split(/\s+/)
    .filter(Boolean);
  if (!parts.length) return "?";
  return (parts.length === 1
    ? parts[0].slice(0, 2)
    : `${parts[0][0]}${parts[parts.length - 1][0]}`
  ).toUpperCase();
}

export type AvatarProps = HTMLAttributes<HTMLSpanElement> & {
  name: string;
  src?: string;
  initials?: string;
  fallback?: ReactNode;
  size?: PdsSize;
  decorative?: boolean;
};

export function Avatar({
  name,
  src,
  initials,
  fallback,
  size = "md",
  decorative = false,
  className,
  ...props
}: AvatarProps) {
  return (
    <span
      {...props}
      className={composeClassNames("pds-avatar", className)}
      data-size={size}
      data-has-image={Boolean(src) || undefined}
      role={decorative ? undefined : "img"}
      aria-label={decorative ? undefined : name}
      aria-hidden={decorative || undefined}
    >
      {src ? (
        <img className="pds-avatar__image" src={src} alt="" />
      ) : (
        <span className="pds-avatar__fallback" aria-hidden="true">
          {fallback ?? initials ?? initialsForName(name)}
        </span>
      )}
    </span>
  );
}

export type IdentitySummaryProps = HTMLAttributes<HTMLDivElement> & {
  name: string;
  description?: ReactNode;
  metadata?: ReactNode;
  avatar?: ReactNode;
  trailing?: ReactNode;
  density?: PdsDensity;
};

export function IdentitySummary({
  name,
  description,
  metadata,
  avatar,
  trailing,
  density = "comfortable",
  className,
  ...props
}: IdentitySummaryProps) {
  return (
    <div
      {...props}
      className={composeClassNames("pds-identity-summary", className)}
      data-density={density}
    >
      <span className="pds-identity-summary__avatar" aria-hidden="true">
        {avatar ?? <Avatar name={name} decorative />}
      </span>
      <span className="pds-identity-summary__copy">
        <strong className="pds-identity-summary__name">{name}</strong>
        {description ? (
          <span className="pds-identity-summary__description">
            {description}
          </span>
        ) : null}
        {metadata ? (
          <span className="pds-identity-summary__metadata">{metadata}</span>
        ) : null}
      </span>
      {trailing ? (
        <span className="pds-identity-summary__trailing">{trailing}</span>
      ) : null}
    </div>
  );
}

export type AppearanceColorMode = "system" | "light" | "dark";
export type ResolvedAppearanceColorMode = "light" | "dark";
export type AppearanceVisualTheme = "apple-like" | "material-like";

type StoredAppearance = {
  colorMode?: AppearanceColorMode;
  visualTheme?: AppearanceVisualTheme;
};

export type AppearanceContextValue = {
  colorMode: AppearanceColorMode;
  resolvedColorMode: ResolvedAppearanceColorMode;
  visualTheme: AppearanceVisualTheme;
  setColorMode: (colorMode: AppearanceColorMode) => void;
  setVisualTheme: (visualTheme: AppearanceVisualTheme) => void;
  resetAppearance: () => void;
};

export type AppearanceProviderProps = {
  children: ReactNode;
  colorMode?: AppearanceColorMode;
  defaultColorMode?: AppearanceColorMode;
  visualTheme?: AppearanceVisualTheme;
  defaultVisualTheme?: AppearanceVisualTheme;
  onColorModeChange?: (colorMode: AppearanceColorMode) => void;
  onVisualThemeChange?: (visualTheme: AppearanceVisualTheme) => void;
  storageKey?: string;
  persist?: boolean;
  attributeTarget?: HTMLElement | null;
};

const AppearanceContext = createContext<AppearanceContextValue | null>(null);

function isColorMode(value: unknown): value is AppearanceColorMode {
  return value === "system" || value === "light" || value === "dark";
}

function isVisualTheme(value: unknown): value is AppearanceVisualTheme {
  return value === "apple-like" || value === "material-like";
}

function readStoredAppearance(storageKey: string | undefined): StoredAppearance {
  if (!storageKey || typeof window === "undefined") return {};
  try {
    const value = JSON.parse(window.localStorage.getItem(storageKey) ?? "{}");
    return {
      colorMode: isColorMode(value.colorMode) ? value.colorMode : undefined,
      visualTheme: isVisualTheme(value.visualTheme)
        ? value.visualTheme
        : undefined
    };
  } catch {
    return {};
  }
}

function writeStoredAppearance(
  storageKey: string,
  appearance: Required<StoredAppearance>
) {
  try {
    window.localStorage.setItem(storageKey, JSON.stringify(appearance));
  } catch {
    // Keep the active session usable when storage is unavailable or blocked.
  }
}

function systemColorMode(): ResolvedAppearanceColorMode {
  if (
    typeof window !== "undefined"
    && window.matchMedia?.("(prefers-color-scheme: dark)").matches
  ) {
    return "dark";
  }
  return "light";
}

export function AppearanceProvider({
  children,
  colorMode: controlledColorMode,
  defaultColorMode = "system",
  visualTheme: controlledVisualTheme,
  defaultVisualTheme = "apple-like",
  onColorModeChange,
  onVisualThemeChange,
  storageKey,
  persist,
  attributeTarget
}: AppearanceProviderProps) {
  const shouldPersist = persist ?? Boolean(storageKey);
  const storedAppearance = useMemo(
    () => (shouldPersist ? readStoredAppearance(storageKey) : {}),
    [shouldPersist, storageKey]
  );
  const [uncontrolledColorMode, setUncontrolledColorMode] =
    useState<AppearanceColorMode>(
      () => storedAppearance.colorMode ?? defaultColorMode
    );
  const [uncontrolledVisualTheme, setUncontrolledVisualTheme] =
    useState<AppearanceVisualTheme>(
      () => storedAppearance.visualTheme ?? defaultVisualTheme
    );
  const [preferredColorMode, setPreferredColorMode] =
    useState<ResolvedAppearanceColorMode>(systemColorMode);
  const colorMode = controlledColorMode ?? uncontrolledColorMode;
  const visualTheme = controlledVisualTheme ?? uncontrolledVisualTheme;
  const resolvedColorMode =
    colorMode === "system" ? preferredColorMode : colorMode;

  useEffect(() => {
    if (typeof window === "undefined" || !window.matchMedia) return undefined;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const updatePreference = () => {
      setPreferredColorMode(media.matches ? "dark" : "light");
    };
    updatePreference();
    media.addEventListener?.("change", updatePreference);
    return () => media.removeEventListener?.("change", updatePreference);
  }, []);

  useEffect(() => {
    if (typeof document === "undefined") return undefined;
    const target = attributeTarget ?? document.documentElement;
    const previousTheme = target.getAttribute("data-theme");
    const previousVisualTheme = target.getAttribute("data-visual-theme");
    target.setAttribute("data-theme", resolvedColorMode);
    target.setAttribute("data-visual-theme", visualTheme);
    return () => {
      if (previousTheme === null) target.removeAttribute("data-theme");
      else target.setAttribute("data-theme", previousTheme);
      if (previousVisualTheme === null) {
        target.removeAttribute("data-visual-theme");
      } else {
        target.setAttribute("data-visual-theme", previousVisualTheme);
      }
    };
  }, [attributeTarget, resolvedColorMode, visualTheme]);

  useEffect(() => {
    if (!shouldPersist || !storageKey || typeof window === "undefined") return;
    writeStoredAppearance(storageKey, { colorMode, visualTheme });
  }, [colorMode, shouldPersist, storageKey, visualTheme]);

  const setColorMode = useCallback(
    (nextColorMode: AppearanceColorMode) => {
      if (controlledColorMode === undefined) {
        setUncontrolledColorMode(nextColorMode);
      }
      onColorModeChange?.(nextColorMode);
    },
    [controlledColorMode, onColorModeChange]
  );
  const setVisualTheme = useCallback(
    (nextVisualTheme: AppearanceVisualTheme) => {
      if (controlledVisualTheme === undefined) {
        setUncontrolledVisualTheme(nextVisualTheme);
      }
      onVisualThemeChange?.(nextVisualTheme);
    },
    [controlledVisualTheme, onVisualThemeChange]
  );
  const resetAppearance = useCallback(() => {
    setColorMode(defaultColorMode);
    setVisualTheme(defaultVisualTheme);
  }, [
    defaultColorMode,
    defaultVisualTheme,
    setColorMode,
    setVisualTheme
  ]);
  const contextValue = useMemo<AppearanceContextValue>(
    () => ({
      colorMode,
      resolvedColorMode,
      visualTheme,
      setColorMode,
      setVisualTheme,
      resetAppearance
    }),
    [
      colorMode,
      resetAppearance,
      resolvedColorMode,
      setColorMode,
      setVisualTheme,
      visualTheme
    ]
  );

  return (
    <AppearanceContext.Provider value={contextValue}>
      {children}
    </AppearanceContext.Provider>
  );
}

export function useAppearance() {
  const context = useContext(AppearanceContext);
  if (!context) {
    throw new Error("useAppearance must be used within an AppearanceProvider");
  }
  return context;
}
