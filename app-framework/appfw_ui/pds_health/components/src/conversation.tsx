import type {
  FormEvent,
  HTMLAttributes,
  ReactNode,
  TextareaHTMLAttributes
} from "react";
import { Button } from "./primitives";
import { composeClassNames, type PdsTone } from "./types";

export type MessageRole = "user" | "assistant" | "system" | "tool";

export type MessageItem = {
  id: string;
  role: MessageRole;
  author: ReactNode;
  content: ReactNode;
  timestamp?: ReactNode;
  status?: ReactNode;
  metadata?: ReactNode;
};

export type MessageThreadProps = HTMLAttributes<HTMLElement> & {
  title?: ReactNode;
  description?: ReactNode;
  messages?: readonly MessageItem[];
  actions?: ReactNode;
  ariaLabel?: string;
};

export function MessageThread({
  title = "Conversation",
  description,
  messages = [],
  actions,
  ariaLabel = "Conversation thread",
  className,
  children,
  ...props
}: MessageThreadProps) {
  return (
    <section
      {...props}
      className={composeClassNames("pds-message-thread", className)}
      aria-label={ariaLabel}
    >
      <header className="pds-message-thread__header">
        <div className="pds-message-thread__copy">
          <h3 className="pds-message-thread__title">{title}</h3>
          {description ? <p className="pds-message-thread__description">{description}</p> : null}
        </div>
        {actions ? <div className="pds-message-thread__actions">{actions}</div> : null}
      </header>
      <ol className="pds-message-thread__list" aria-label="Messages">
        {messages.map((message) => (
          <li key={message.id}>
            <Message
              role={message.role}
              author={message.author}
              timestamp={message.timestamp}
              status={message.status}
              metadata={message.metadata}
            >
              {message.content}
            </Message>
          </li>
        ))}
        {children ? <li>{children}</li> : null}
      </ol>
    </section>
  );
}

export type MessageProps = HTMLAttributes<HTMLElement> & {
  role?: MessageRole;
  author: ReactNode;
  timestamp?: ReactNode;
  status?: ReactNode;
  metadata?: ReactNode;
};

export function Message({
  role = "assistant",
  author,
  timestamp,
  status,
  metadata,
  className,
  children,
  ...props
}: MessageProps) {
  return (
    <article
      {...props}
      className={composeClassNames("pds-message", className)}
      data-role={role}
    >
      <header className="pds-message__header">
        <strong className="pds-message__author">{author}</strong>
        {timestamp ? <span className="pds-message__timestamp">{timestamp}</span> : null}
        {status ? <span className="pds-message__status">{status}</span> : null}
      </header>
      <div className="pds-message__body">{children}</div>
      {metadata ? <footer className="pds-message__metadata">{metadata}</footer> : null}
    </article>
  );
}

export type MessageComposerProps = Omit<HTMLAttributes<HTMLFormElement>, "onSubmit"> & {
  value?: string;
  placeholder?: string;
  disabled?: boolean;
  minRows?: number;
  submitLabel?: ReactNode;
  helperText?: ReactNode;
  inputProps?: Omit<TextareaHTMLAttributes<HTMLTextAreaElement>, "value" | "onChange" | "placeholder" | "disabled" | "rows">;
  onValueChange?: (value: string) => void;
  onSubmitMessage?: (value: string) => void;
};

export function MessageComposer({
  value = "",
  placeholder = "Ask a question",
  disabled = false,
  minRows = 3,
  submitLabel = "Send",
  helperText,
  inputProps,
  onValueChange,
  onSubmitMessage,
  className,
  ...props
}: MessageComposerProps) {
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const nextValue = value.trim();
    if (!nextValue || disabled) return;
    onSubmitMessage?.(nextValue);
  }

  return (
    <form
      {...props}
      className={composeClassNames("pds-message-composer", className)}
      onSubmit={submit}
    >
      <label className="pds-message-composer__label">
        <span className="pds-message-composer__label-text">Message</span>
        <textarea
          {...inputProps}
          className={composeClassNames("pds-message-composer__input", inputProps?.className)}
          value={value}
          placeholder={placeholder}
          disabled={disabled}
          rows={minRows}
          onChange={(event) => onValueChange?.(event.currentTarget.value)}
        />
      </label>
      <div className="pds-message-composer__footer">
        {helperText ? <span className="pds-message-composer__helper">{helperText}</span> : null}
        <Button variant="primary" size="sm" type="submit" disabled={disabled || !value.trim()}>
          {submitLabel}
        </Button>
      </div>
    </form>
  );
}

export type StreamingTextProps = HTMLAttributes<HTMLDivElement> & {
  isStreaming?: boolean;
  cursorLabel?: string;
};

export function StreamingText({
  isStreaming = false,
  cursorLabel = "Response streaming",
  className,
  children,
  ...props
}: StreamingTextProps) {
  return (
    <div
      {...props}
      className={composeClassNames("pds-streaming-text", className)}
      data-streaming={isStreaming || undefined}
      role="status"
      aria-live="polite"
    >
      <span className="pds-streaming-text__content">{children}</span>
      {isStreaming ? (
        <>
          <span className="pds-streaming-text__cursor" aria-hidden="true" />
          <span className="pds-streaming-text__sr-label">{cursorLabel}</span>
        </>
      ) : null}
    </div>
  );
}

export type ToolCallStatusValue = "queued" | "running" | "blocked" | "completed" | "failed";

export type ToolCallStatusProps = HTMLAttributes<HTMLDivElement> & {
  label: ReactNode;
  status?: ToolCallStatusValue;
  detail?: ReactNode;
  requestId?: ReactNode;
};

export function ToolCallStatus({
  label,
  status = "queued",
  detail,
  requestId,
  className,
  ...props
}: ToolCallStatusProps) {
  return (
    <div
      {...props}
      className={composeClassNames("pds-tool-call-status", className)}
      data-status={status}
      role="status"
      aria-live={status === "failed" || status === "blocked" ? "assertive" : "polite"}
    >
      <span className="pds-tool-call-status__indicator" aria-hidden="true" />
      <span className="pds-tool-call-status__copy">
        <strong>{label}</strong>
        {detail ? <small>{detail}</small> : null}
      </span>
      {requestId ? <code className="pds-tool-call-status__request">{requestId}</code> : null}
    </div>
  );
}

export type EntityRefCardProps = HTMLAttributes<HTMLElement> & {
  title: ReactNode;
  entityType?: ReactNode;
  caption?: ReactNode;
  recordLocator?: ReactNode;
  href?: string;
  status?: ReactNode;
  actions?: ReactNode;
};

export function EntityRefCard({
  title,
  entityType,
  caption,
  recordLocator,
  href,
  status,
  actions,
  className,
  ...props
}: EntityRefCardProps) {
  const titleNode = href ? <a href={href}>{title}</a> : title;
  return (
    <article
      {...props}
      className={composeClassNames("pds-entity-ref-card", className)}
      aria-label={typeof title === "string" ? title : "Referenced entity"}
    >
      <header className="pds-entity-ref-card__header">
        <div className="pds-entity-ref-card__copy">
          {entityType ? <span className="pds-entity-ref-card__entity">{entityType}</span> : null}
          <h3 className="pds-entity-ref-card__title">{titleNode}</h3>
          {caption ? <p className="pds-entity-ref-card__caption">{caption}</p> : null}
        </div>
        {status ? <span className="pds-entity-ref-card__status">{status}</span> : null}
      </header>
      {recordLocator ? (
        <p className="pds-entity-ref-card__locator">
          <span>Record locator</span>
          <code>{recordLocator}</code>
        </p>
      ) : null}
      {actions ? <div className="pds-entity-ref-card__actions">{actions}</div> : null}
    </article>
  );
}

export type CitationItem = {
  id: string;
  label: ReactNode;
  source?: ReactNode;
  href?: string;
  detail?: ReactNode;
};

export type CitationListProps = HTMLAttributes<HTMLElement> & {
  citations: readonly CitationItem[];
  title?: ReactNode;
  ariaLabel?: string;
};

export function CitationList({
  citations,
  title = "Citations",
  ariaLabel = "Citations",
  className,
  ...props
}: CitationListProps) {
  return (
    <section
      {...props}
      className={composeClassNames("pds-citation-list", className)}
      aria-label={ariaLabel}
    >
      <h3 className="pds-citation-list__title">{title}</h3>
      <ol className="pds-citation-list__items">
        {citations.map((citation) => (
          <li key={citation.id} className="pds-citation-list__item">
            {citation.href ? <a href={citation.href}>{citation.label}</a> : <span>{citation.label}</span>}
            {citation.source ? <small>{citation.source}</small> : null}
            {citation.detail ? <em>{citation.detail}</em> : null}
          </li>
        ))}
      </ol>
    </section>
  );
}

export type ConfidenceSignalValue = "low" | "medium" | "high" | "unknown";

export type ConfidenceSignalProps = HTMLAttributes<HTMLDivElement> & {
  value?: ConfidenceSignalValue;
  label?: ReactNode;
  detail?: ReactNode;
  tone?: Extract<PdsTone, "neutral" | "accent" | "success" | "warning" | "danger">;
};

export function ConfidenceSignal({
  value = "unknown",
  label = "Confidence",
  detail,
  tone = value === "high" ? "success" : value === "low" ? "warning" : "accent",
  className,
  ...props
}: ConfidenceSignalProps) {
  return (
    <div
      {...props}
      className={composeClassNames("pds-confidence-signal", className)}
      data-confidence={value}
      data-tone={tone}
      role="status"
    >
      <span className="pds-confidence-signal__label">{label}</span>
      <strong className="pds-confidence-signal__value">{value}</strong>
      {detail ? <small className="pds-confidence-signal__detail">{detail}</small> : null}
    </div>
  );
}

export type AgentTimelineStatus = "queued" | "running" | "waiting" | "completed" | "blocked" | "failed";

export type AgentTimelineItem = {
  id: string;
  title: ReactNode;
  status?: AgentTimelineStatus;
  timestamp?: ReactNode;
  detail?: ReactNode;
  evidence?: ReactNode;
};

export type AgentTimelineProps = HTMLAttributes<HTMLElement> & {
  title?: ReactNode;
  description?: ReactNode;
  items: readonly AgentTimelineItem[];
  ariaLabel?: string;
};

export function AgentTimeline({
  title = "Agent timeline",
  description,
  items,
  ariaLabel = "Agent timeline",
  className,
  ...props
}: AgentTimelineProps) {
  return (
    <aside
      {...props}
      className={composeClassNames("pds-agent-timeline", className)}
      aria-label={ariaLabel}
    >
      <header className="pds-agent-timeline__header">
        <h3 className="pds-agent-timeline__title">{title}</h3>
        {description ? <p className="pds-agent-timeline__description">{description}</p> : null}
      </header>
      <ol className="pds-agent-timeline__list">
        {items.map((item) => (
          <li key={item.id} className="pds-agent-timeline__item" data-status={item.status ?? "queued"}>
            <span className="pds-agent-timeline__marker" aria-hidden="true" />
            <span className="pds-agent-timeline__content">
              <strong>{item.title}</strong>
              {item.timestamp ? <small>{item.timestamp}</small> : null}
              {item.detail ? <span>{item.detail}</span> : null}
              {item.evidence ? <em>{item.evidence}</em> : null}
            </span>
          </li>
        ))}
      </ol>
    </aside>
  );
}

export type FlowGraphNode = {
  id: string;
  label: ReactNode;
  detail?: ReactNode;
  status?: AgentTimelineStatus;
};

export type FlowGraphEdge = {
  id: string;
  from: string;
  to: string;
  label?: ReactNode;
};

export type FlowGraphShellProps = HTMLAttributes<HTMLElement> & {
  title?: ReactNode;
  description?: ReactNode;
  nodes?: readonly FlowGraphNode[];
  edges?: readonly FlowGraphEdge[];
  toolbar?: ReactNode;
  legend?: ReactNode;
  renderGraph?: (input: {
    nodes: readonly FlowGraphNode[];
    edges: readonly FlowGraphEdge[];
  }) => ReactNode;
  ariaLabel?: string;
};

export function FlowGraphShell({
  title = "Flow graph",
  description,
  nodes = [],
  edges = [],
  toolbar,
  legend,
  renderGraph,
  ariaLabel = "Flow graph",
  className,
  ...props
}: FlowGraphShellProps) {
  return (
    <section
      {...props}
      className={composeClassNames("pds-flow-graph-shell", className)}
      aria-label={ariaLabel}
    >
      <header className="pds-flow-graph-shell__header">
        <div className="pds-flow-graph-shell__copy">
          <h3 className="pds-flow-graph-shell__title">{title}</h3>
          {description ? <p className="pds-flow-graph-shell__description">{description}</p> : null}
        </div>
        {toolbar ? <div className="pds-flow-graph-shell__toolbar">{toolbar}</div> : null}
      </header>
      <div className="pds-flow-graph-shell__viewport" role="img" aria-label={ariaLabel}>
        {renderGraph ? (
          renderGraph({ nodes, edges })
        ) : (
          <ol className="pds-flow-graph-shell__fallback">
            {nodes.map((node) => (
              <li key={node.id} data-status={node.status ?? "queued"}>
                <strong>{node.label}</strong>
                {node.detail ? <span>{node.detail}</span> : null}
              </li>
            ))}
          </ol>
        )}
      </div>
      {legend || edges.length > 0 ? (
        <footer className="pds-flow-graph-shell__footer">
          {legend ? <div className="pds-flow-graph-shell__legend">{legend}</div> : null}
          {edges.length > 0 ? <span>{edges.length} relationships</span> : null}
        </footer>
      ) : null}
    </section>
  );
}
