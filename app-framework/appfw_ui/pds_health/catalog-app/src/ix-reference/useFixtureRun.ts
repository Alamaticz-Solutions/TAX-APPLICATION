import { useCallback, useEffect, useRef, useState } from "react";
import {
  beginIxReferencePlaybackState,
  initialIxReferencePlaybackState,
  pauseIxReferencePlaybackState,
  reduceIxReferencePlaybackEvent,
  resumeIxReferencePlaybackState,
  type IxArtifact,
  type IxReferencePlaybackState,
  type ScheduledIxReferencePlaybackEvent
} from "./contract";

type StartOptions = {
  preservedArtifact?: IxArtifact;
};

export type FixtureRunController = {
  state: IxReferencePlaybackState;
  start: (schedule: readonly ScheduledIxReferencePlaybackEvent[], options?: StartOptions) => void;
  stop: () => void;
  resume: () => void;
  reset: () => void;
  canResume: boolean;
};

export function useFixtureRun(): FixtureRunController {
  const [state, setState] = useState<IxReferencePlaybackState>(initialIxReferencePlaybackState);
  const stateRef = useRef(state);
  const scheduleRef = useRef<readonly ScheduledIxReferencePlaybackEvent[]>([]);
  const nextIndexRef = useRef(0);
  const tokenRef = useRef(0);
  const timerRef = useRef<number | undefined>();

  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  const clearPendingTimer = useCallback(() => {
    if (timerRef.current !== undefined) {
      window.clearTimeout(timerRef.current);
      timerRef.current = undefined;
    }
  }, []);

  const playFrom = useCallback(function play(index: number, token: number) {
    const scheduled = scheduleRef.current[index];
    if (!scheduled || token !== tokenRef.current) return;

    timerRef.current = window.setTimeout(() => {
      if (token !== tokenRef.current) return;
      try {
        setState((current) => reduceIxReferencePlaybackEvent(current, scheduled.event));
        nextIndexRef.current = index + 1;
        play(index + 1, token);
      } catch (error) {
        tokenRef.current += 1;
        clearPendingTimer();
        const message = error instanceof Error ? error.message : "The deterministic IX fixture failed.";
        setState((current) => ({
          ...current,
          status: "failed",
          playbackPhase: "terminal",
          fixturePlaybackPaused: false,
          phase: undefined,
          phaseLabel: "Reference stream failed safely",
          phaseDetail: message,
          terminalMessage: message,
          error: "fixture_contract_error"
        }));
      }
    }, scheduled.afterMs);
  }, [clearPendingTimer]);

  const start = useCallback((
    schedule: readonly ScheduledIxReferencePlaybackEvent[],
    options: StartOptions = {}
  ) => {
    const firstEvent = schedule[0]?.event;
    if (!firstEvent) return;
    tokenRef.current += 1;
    const token = tokenRef.current;
    clearPendingTimer();
    scheduleRef.current = schedule;
    nextIndexRef.current = 0;
    setState(beginIxReferencePlaybackState(
      firstEvent.playbackId,
      firstEvent.type === "playback_started" ? firstEvent.intentLabel : "Starting IX reference work",
      options.preservedArtifact
    ));
    playFrom(0, token);
  }, [clearPendingTimer, playFrom]);

  const stop = useCallback(() => {
    if (stateRef.current.status !== "running" || stateRef.current.fixturePlaybackPaused) return;
    tokenRef.current += 1;
    clearPendingTimer();
    setState((current) => pauseIxReferencePlaybackState(current));
  }, [clearPendingTimer]);

  const resume = useCallback(() => {
    if (stateRef.current.status !== "running" || !stateRef.current.fixturePlaybackPaused) return;
    tokenRef.current += 1;
    const token = tokenRef.current;
    setState((current) => resumeIxReferencePlaybackState(current));
    playFrom(nextIndexRef.current, token);
  }, [playFrom]);

  const reset = useCallback(() => {
    tokenRef.current += 1;
    clearPendingTimer();
    scheduleRef.current = [];
    nextIndexRef.current = 0;
    setState(initialIxReferencePlaybackState);
  }, [clearPendingTimer]);

  useEffect(() => () => {
    tokenRef.current += 1;
    clearPendingTimer();
  }, [clearPendingTimer]);

  return {
    state,
    start,
    stop,
    resume,
    reset,
    canResume: state.status === "running"
      && state.fixturePlaybackPaused
      && nextIndexRef.current < scheduleRef.current.length
  };
}
