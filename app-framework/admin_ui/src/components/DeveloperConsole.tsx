import { useEffect, useMemo, useState } from "react";
import { CheckCircle2, Clipboard, Terminal, Trash2, XCircle } from "lucide-react";
import { Button, Drawer, IconButton } from "@appfw/pds-health-components";
import type { GraphqlTrace } from "../types";
import { formatResponseTime } from "../lib/timing";

type DeveloperConsoleProps = {
  schemaName: string;
  traces: GraphqlTrace[];
  troubleshootingEnabled?: boolean;
  onClear: () => void;
  onClose: () => void;
};

export function DeveloperConsole({ schemaName, traces, troubleshootingEnabled = false, onClear, onClose }: DeveloperConsoleProps) {
  const [selectedTraceId, setSelectedTraceId] = useState<string | null>(traces[0]?.id ?? null);
  const active = useMemo(
    () => traces.find((trace) => trace.id === selectedTraceId) ?? traces[0] ?? null,
    [selectedTraceId, traces]
  );

  useEffect(() => {
    if (!traces.length) {
      setSelectedTraceId(null);
      return;
    }
    if (!selectedTraceId || !traces.some((trace) => trace.id === selectedTraceId)) {
      setSelectedTraceId(traces[0].id);
    }
  }, [selectedTraceId, traces]);

  return (
    <Drawer
      open
      className="console-drawer"
      size="lg"
      title="GraphQL Inspector"
      description="Developer Console"
      onClose={onClose}
      footer={(
        <Button disabled={!traces.length} onClick={onClear} title="Clear history">
          <Trash2 size={17} />
          Clear history
        </Button>
      )}
    >
        <div className="console-body">
          <nav className="trace-list">
            <div className={`troubleshooting-status ${troubleshootingEnabled ? "ok" : ""}`}>
              <strong>Server Troubleshooting</strong>
              <span>
                {troubleshootingEnabled
                  ? "Enabled for admin-only diagnostics"
                  : "Disabled. Set APP_ADMIN_TROUBLESHOOTING_ENABLED=true to enable server-backed diagnostics."}
              </span>
            </div>
            {traces.length ? (
              traces.map((trace) => (
                <button
                  className={trace.id === active?.id ? "is-active" : ""}
                  key={trace.id}
                  onClick={() => setSelectedTraceId(trace.id)}
                  type="button"
                >
                  {trace.ok ? <CheckCircle2 size={15} /> : <XCircle size={15} />}
                  <span>
                    <strong>{trace.operation}</strong>
                    <small>
                      {trace.schemaName} | {formatResponseTime(trace.responseMs)}
                    </small>
                  </span>
                </button>
              ))
            ) : (
              <div className="empty-console">
                <Terminal size={22} />
                No GraphQL activity captured for {schemaName || "this schema"} yet
              </div>
            )}
          </nav>

          <div className="trace-details">
            {active ? (
              <section className="trace-card" key={active.id}>
                <div className="trace-head">
                  <span className={`trace-status ${active.ok ? "ok" : "error"}`}>{active.ok ? "OK" : "ERROR"}</span>
                  <strong>{active.operation}</strong>
                  <em>{new Date(active.createdAt).toLocaleTimeString()}</em>
                </div>
                <div className="trace-metrics">
                  <span>Schema: {active.schemaName}</span>
                  <span>HTTP: {active.httpStatus}</span>
                  <span>Response time: {formatResponseTime(active.responseMs)}</span>
                  {active.requestId && <span>Request: {active.responseRequestId ?? active.requestId}</span>}
                  {active.correlationId && <span>Correlation: {active.responseCorrelationId ?? active.correlationId}</span>}
                </div>
                {active.error && <div className="trace-error">{active.error}</div>}
                <TraceBlock label="Query" value={active.query} />
                <TraceBlock label="Variables" value={prettyJson(active.variables)} />
                <TraceBlock label={active.ok ? "Data" : "Payload"} value={prettyJson(active.ok ? active.data : active.payload)} />
              </section>
            ) : (
              <div className="empty-console large">
                <Terminal size={28} />
                Run a grid query, open a record, or save a form in {schemaName || "this schema"} to inspect the request.
              </div>
            )}
          </div>
        </div>
    </Drawer>
  );
}

function TraceBlock({ label, value }: { label: string; value: string }) {
  async function copyValue() {
    await navigator.clipboard?.writeText(value);
  }

  return (
    <section className="trace-block">
      <div>
        <strong>{label}</strong>
        <IconButton
          size="sm"
          ariaLabel={`Copy ${label}`}
          tooltip={`Copy ${label}`}
          icon={<Clipboard size={14} />}
          onClick={copyValue}
        />
      </div>
      <pre>{value}</pre>
    </section>
  );
}

function prettyJson(value: unknown) {
  if (value === undefined) return "{}";
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}
