import {
  useEffect,
  useState,
  type ReactElement,
  type ReactNode,
  type RefObject
} from "react";
import {
  Tooltip as AriaTooltip,
  TooltipTrigger as AriaTooltipTrigger
} from "react-aria-components";

type OverflowTooltipProps = {
  content: ReactNode;
  children: ReactElement;
  overflowTargetRef: RefObject<HTMLElement>;
  position?: "top" | "right" | "bottom" | "left";
  disabled?: boolean;
  delay?: number;
};

export default function OverflowTooltip({
  content,
  children,
  overflowTargetRef,
  position = "right",
  disabled = false,
  delay = 500
}: OverflowTooltipProps) {
  const [isOverflowing, setIsOverflowing] = useState(false);

  useEffect(() => {
    const target = overflowTargetRef.current;
    if (!target || disabled) {
      setIsOverflowing(false);
      return undefined;
    }

    const measure = () => {
      setIsOverflowing(
        target.scrollWidth > target.clientWidth + 1
          || target.scrollHeight > target.clientHeight + 1
      );
    };
    const frame = requestAnimationFrame(measure);
    const observer = typeof ResizeObserver === "undefined"
      ? null
      : new ResizeObserver(measure);
    observer?.observe(target);
    return () => {
      cancelAnimationFrame(frame);
      observer?.disconnect();
    };
  }, [disabled, overflowTargetRef]);

  return (
    <AriaTooltipTrigger
      isDisabled={disabled || !isOverflowing}
      delay={delay}
      closeDelay={100}
    >
      {children}
      <AriaTooltip
        className="pds-overflow-tooltip"
        placement={position}
        offset={8}
      >
        {content}
      </AriaTooltip>
    </AriaTooltipTrigger>
  );
}
