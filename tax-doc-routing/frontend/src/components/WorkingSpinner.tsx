import { useEffect, useState } from 'react';

/**
 * A "working" indicator that keeps turning whatever the computer's animation setting is.
 *
 * The design system's own spinner is a CSS animation, which Windows' "show animations" switch (and
 * the browser's reduced-motion preference) turns off, leaving a frozen arc that looks like a hung
 * page. Here the turn is stepped by a timer instead, so it still moves. It is small and slow, and
 * used only where the user is waiting on the server; the words beside it say what is happening.
 */
export function WorkingSpinner({ size = 16 }: { size?: number }) {
  const [step, setStep] = useState(0);
  useEffect(() => {
    const timer = window.setInterval(() => setStep((s) => (s + 1) % 12), 90);
    return () => window.clearInterval(timer);
  }, []);
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" aria-hidden="true" focusable="false" data-testid="working-spinner">
      <circle cx="12" cy="12" r="9" stroke="currentColor" strokeOpacity="0.25" strokeWidth="3" />
      <g transform={`rotate(${step * 30} 12 12)`}>
        <path d="M12 3a9 9 0 0 1 9 9" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
      </g>
    </svg>
  );
}
