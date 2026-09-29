import type { HTMLAttributes, ReactNode } from "react";
import { useId } from "react";
import {
  Message,
  MessageComposer,
  MessageThread,
  StreamingText,
  type MessageComposerProps,
  type MessageItem,
  type MessageThreadProps
} from "./conversation";
import { Button } from "./primitives";
import { composeClassNames, describedBy } from "./types";

export type ConversationWorkspacePrompt = {
  id: string;
  value: string;
  label?: ReactNode;
  ariaLabel?: string;
  disabled?: boolean;
};

export type ConversationWorkspaceProps = Omit<
  HTMLAttributes<HTMLElement>,
  "children" | "title"
> & {
  title?: ReactNode;
  description?: ReactNode;
  headerActions?: ReactNode;
  messages?: readonly MessageItem[];
  threadProps?: Omit<MessageThreadProps, "children" | "messages">;
  messageRegionLabel?: string;
  contextRail?: ReactNode;
  contextRailLabel?: string;
  suggestedPrompts?: readonly ConversationWorkspacePrompt[];
  suggestedPromptsLabel?: string;
  onSuggestedPromptSelect?: (
    value: string,
    prompt: ConversationWorkspacePrompt
  ) => void;
  actionRegion?: ReactNode;
  actionRegionLabel?: string;
  composerProps?: MessageComposerProps;
  isBusy?: boolean;
  busyAuthor?: ReactNode;
  busyContent?: ReactNode;
  busyCursorLabel?: string;
  busyStatus?: ReactNode;
  ariaLabel?: string;
};

/**
 * A product-neutral conversation floor plan for invoked assistance.
 *
 * Products own prompts, messages, model calls, evidence, actions, and context
 * content. PDS owns the responsive regions, busy behavior, suggested-prompt
 * controls, and accessible thread/composer composition.
 */
export function ConversationWorkspace({
  title,
  description,
  headerActions,
  messages = [],
  threadProps,
  messageRegionLabel = "Conversation messages",
  contextRail,
  contextRailLabel = "Conversation context",
  suggestedPrompts = [],
  suggestedPromptsLabel = "Suggested prompts",
  onSuggestedPromptSelect,
  actionRegion,
  actionRegionLabel = "Conversation actions",
  composerProps,
  isBusy = false,
  busyAuthor = "Assistant",
  busyContent = "Working…",
  busyCursorLabel = "Response in progress",
  busyStatus,
  ariaLabel,
  id,
  className,
  "aria-label": nativeAriaLabel,
  "aria-labelledby": nativeAriaLabelledBy,
  "aria-describedby": nativeAriaDescribedBy,
  ...props
}: ConversationWorkspaceProps) {
  const generatedId = useId();
  const rootId = id ?? `pds-conversation-workspace-${generatedId}`;
  const titleId = title ? `${rootId}-title` : undefined;
  const descriptionId = description ? `${rootId}-description` : undefined;
  const {
    className: threadClassName,
    ariaLabel: threadAriaLabel,
    ...restThreadProps
  } = threadProps ?? {};
  const {
    className: composerClassName,
    disabled: composerDisabled,
    ...restComposerProps
  } = composerProps ?? {};
  const resolvedAriaLabel =
    ariaLabel ?? nativeAriaLabel ?? (title ? undefined : "Conversation workspace");
  const resolvedAriaLabelledBy =
    nativeAriaLabelledBy ?? (resolvedAriaLabel ? undefined : titleId);
  const hasHeader = Boolean(title || description || headerActions);

  return (
    <section
      {...props}
      id={rootId}
      className={composeClassNames("pds-conversation-workspace", className)}
      data-busy={isBusy || undefined}
      data-has-context={Boolean(contextRail) || undefined}
      data-has-header={hasHeader || undefined}
      aria-label={resolvedAriaLabel}
      aria-labelledby={resolvedAriaLabelledBy}
      aria-describedby={describedBy(nativeAriaDescribedBy, descriptionId)}
      aria-busy={isBusy}
    >
      {hasHeader ? (
        <header className="pds-conversation-workspace__header">
          <div className="pds-conversation-workspace__heading">
            {title ? (
              <h2 id={titleId} className="pds-conversation-workspace__title">
                {title}
              </h2>
            ) : null}
            {description ? (
              <p
                id={descriptionId}
                className="pds-conversation-workspace__description"
              >
                {description}
              </p>
            ) : null}
          </div>
          {headerActions ? (
            <div className="pds-conversation-workspace__header-actions">
              {headerActions}
            </div>
          ) : null}
        </header>
      ) : null}

      <div className="pds-conversation-workspace__body">
        <div className="pds-conversation-workspace__main">
          <div
            className="pds-conversation-workspace__thread-region"
            role="log"
            aria-label={messageRegionLabel}
            aria-live="polite"
            aria-relevant="additions text"
            aria-busy={isBusy}
          >
            <MessageThread
              {...restThreadProps}
              className={composeClassNames(
                "pds-conversation-workspace__thread",
                threadClassName
              )}
              ariaLabel={threadAriaLabel ?? messageRegionLabel}
              messages={messages}
            >
              {isBusy ? (
                <Message
                  className="pds-conversation-workspace__busy-message"
                  role="assistant"
                  author={busyAuthor}
                  status={busyStatus}
                >
                  <StreamingText
                    isStreaming
                    cursorLabel={busyCursorLabel}
                  >
                    {busyContent}
                  </StreamingText>
                </Message>
              ) : null}
            </MessageThread>
          </div>

          {suggestedPrompts.length > 0 ? (
            <div
              className="pds-conversation-workspace__suggestions"
              role="group"
              aria-label={suggestedPromptsLabel}
            >
              {suggestedPrompts.map((prompt) => (
                <Button
                  key={prompt.id}
                  type="button"
                  size="sm"
                  variant="secondary"
                  aria-label={prompt.ariaLabel}
                  disabled={
                    isBusy || prompt.disabled || !onSuggestedPromptSelect
                  }
                  onClick={() => onSuggestedPromptSelect?.(prompt.value, prompt)}
                >
                  {prompt.label ?? prompt.value}
                </Button>
              ))}
            </div>
          ) : null}

          {actionRegion ? (
            <section
              className="pds-conversation-workspace__action-region"
              aria-label={actionRegionLabel}
            >
              {actionRegion}
            </section>
          ) : null}

          {composerProps ? (
            <MessageComposer
              {...restComposerProps}
              className={composeClassNames(
                "pds-conversation-workspace__composer",
                composerClassName
              )}
              disabled={isBusy || composerDisabled}
            />
          ) : null}
        </div>

        {contextRail ? (
          <aside
            className="pds-conversation-workspace__context-rail"
            aria-label={contextRailLabel}
          >
            {contextRail}
          </aside>
        ) : null}
      </div>
    </section>
  );
}
