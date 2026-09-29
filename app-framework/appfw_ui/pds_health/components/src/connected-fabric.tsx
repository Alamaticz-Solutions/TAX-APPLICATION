import { useEffect, useRef, type FC } from "react";
import type {
  AppearanceColorMode,
  AppearanceVisualTheme
} from "./foundation";

export type ConnectedFabricProps = {
  theme: AppearanceColorMode;
  visualTheme: AppearanceVisualTheme;
  contentRootSelector?: string;
  panelSelector?: string;
  scrollRootSelector?: string;
};

type MeshPointer = {
  x: number;
  y: number;
  targetX: number;
  targetY: number;
  strength: number;
  targetStrength: number;
};

type MeshSegment = {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
};

type MeshPoint = {
  x: number;
  y: number;
};

type MeshNode = {
  point: MeshPoint;
  neighbors: number[];
};

type MeshRoute = {
  points: MeshPoint[];
  cumulativeLengths: number[];
  edgeKeys: string[];
  length: number;
};

type MeshSignal = {
  kind: "ambient" | "hover";
  startNodeIndex: number;
  route: MeshRoute;
  startedAt: number;
  drawDuration: number;
  duration: number;
  paletteOffset: number;
  paletteDirection: 1 | -1;
  opacityScale: number;
  radius: number;
};

type HexMesh = {
  path: Path2D;
  segments: MeshSegment[];
  nodes: MeshNode[];
};

type MeshColors = {
  blue: string;
  blueBright: string;
  teal: string;
  violet: string;
};

const HEX_RADIUS = 54;
const MESH_ROTATION = (45 * Math.PI) / 180;
const FIELD_SIGMA = 130;
const CORE_SIGMA = 54;
const FIELD_CUTOFF = 480;
const LINE_GRADIENT_STEPS = 3;
const MESH_STROKE_WIDTH = 1.44;
const MAX_PIXEL_RATIO = 1.5;
const FRAME_INTERVAL = 15;
const AMBIENT_FRAME_INTERVAL = 32;
const AMBIENT_SIGNAL_LIMIT = 1;
const HOVER_SIGNAL_LIMIT = 4;
const AMBIENT_SIGNAL_DELAY_MIN = 10000;
const AMBIENT_SIGNAL_DELAY_RANGE = 20000;
const HOVER_SIGNAL_DELAY_MIN = 180;
const HOVER_SIGNAL_DELAY_RANGE = 280;
const HOVER_SIGNAL_SIGMA = 220;
const HOVER_LOCAL_SPAWN_SCALE = 0.75;
const SIGNAL_ROUTE_EDGE_MIN = 5;
const SIGNAL_ROUTE_EDGE_RANGE = 4;
const SIGNAL_ROUTE_BUILD_ATTEMPTS = 12;
const SIGNAL_ARRIVAL_PAUSE_DURATION = 160;
const SIGNAL_REFLECTION_DURATION = 520;
const SIGNAL_POST_REFLECTION_DURATION = 450;
const SIGNAL_FADE_DURATION = 1500;
const SIGNAL_ACTIVATION_DURATION = 120;
const SIGNAL_RING_APPEAR_DURATION = 140;
const HOVER_ATTACK_DURATION = 500;
const HOVER_IDLE_GRACE = 120;
const HOVER_DECAY_DURATION = 2500;
const FOCUS_SHADE_ENABLED = false;
const DEFAULT_PANEL_SELECTOR =
  ".component-card, .catalog__main > .pds-surface, .recipes__grid > .pds-surface";

function resolvedScheme(mode: AppearanceColorMode): "light" | "dark" {
  if (mode !== "system") return mode;
  if (typeof window === "undefined" || !window.matchMedia) return "light";
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

function segmentKey(x1: number, y1: number, x2: number, y2: number): string {
  const first = `${Math.round(x1 * 10)},${Math.round(y1 * 10)}`;
  const second = `${Math.round(x2 * 10)},${Math.round(y2 * 10)}`;
  return first < second ? `${first}:${second}` : `${second}:${first}`;
}

function pointKey(point: MeshPoint): string {
  return `${Math.round(point.x * 10)},${Math.round(point.y * 10)}`;
}

function meshEdgeKey(firstNodeIndex: number, secondNodeIndex: number): string {
  return firstNodeIndex < secondNodeIndex
    ? `${firstNodeIndex}:${secondNodeIndex}`
    : `${secondNodeIndex}:${firstNodeIndex}`;
}

function smootherStep(value: number): number {
  const bounded = Math.max(0, Math.min(1, value));
  return bounded * bounded * bounded * (bounded * (bounded * 6 - 15) + 10);
}

function easeInOutQuart(value: number): number {
  const bounded = Math.max(0, Math.min(1, value));
  return bounded < 0.5
    ? 8 * bounded * bounded * bounded * bounded
    : 1 - Math.pow(-2 * bounded + 2, 4) / 2;
}

function easeOutCubic(value: number): number {
  const bounded = Math.max(0, Math.min(1, value));
  return 1 - Math.pow(1 - bounded, 3);
}

function buildHexMesh(width: number, height: number): HexMesh {
  const segmentsByKey = new Map<string, MeshSegment>();
  const horizontalStep = HEX_RADIUS * 1.5;
  const verticalStep = Math.sqrt(3) * HEX_RADIUS;
  const meshPadding = HEX_RADIUS * 5;
  const minimumX = -meshPadding;
  const maximumX = width + meshPadding;
  const minimumY = -meshPadding;
  const maximumY = height + meshPadding;
  const firstColumn = Math.floor(minimumX / horizontalStep) - 1;
  const lastColumn = Math.ceil(maximumX / horizontalStep) + 1;
  const viewportCenterX = width / 2;
  const viewportCenterY = height / 2;
  const rotationCosine = Math.cos(MESH_ROTATION);
  const rotationSine = Math.sin(MESH_ROTATION);

  function rotatePoint(x: number, y: number): { x: number; y: number } {
    const offsetX = x - viewportCenterX;
    const offsetY = y - viewportCenterY;
    return {
      x: viewportCenterX + offsetX * rotationCosine - offsetY * rotationSine,
      y: viewportCenterY + offsetX * rotationSine + offsetY * rotationCosine
    };
  }

  for (let column = firstColumn; column <= lastColumn; column += 1) {
    const centerX = column * horizontalStep;
    const rowOffset = Math.abs(column % 2) * (verticalStep / 2);
    const firstRow = Math.floor((minimumY - rowOffset) / verticalStep) - 1;
    const lastRow = Math.ceil((maximumY - rowOffset) / verticalStep) + 1;

    for (let row = firstRow; row <= lastRow; row += 1) {
      const centerY = row * verticalStep + rowOffset;
      const vertices = Array.from({ length: 6 }, (_, index) => {
        const angle = (Math.PI / 3) * index;
        return rotatePoint(
          centerX + Math.cos(angle) * HEX_RADIUS,
          centerY + Math.sin(angle) * HEX_RADIUS
        );
      });
      vertices.forEach((start, index) => {
        const end = vertices[(index + 1) % vertices.length];
        const key = segmentKey(start.x, start.y, end.x, end.y);
        if (segmentsByKey.has(key)) return;
        segmentsByKey.set(key, {
          x1: start.x,
          y1: start.y,
          x2: end.x,
          y2: end.y
        });
      });
    }
  }

  const path = new Path2D();
  const segments = [...segmentsByKey.values()];
  const nodes: MeshNode[] = [];
  const nodeIndexes = new Map<string, number>();

  function nodeIndex(point: MeshPoint): number {
    const key = pointKey(point);
    const existingIndex = nodeIndexes.get(key);
    if (existingIndex !== undefined) return existingIndex;
    const index = nodes.length;
    nodeIndexes.set(key, index);
    nodes.push({ point, neighbors: [] });
    return index;
  }

  segments.forEach((segment) => {
    path.moveTo(segment.x1, segment.y1);
    path.lineTo(segment.x2, segment.y2);
    const startIndex = nodeIndex({ x: segment.x1, y: segment.y1 });
    const endIndex = nodeIndex({ x: segment.x2, y: segment.y2 });
    if (!nodes[startIndex].neighbors.includes(endIndex)) {
      nodes[startIndex].neighbors.push(endIndex);
    }
    if (!nodes[endIndex].neighbors.includes(startIndex)) {
      nodes[endIndex].neighbors.push(startIndex);
    }
  });

  return { path, segments, nodes };
}

function pointOnRoute(route: MeshRoute, distance: number): MeshPoint {
  const boundedDistance = Math.max(0, Math.min(route.length, distance));
  for (let index = 1; index < route.points.length; index += 1) {
    const segmentEnd = route.cumulativeLengths[index];
    if (boundedDistance > segmentEnd) continue;
    const segmentStart = route.cumulativeLengths[index - 1];
    const segmentLength = segmentEnd - segmentStart;
    const localProgress = segmentLength === 0
      ? 0
      : (boundedDistance - segmentStart) / segmentLength;
    const start = route.points[index - 1];
    const end = route.points[index];
    return {
      x: start.x + (end.x - start.x) * localProgress,
      y: start.y + (end.y - start.y) * localProgress
    };
  }
  return route.points[route.points.length - 1];
}

function routeSegmentIndex(route: MeshRoute, distance: number): number {
  const boundedDistance = Math.max(0, Math.min(route.length, distance));
  for (let index = 1; index < route.cumulativeLengths.length; index += 1) {
    if (boundedDistance <= route.cumulativeLengths[index]) return index - 1;
  }
  return Math.max(0, route.points.length - 2);
}

function drawMesh(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  mesh: HexMesh,
  pointer: MeshPointer,
  colors: MeshColors,
  dark: boolean,
  panelRegions: DOMRect[],
  activePanel: DOMRect | null,
  signals: MeshSignal[],
  time: number
): void {
  context.clearRect(0, 0, width, height);
  context.lineCap = "butt";
  context.lineJoin = "round";

  if (FOCUS_SHADE_ENABLED && pointer.strength >= 0.002) {
    const shadeRadius = FIELD_CUTOFF * 1.75;
    const focusShade = context.createRadialGradient(
      pointer.x,
      pointer.y,
      0,
      pointer.x,
      pointer.y,
      shadeRadius
    );
    focusShade.addColorStop(0, colors.blueBright);
    focusShade.addColorStop(0.18, colors.blue);
    focusShade.addColorStop(0.42, colors.violet);
    focusShade.addColorStop(1, "rgba(255, 255, 255, 0)");
    context.fillStyle = focusShade;
    context.globalAlpha = pointer.strength * (dark ? 0.016 : 0.022);
    context.fillRect(0, 0, width, height);
  }

  const fabricColor = context.createLinearGradient(width * 0.04, height * 0.08, width * 0.96, height * 0.92);
  fabricColor.addColorStop(0, colors.blueBright);
  fabricColor.addColorStop(0.3, colors.blue);
  fabricColor.addColorStop(0.63, colors.violet);
  fabricColor.addColorStop(1, colors.blue);

  context.strokeStyle = fabricColor;
  context.lineWidth = MESH_STROKE_WIDTH;
  context.globalAlpha = dark ? 0.0211 : 0.0196;
  context.stroke(mesh.path);

  function drawProximity(opacityScale: number): void {
    if (pointer.strength < 0.002) return;

    context.strokeStyle = fabricColor;

    mesh.segments.forEach((segment) => {
      for (let step = 0; step < LINE_GRADIENT_STEPS; step += 1) {
        const startProgress = step / LINE_GRADIENT_STEPS;
        const endProgress = (step + 1) / LINE_GRADIENT_STEPS;
        const midpointProgress = (startProgress + endProgress) / 2;
        const midpointX = segment.x1 + (segment.x2 - segment.x1) * midpointProgress;
        const midpointY = segment.y1 + (segment.y2 - segment.y1) * midpointProgress;
        const distance = Math.hypot(pointer.x - midpointX, pointer.y - midpointY);
        if (distance >= FIELD_CUTOFF) continue;

        const broad = Math.exp(-(distance * distance) / (2 * FIELD_SIGMA * FIELD_SIGMA));
        const core = Math.exp(-(distance * distance) / (2 * CORE_SIGMA * CORE_SIGMA));
        const influence = pointer.strength * (broad * 0.76 + core * 0.24);
        if (influence < 0.003) continue;

        const startX = segment.x1 + (segment.x2 - segment.x1) * startProgress;
        const startY = segment.y1 + (segment.y2 - segment.y1) * startProgress;
        const endX = segment.x1 + (segment.x2 - segment.x1) * endProgress;
        const endY = segment.y1 + (segment.y2 - segment.y1) * endProgress;

        context.lineWidth = MESH_STROKE_WIDTH;
        context.globalAlpha = influence * (dark ? 0.053 : 0.0455) * opacityScale;
        context.beginPath();
        context.moveTo(startX, startY);
        context.lineTo(endX, endY);
        context.stroke();
      }
    });
  }

  function drawSignals(): void {
    if (signals.length === 0) return;

    const signalPalette = [colors.blue, colors.teal, colors.violet];
    function paletteColor(signal: MeshSignal, colorStep: number): string {
      const paletteIndex =
        ((signal.paletteOffset + signal.paletteDirection * colorStep) %
          signalPalette.length +
          signalPalette.length) %
        signalPalette.length;
      return signalPalette[paletteIndex];
    }

    function drawRouteRange(
      signal: MeshSignal,
      startDistance: number,
      endDistance: number,
      lineWidth: number,
      opacity: number,
      additive: boolean,
      solidColor?: string
    ): void {
      context.save();
      context.globalAlpha = opacity;
      if (additive) context.globalCompositeOperation = "lighter";

      for (
        let segmentIndex = 0;
        segmentIndex < signal.route.points.length - 1;
        segmentIndex += 1
      ) {
        const segmentStart = signal.route.cumulativeLengths[segmentIndex];
        const segmentEnd = signal.route.cumulativeLengths[segmentIndex + 1];
        const visibleStart = Math.max(startDistance, segmentStart);
        const visibleEnd = Math.min(endDistance, segmentEnd);
        if (visibleEnd <= visibleStart) continue;

        const start = pointOnRoute(signal.route, visibleStart);
        const end = pointOnRoute(signal.route, visibleEnd);
        if (solidColor) {
          context.strokeStyle = solidColor;
        } else {
          const fullStart = signal.route.points[segmentIndex];
          const fullEnd = signal.route.points[segmentIndex + 1];
          const gradient = context.createLinearGradient(
            fullStart.x,
            fullStart.y,
            fullEnd.x,
            fullEnd.y
          );
          gradient.addColorStop(0, paletteColor(signal, segmentIndex));
          gradient.addColorStop(1, paletteColor(signal, segmentIndex + 1));
          context.strokeStyle = gradient;
        }
        context.lineWidth = lineWidth;
        context.beginPath();
        context.moveTo(start.x, start.y);
        context.lineTo(end.x, end.y);
        context.stroke();
      }
      context.restore();
    }

    function drawEndpointRing(
      point: MeshPoint,
      color: string,
      opacity: number,
      radius: number
    ): void {
      if (opacity <= 0) return;

      context.save();
      context.strokeStyle = color;
      context.globalAlpha = opacity * (dark ? 0.22 : 0.16);
      context.globalCompositeOperation = "lighter";
      context.lineWidth = Math.max(1.4, radius * 0.72);
      context.beginPath();
      context.arc(point.x, point.y, radius * 1.45, 0, Math.PI * 2);
      context.stroke();
      context.restore();

      context.save();
      context.strokeStyle = color;
      context.globalAlpha = opacity * (dark ? 0.78 : 0.68);
      context.lineWidth = Math.max(0.72, radius * 0.3);
      context.beginPath();
      context.arc(point.x, point.y, radius, 0, Math.PI * 2);
      context.stroke();
      context.restore();
    }

    context.save();
    context.lineCap = "round";
    context.lineJoin = "round";
    signals.forEach((signal) => {
      const age = time - signal.startedAt;
      if (age < 0 || age >= signal.duration) return;

      const arrivalEndsAt =
        signal.drawDuration + SIGNAL_ARRIVAL_PAUSE_DURATION;
      const reflectionEndsAt = arrivalEndsAt + SIGNAL_REFLECTION_DURATION;
      const fadeStartsAt =
        reflectionEndsAt + SIGNAL_POST_REFLECTION_DURATION;
      const drawing = age < signal.drawDuration;
      const fading = age >= fadeStartsAt;
      let visibleStart = 0;
      let visibleEnd = signal.route.length;
      if (drawing) {
        visibleEnd =
          signal.route.length * easeInOutQuart(age / signal.drawDuration);
      }
      if (visibleEnd <= visibleStart) return;

      const activation = easeOutCubic(age / SIGNAL_ACTIVATION_DURATION);
      const fadeProgress = fading
        ? (age - fadeStartsAt) / SIGNAL_FADE_DURATION
        : 0;
      const fadeOpacity = 1 - smootherStep(fadeProgress);
      const opacity = activation * signal.opacityScale * fadeOpacity;
      drawRouteRange(
        signal,
        visibleStart,
        visibleEnd,
        signal.radius * 2.5,
        opacity * (dark ? 0.085 : 0.06),
        true
      );
      drawRouteRange(
        signal,
        visibleStart,
        visibleEnd,
        Math.max(0.72, signal.radius * 0.9),
        opacity * (dark ? 0.34 : 0.27),
        false
      );

      if (age >= arrivalEndsAt && age < reflectionEndsAt) {
        const reflectionProgress =
          (age - arrivalEndsAt) / SIGNAL_REFLECTION_DURATION;
        const reflectionDistance =
          signal.route.length * (1 - easeInOutQuart(reflectionProgress));
        const reflectionEnvelope =
          smootherStep(reflectionProgress / 0.12) *
          (1 - smootherStep((reflectionProgress - 0.82) / 0.18));
        const reflectionRange = 22 + signal.radius * 4;

        drawRouteRange(
          signal,
          Math.max(0, reflectionDistance - reflectionRange),
          Math.min(signal.route.length, reflectionDistance + reflectionRange),
          signal.radius * 3.55,
          reflectionEnvelope * opacity * (dark ? 0.22 : 0.17),
          true,
          colors.blueBright
        );
        drawRouteRange(
          signal,
          Math.max(0, reflectionDistance - reflectionRange * 0.34),
          Math.min(
            signal.route.length,
            reflectionDistance + reflectionRange * 0.34
          ),
          Math.max(0.92, signal.radius * 1.18),
          reflectionEnvelope * opacity * (dark ? 0.9 : 0.76),
          false,
          colors.blueBright
        );
      }

      const firstPoint = signal.route.points[0];
      const lastPoint = signal.route.points[signal.route.points.length - 1];
      const sourceRingOpacity = activation * fadeOpacity;
      const destinationRingOpacity =
        age < signal.drawDuration
          ? 0
          : easeOutCubic(
              (age - signal.drawDuration) / SIGNAL_RING_APPEAR_DURATION
            ) * fadeOpacity;
      const ringRadius = 2.5 + signal.radius * 0.5;
      drawEndpointRing(
        firstPoint,
        paletteColor(signal, 0),
        sourceRingOpacity * signal.opacityScale,
        ringRadius
      );
      drawEndpointRing(
        lastPoint,
        paletteColor(signal, signal.route.points.length - 1),
        destinationRingOpacity * signal.opacityScale,
        ringRadius
      );

      if (visibleEnd > visibleStart) {
        const head = pointOnRoute(signal.route, visibleEnd);
        const headSegment = routeSegmentIndex(signal.route, visibleEnd);
        const headColor = paletteColor(signal, headSegment + 1);
        context.save();
        context.globalCompositeOperation = "lighter";
        context.fillStyle = headColor;
        context.globalAlpha = opacity * (dark ? 0.24 : 0.17);
        context.beginPath();
        context.arc(head.x, head.y, signal.radius * 2.7, 0, Math.PI * 2);
        context.fill();
        context.restore();

        context.fillStyle = headColor;
        context.globalAlpha = signal.kind === "ambient" ? 0.52 : 1;
        context.beginPath();
        context.arc(head.x, head.y, signal.radius * 1.12, 0, Math.PI * 2);
        context.fill();
      }
    });
    context.restore();
  }

  drawProximity(1);
  drawSignals();

  panelRegions.forEach((region) => {
    context.clearRect(region.left - 1, region.top - 1, region.width + 2, region.height + 2);
  });

  if (activePanel) {
    context.save();
    context.beginPath();
    context.roundRect(activePanel.left, activePanel.top, activePanel.width, activePanel.height, 8);
    context.clip();
    drawProximity(0.45);
    context.restore();
  }

  context.globalAlpha = 1;
}

export const ConnectedFabric: FC<ConnectedFabricProps> = ({
  theme,
  visualTheme,
  contentRootSelector = ".catalog__main",
  panelSelector = DEFAULT_PANEL_SELECTOR,
  scrollRootSelector
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return undefined;
    const context = canvas.getContext("2d");
    if (!context) return undefined;
    const canvasElement: HTMLCanvasElement = canvas;
    const drawingContext: CanvasRenderingContext2D = context;
    const generation = String(Number(canvasElement.dataset.meshGeneration || "0") + 1);
    canvasElement.dataset.meshGeneration = generation;
    let disposed = false;

    const motionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    const finePointerQuery = window.matchMedia("(hover: hover) and (pointer: fine)");
    const schemeQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const pointer: MeshPointer = {
      x: window.innerWidth / 2,
      y: window.innerHeight / 2,
      targetX: window.innerWidth / 2,
      targetY: window.innerHeight / 2,
      strength: 0,
      targetStrength: 0
    };

    let width = window.innerWidth;
    let height = window.innerHeight;
    let mesh = buildHexMesh(width, height);
    let frame = 0;
    let previousFrame = 0;
    let colors: MeshColors = {
      blue: "#009ff5",
      blueBright: "#27c9ff",
      teal: "#00d1c2",
      violet: "#7658fa"
    };
    let panelRegions: DOMRect[] = [];
    let activePanelElement: Element | null = null;
    let layoutFrame = 0;
    let pulseActive = false;
    let pulseStartedAt = 0;
    let pulseStartStrength = 0;
    let lastPointerActivityAt = 0;
    let signals: MeshSignal[] = [];
    let nextAmbientSignalAt = 0;
    let nextHoverSignalAt = 0;
    let randomState = ((width * 2654435761) ^ (height * 1597334677)) >>> 0;
    const contentRoot = document.querySelector(contentRootSelector);
    const scrollRoot = scrollRootSelector
      ? document.querySelector(scrollRootSelector)
      : null;

    function nextRandom(): number {
      randomState ^= randomState << 13;
      randomState ^= randomState >>> 17;
      randomState ^= randomState << 5;
      return (randomState >>> 0) / 4294967296;
    }

    function refreshColors(): void {
      const colorProbe = document.createElement("span");
      colorProbe.style.position = "fixed";
      colorProbe.style.pointerEvents = "none";
      colorProbe.style.visibility = "hidden";
      document.body.append(colorProbe);

      function resolveColor(token: string, fallback: string): string {
        colorProbe.style.color = `var(${token}, ${fallback})`;
        return getComputedStyle(colorProbe).color || fallback;
      }

      colors = {
        blue: resolveColor("--pds-color-signal-blue", "#009ff5"),
        blueBright: resolveColor("--pds-color-brand-blue-bright", "#27c9ff"),
        teal: resolveColor("--pds-color-signal-teal", "#00d1c2"),
        violet: resolveColor("--pds-color-signal-violet", "#7658fa")
      };
      colorProbe.remove();
    }

    function refreshPanelRegions(): void {
      panelRegions = [...document.querySelectorAll(panelSelector)]
        .map((element) => element.getBoundingClientRect())
        .filter((region) => region.bottom > 0 && region.top < height && region.right > 0 && region.left < width);
    }

    function activePanelRegion(): DOMRect | null {
      if (!activePanelElement?.isConnected) return null;
      const region = activePanelElement.getBoundingClientRect();
      return region.bottom > 0 && region.top < height ? region : null;
    }

    function draw(time = performance.now()): void {
      drawMesh(
        drawingContext,
        width,
        height,
        mesh,
        pointer,
        colors,
        resolvedScheme(theme) === "dark",
        panelRegions,
        activePanelRegion(),
        signals,
        time
      );
    }

    function resize(): void {
      width = window.innerWidth;
      height = window.innerHeight;
      const pixelRatio = Math.min(window.devicePixelRatio || 1, MAX_PIXEL_RATIO);
      canvasElement.width = Math.round(width * pixelRatio);
      canvasElement.height = Math.round(height * pixelRatio);
      drawingContext.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0);
      mesh = buildHexMesh(width, height);
      signals = [];
      nextAmbientSignalAt = 0;
      nextHoverSignalAt = 0;
      refreshPanelRegions();
      draw();
    }

    function selectSignalStartNode(
      hoverActivity: number,
      occupiedEdges: ReadonlySet<string>,
      attemptedStartNodes: ReadonlySet<number>
    ): number {
      const availableNodes = mesh.nodes
        .map((node, nodeIndex) => ({ node, nodeIndex }))
        .filter(
          ({ node, nodeIndex }) =>
            !attemptedStartNodes.has(nodeIndex) &&
            node.neighbors.length >= 2 &&
            node.neighbors.some(
              (neighborIndex) =>
                !occupiedEdges.has(meshEdgeKey(nodeIndex, neighborIndex))
            ) &&
            node.point.x >= 0 &&
            node.point.x <= width &&
            node.point.y >= 0 &&
            node.point.y <= height
        );
      const exposedNodes = availableNodes.filter(({ node }) =>
        panelRegions.every(
          (region) =>
            node.point.x < region.left ||
            node.point.x > region.right ||
            node.point.y < region.top ||
            node.point.y > region.bottom
        )
      );
      const candidates = exposedNodes.length > 0 ? exposedNodes : availableNodes;
      if (candidates.length === 0) return -1;

      const localSpawnChance =
        HOVER_LOCAL_SPAWN_SCALE *
        smootherStep(Math.min(1, hoverActivity * 1.15));
      if (hoverActivity < 0.02 || nextRandom() > localSpawnChance) {
        return candidates[Math.floor(nextRandom() * candidates.length)].nodeIndex;
      }

      const weights = candidates.map(({ node }) => {
        const distanceSquared =
          (pointer.x - node.point.x) ** 2 + (pointer.y - node.point.y) ** 2;
        return Math.exp(
          -distanceSquared / (2 * HOVER_SIGNAL_SIGMA * HOVER_SIGNAL_SIGMA)
        );
      });
      const totalWeight = weights.reduce((total, weight) => total + weight, 0);
      if (totalWeight <= 0) {
        return candidates[Math.floor(nextRandom() * candidates.length)].nodeIndex;
      }

      let selection = nextRandom() * totalWeight;
      for (let index = 0; index < candidates.length; index += 1) {
        selection -= weights[index];
        if (selection <= 0) return candidates[index].nodeIndex;
      }
      return candidates[candidates.length - 1].nodeIndex;
    }

    function buildSignalRoute(
      startNodeIndex: number,
      occupiedEdges: ReadonlySet<string>
    ): MeshRoute {
      const routeNodeIndexes = [startNodeIndex];
      const routeEdgeKeys: string[] = [];
      const unavailableEdges = new Set(occupiedEdges);
      const edgeCount =
        SIGNAL_ROUTE_EDGE_MIN + Math.floor(nextRandom() * SIGNAL_ROUTE_EDGE_RANGE);
      let currentNodeIndex = startNodeIndex;

      for (let edge = 0; edge < edgeCount; edge += 1) {
        const neighbors = mesh.nodes[currentNodeIndex]?.neighbors ?? [];
        const candidates = neighbors.filter(
          (neighborIndex) =>
            !unavailableEdges.has(
              meshEdgeKey(currentNodeIndex, neighborIndex)
            )
        );
        if (candidates.length === 0) break;

        const nextNodeIndex = candidates[Math.floor(nextRandom() * candidates.length)];
        const edgeKey = meshEdgeKey(currentNodeIndex, nextNodeIndex);
        routeNodeIndexes.push(nextNodeIndex);
        routeEdgeKeys.push(edgeKey);
        unavailableEdges.add(edgeKey);
        currentNodeIndex = nextNodeIndex;
      }

      const points = routeNodeIndexes.map((nodeIndex) => mesh.nodes[nodeIndex].point);
      const cumulativeLengths = [0];
      for (let index = 1; index < points.length; index += 1) {
        const previous = points[index - 1];
        const current = points[index];
        cumulativeLengths.push(
          cumulativeLengths[index - 1] +
            Math.hypot(current.x - previous.x, current.y - previous.y)
        );
      }
      return {
        points,
        cumulativeLengths,
        edgeKeys: routeEdgeKeys,
        length: cumulativeLengths[cumulativeLengths.length - 1]
      };
    }

    function spawnSignal(
      time: number,
      kind: MeshSignal["kind"],
      hoverActivity: number
    ): void {
      if (mesh.nodes.length === 0) return;

      const occupiedEdges = new Set(
        signals.flatMap(({ route }) => route.edgeKeys)
      );
      const attemptedStartNodes = new Set<number>();
      let startNodeIndex = -1;
      let route: MeshRoute | null = null;

      for (
        let attempt = 0;
        attempt < SIGNAL_ROUTE_BUILD_ATTEMPTS;
        attempt += 1
      ) {
        startNodeIndex = selectSignalStartNode(
          kind === "hover" ? hoverActivity : 0,
          occupiedEdges,
          attemptedStartNodes
        );
        if (startNodeIndex < 0) break;
        attemptedStartNodes.add(startNodeIndex);

        const candidateRoute = buildSignalRoute(startNodeIndex, occupiedEdges);
        if (candidateRoute.edgeKeys.length < SIGNAL_ROUTE_EDGE_MIN) continue;
        const candidateEdgeCount = new Set(candidateRoute.edgeKeys).size;
        const intersectsVisibleEdge = candidateRoute.edgeKeys.some((edgeKey) =>
          occupiedEdges.has(edgeKey)
        );
        if (
          candidateEdgeCount !== candidateRoute.edgeKeys.length ||
          intersectsVisibleEdge
        ) {
          continue;
        }
        route = candidateRoute;
        break;
      }

      if (!route || startNodeIndex < 0) return;

      const drawDuration = 1050 + nextRandom() * 750;
      signals.push({
        kind,
        startNodeIndex,
        route,
        startedAt: time,
        drawDuration,
        duration:
          drawDuration +
          SIGNAL_ARRIVAL_PAUSE_DURATION +
          SIGNAL_REFLECTION_DURATION +
          SIGNAL_POST_REFLECTION_DURATION +
          SIGNAL_FADE_DURATION,
        paletteOffset: Math.floor(nextRandom() * 3),
        paletteDirection: nextRandom() < 0.5 ? -1 : 1,
        opacityScale:
          kind === "ambient"
            ? 0.34 + nextRandom() * 0.16
            : 0.65 + nextRandom() * 0.35,
        radius: 0.92 + nextRandom() * 0.32
      });
    }

    function updateSignals(time: number): void {
      signals = signals.filter((signal) => time - signal.startedAt < signal.duration);

      if (nextAmbientSignalAt === 0) {
        nextAmbientSignalAt =
          time +
          AMBIENT_SIGNAL_DELAY_MIN +
          nextRandom() * AMBIENT_SIGNAL_DELAY_RANGE;
      }
      if (time >= nextAmbientSignalAt) {
        const ambientSignalCount = signals.filter(
          ({ kind }) => kind === "ambient"
        ).length;
        if (ambientSignalCount < AMBIENT_SIGNAL_LIMIT) {
          spawnSignal(time, "ambient", 0);
        }
        nextAmbientSignalAt =
          time +
          AMBIENT_SIGNAL_DELAY_MIN +
          nextRandom() * AMBIENT_SIGNAL_DELAY_RANGE;
      }

      const hoverActivity = smootherStep(pointer.strength);
      const hoverSignalLimit = Math.round(
        HOVER_SIGNAL_LIMIT * hoverActivity
      );
      if (hoverSignalLimit === 0) {
        nextHoverSignalAt = 0;
        return;
      }
      if (nextHoverSignalAt === 0) {
        nextHoverSignalAt =
          time +
          HOVER_SIGNAL_DELAY_MIN +
          nextRandom() * HOVER_SIGNAL_DELAY_RANGE;
      }
      if (time < nextHoverSignalAt) return;

      const hoverSignalCount = signals.filter(
        ({ kind }) => kind === "hover"
      ).length;
      if (hoverSignalCount < hoverSignalLimit) {
        spawnSignal(time, "hover", hoverActivity);
      }
      nextHoverSignalAt =
        time +
        HOVER_SIGNAL_DELAY_MIN +
        nextRandom() * HOVER_SIGNAL_DELAY_RANGE;
    }

    function hoverEnvelope(time: number): number {
      if (!pulseActive) return 0;

      const attackProgress = (time - pulseStartedAt) / HOVER_ATTACK_DURATION;
      const attackStrength =
        pulseStartStrength + (1 - pulseStartStrength) * smootherStep(attackProgress);
      const decayStartedAt = Math.max(
        pulseStartedAt + HOVER_ATTACK_DURATION,
        lastPointerActivityAt + HOVER_IDLE_GRACE
      );
      if (time <= decayStartedAt) return attackStrength;

      const decayProgress = (time - decayStartedAt) / HOVER_DECAY_DURATION;
      if (decayProgress >= 1) {
        pulseActive = false;
        return 0;
      }
      return 1 - smootherStep(decayProgress);
    }

    function render(time: number): void {
      if (disposed || canvasElement.dataset.meshGeneration !== generation) {
        frame = 0;
        return;
      }
      const positionMoving =
        Math.abs(pointer.targetX - pointer.x) >= 0.2 ||
        Math.abs(pointer.targetY - pointer.y) >= 0.2;
      const frameInterval = pulseActive || positionMoving ? FRAME_INTERVAL : AMBIENT_FRAME_INTERVAL;
      if (previousFrame !== 0 && time - previousFrame < frameInterval) {
        frame = window.requestAnimationFrame(render);
        return;
      }
      const elapsed = previousFrame === 0 ? 1000 / 60 : Math.min(time - previousFrame, 1000 / 30);
      previousFrame = time;

      const frameScale = elapsed / (1000 / 60);
      const positionBlend = 1 - Math.pow(0.91, frameScale);
      pointer.x += (pointer.targetX - pointer.x) * positionBlend;
      pointer.y += (pointer.targetY - pointer.y) * positionBlend;
      pointer.targetStrength = hoverEnvelope(time);
      pointer.strength = pointer.targetStrength;
      const ambientEnabled = !motionQuery.matches && finePointerQuery.matches && !document.hidden;
      canvasElement.dataset.motionState = ambientEnabled ? "responsive" : "static";
      if (ambientEnabled) updateSignals(time);
      else signals = [];
      const activeEdges = signals.flatMap(({ route }) => route.edgeKeys);
      const activeUniqueEdgeCount = new Set(activeEdges).size;
      canvasElement.dataset.activeCircuitCount = String(signals.length);
      canvasElement.dataset.activeEdgeCount = String(activeEdges.length);
      canvasElement.dataset.activeUniqueEdgeCount = String(activeUniqueEdgeCount);
      canvasElement.dataset.maxActiveCircuitCount = String(
        Math.max(
          Number(canvasElement.dataset.maxActiveCircuitCount ?? 0),
          signals.length
        )
      );
      canvasElement.dataset.maxActiveEdgeCount = String(
        Math.max(
          Number(canvasElement.dataset.maxActiveEdgeCount ?? 0),
          activeEdges.length
        )
      );
      if (activeEdges.length !== activeUniqueEdgeCount) {
        canvasElement.dataset.edgeConflictDetected = "true";
      }
      draw(time);

      const positionSettled =
        Math.abs(pointer.targetX - pointer.x) < 0.2 && Math.abs(pointer.targetY - pointer.y) < 0.2;
      if (!positionSettled || pulseActive || ambientEnabled) {
        frame = window.requestAnimationFrame(render);
      } else {
        frame = 0;
      }
    }

    function scheduleRender(): void {
      if (
        disposed ||
        canvasElement.dataset.meshGeneration !== generation ||
        frame !== 0 ||
        document.hidden
      ) {
        return;
      }
      previousFrame = 0;
      frame = window.requestAnimationFrame(render);
    }

    function restart(): void {
      window.cancelAnimationFrame(frame);
      frame = 0;
      signals = [];
      nextAmbientSignalAt = 0;
      nextHoverSignalAt = 0;
      const canRespond = !motionQuery.matches && finePointerQuery.matches && !document.hidden;
      canvasElement.dataset.motionState = canRespond ? "responsive" : "static";
      if (!canRespond) {
        pulseActive = false;
        pointer.strength = 0;
        pointer.targetStrength = 0;
        draw();
      } else {
        scheduleRender();
      }
    }

    function handlePointerMove(event: PointerEvent): void {
      if (!finePointerQuery.matches || motionQuery.matches) return;
      const now = performance.now();
      const decayStartedAt = Math.max(
        pulseStartedAt + HOVER_ATTACK_DURATION,
        lastPointerActivityAt + HOVER_IDLE_GRACE
      );
      if (!pulseActive || now > decayStartedAt) {
        pulseStartedAt = now;
        pulseStartStrength = pointer.strength;
        pulseActive = true;
        nextHoverSignalAt =
          nextHoverSignalAt === 0
            ? now + HOVER_SIGNAL_DELAY_MIN
            : Math.min(nextHoverSignalAt, now + HOVER_SIGNAL_DELAY_MIN);
      }
      lastPointerActivityAt = now;
      activePanelElement =
        event.target instanceof Element ? event.target.closest(panelSelector) : null;
      if (pointer.strength === 0) {
        pointer.x = event.clientX;
        pointer.y = event.clientY;
      }
      pointer.targetX = event.clientX;
      pointer.targetY = event.clientY;
      scheduleRender();
    }

    function releasePointer(): void {
      lastPointerActivityAt = performance.now() - HOVER_IDLE_GRACE;
      activePanelElement = null;
      scheduleRender();
    }

    function handleLayoutChange(): void {
      if (layoutFrame !== 0) return;
      layoutFrame = window.requestAnimationFrame(() => {
        layoutFrame = 0;
        refreshPanelRegions();
        draw();
      });
    }

    function handleSchemeChange(): void {
      refreshColors();
      draw();
    }

    refreshColors();
    resize();
    const mutationObserver = new MutationObserver(handleLayoutChange);
    const resizeObserver = new ResizeObserver(handleLayoutChange);
    if (contentRoot) {
      mutationObserver.observe(contentRoot, { childList: true, subtree: true });
      resizeObserver.observe(contentRoot);
    }

    window.addEventListener("resize", resize, { passive: true });
    window.addEventListener("scroll", handleLayoutChange, { passive: true });
    scrollRoot?.addEventListener("scroll", handleLayoutChange, { passive: true });
    window.addEventListener("pointermove", handlePointerMove, { passive: true });
    window.addEventListener("blur", releasePointer);
    document.documentElement.addEventListener("mouseleave", releasePointer);
    document.addEventListener("visibilitychange", restart);
    motionQuery.addEventListener("change", restart);
    finePointerQuery.addEventListener("change", restart);
    schemeQuery.addEventListener("change", handleSchemeChange);
    scheduleRender();

    return () => {
      disposed = true;
      mutationObserver.disconnect();
      resizeObserver.disconnect();
      window.cancelAnimationFrame(frame);
      window.cancelAnimationFrame(layoutFrame);
      window.removeEventListener("resize", resize);
      window.removeEventListener("scroll", handleLayoutChange);
      scrollRoot?.removeEventListener("scroll", handleLayoutChange);
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("blur", releasePointer);
      document.documentElement.removeEventListener("mouseleave", releasePointer);
      document.removeEventListener("visibilitychange", restart);
      motionQuery.removeEventListener("change", restart);
      finePointerQuery.removeEventListener("change", restart);
      schemeQuery.removeEventListener("change", handleSchemeChange);
    };
  }, [
    contentRootSelector,
    panelSelector,
    scrollRootSelector,
    theme,
    visualTheme
  ]);

  return (
    <canvas
      ref={canvasRef}
      className="pds-connected-fabric catalog__connected-fabric"
      aria-hidden="true"
      data-pds-visual-language="connected-fabric"
      data-visual-theme={visualTheme}
      data-semantic-role="decorative"
      data-edge-policy="shared-nodes-exclusive-edges"
      data-motion-state="initializing"
      data-active-circuit-count="0"
      data-active-edge-count="0"
      data-active-unique-edge-count="0"
      data-max-active-circuit-count="0"
      data-max-active-edge-count="0"
      data-edge-conflict-detected="false"
    />
  );
};
