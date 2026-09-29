/**
 * Transport client for the Framework IX foreground endpoints.
 *
 * The client speaks the real wire contract only: POST `/chat/stream` with a
 * run request (start) or with a `last-event-id` header and empty body
 * (resume), receiving the canonical envelope stream as server-sent events;
 * POST `/chat/runs/:runId/cancel` with an idempotent command id. Nothing is
 * mocked, intercepted, or fixture-fed here. The endpoints become servable
 * when the B_HOST leaf constructs and mounts the chat-gated IX runtime
 * (DEFERRED_TO_B_HOST); until then calls surface the backend's real
 * unavailability, which the journey renders as the unavailable posture.
 */

import {
  IX_CANCEL_REQUEST_SCHEMA_VERSION,
  IX_STREAM_PATH,
  ixCancelPath,
  type IxCallerAuth,
  type IxCancelReceiptWire,
  type IxEventWire,
  type IxRunRequestWire,
  type IxStreamFrame
} from './types';

export type IxStreamHandle = {
  /** Resolves when the stream closes (server end or abort). */
  done: Promise<void>;
  abort: () => void;
};

export type IxTransportAccepted = {
  status: number;
  lastEventId: string | null;
};

export type IxStreamCallbacks = {
  onFrame: (frame: IxStreamFrame) => void;
  onStreamError: (error: IxTransportUnavailable) => void;
  /** Fired only after fetch returns an authenticated 2xx. */
  onTransportAccepted?: (info: IxTransportAccepted) => void;
};

/** The transport said no (or is not mounted yet). */
export class IxTransportUnavailable extends Error {
  readonly status: number | null;

  constructor(message: string, status: number | null) {
    super(message);
    this.name = 'IxTransportUnavailable';
    this.status = status;
  }
}

function ixHeaders(auth: IxCallerAuth): Headers {
  const headers = new Headers();
  headers.set('authorization', `Bearer ${auth.bearerToken}`);
  headers.set('x-tenant-id', auth.tenantId);
  headers.set('accept', 'text/event-stream');
  return headers;
}

/** Starts a new run and streams its canonical envelopes. */
export function startRunStream(
  request: IxRunRequestWire,
  auth: IxCallerAuth,
  callbacks: IxStreamCallbacks,
  baseUrl = ''
): IxStreamHandle {
  const headers = ixHeaders(auth);
  headers.set('content-type', 'application/json');
  return openStream(
    `${baseUrl}${IX_STREAM_PATH}`,
    { method: 'POST', headers, body: JSON.stringify(request) },
    callbacks
  );
}

/** Resumes an existing run from a commit-boundary cursor. */
export function resumeRunStream(
  cursor: string,
  auth: IxCallerAuth,
  callbacks: IxStreamCallbacks,
  baseUrl = ''
): IxStreamHandle {
  const headers = ixHeaders(auth);
  headers.set('last-event-id', cursor);
  return openStream(
    `${baseUrl}${IX_STREAM_PATH}`,
    { method: 'POST', headers },
    callbacks
  );
}

/** Requests an idempotent cancel with a stable command id. */
export async function cancelRun(
  runId: string,
  commandId: string,
  auth: IxCallerAuth,
  baseUrl = ''
): Promise<IxCancelReceiptWire> {
  const headers = ixHeaders(auth);
  headers.set('content-type', 'application/json');
  headers.set('accept', 'application/json');
  let response: Response;
  try {
    response = await fetch(`${baseUrl}${ixCancelPath(runId)}`, {
      method: 'POST',
      headers,
      body: JSON.stringify({
        schemaVersion: IX_CANCEL_REQUEST_SCHEMA_VERSION,
        commandId
      })
    });
  } catch {
    throw new IxTransportUnavailable('The IX transport is unreachable.', null);
  }
  if (response.status === 202 || response.status === 200 || response.status === 404) {
    return (await response.json()) as IxCancelReceiptWire;
  }
  throw new IxTransportUnavailable(
    `Cancel was refused by the transport (${response.status}).`,
    response.status
  );
}

function openStream(
  url: string,
  init: RequestInit,
  callbacks: IxStreamCallbacks
): IxStreamHandle {
  const controller = new AbortController();
  const done = (async () => {
    let response: Response;
    try {
      response = await fetch(url, { ...init, signal: controller.signal });
    } catch (error) {
      if (!controller.signal.aborted) {
        callbacks.onStreamError(
          new IxTransportUnavailable('The IX transport is unreachable.', null)
        );
      }
      return;
    }
    if (!response.ok || !response.body) {
      callbacks.onStreamError(
        new IxTransportUnavailable(
          `The IX transport refused the stream (${response.status}).`,
          response.status
        )
      );
      return;
    }
    const requestHeaders = new Headers(init.headers);
    callbacks.onTransportAccepted?.({
      status: response.status,
      lastEventId: requestHeaders.get('last-event-id')
    });
    try {
      await consumeServerSentEvents(response.body, callbacks.onFrame);
    } catch (error) {
      if (!controller.signal.aborted) {
        callbacks.onStreamError(
          new IxTransportUnavailable('The IX stream ended abnormally.', null)
        );
      }
    }
  })();
  return {
    done,
    abort: () => controller.abort()
  };
}

/**
 * Minimal SSE parser for the canonical envelope stream: each message is one
 * Framework `IxEvent` JSON in `data:`; the final event of an envelope
 * carries the commit cursor in `id:`; `heartbeat` comments are ignored.
 */
async function consumeServerSentEvents(
  body: ReadableStream<Uint8Array>,
  onFrame: (frame: IxStreamFrame) => void
): Promise<void> {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let buffered = '';
  for (;;) {
    const { done, value } = await reader.read();
    if (done) {
      break;
    }
    buffered += decoder.decode(value, { stream: true });
    let boundary = buffered.indexOf('\n\n');
    while (boundary >= 0) {
      const rawMessage = buffered.slice(0, boundary);
      buffered = buffered.slice(boundary + 2);
      const frame = parseSseMessage(rawMessage);
      if (frame) {
        onFrame(frame);
      }
      boundary = buffered.indexOf('\n\n');
    }
  }
}

function parseSseMessage(rawMessage: string): IxStreamFrame | null {
  let id: string | undefined;
  const dataLines: string[] = [];
  for (const line of rawMessage.split('\n')) {
    if (line.startsWith(':')) {
      continue; // keep-alive heartbeat comment
    }
    if (line.startsWith('id:')) {
      id = line.slice(3).trim();
    } else if (line.startsWith('data:')) {
      dataLines.push(line.slice(5).trimStart());
    }
  }
  if (dataLines.length === 0) {
    return null;
  }
  const data = dataLines.join('\n');
  if (data === 'heartbeat') {
    return null;
  }
  let event: IxEventWire;
  try {
    event = JSON.parse(data) as IxEventWire;
  } catch {
    return null;
  }
  if (typeof event !== 'object' || event === null || !('payload' in event)) {
    return null;
  }
  return { event, commitCursor: id };
}
