#[cfg(feature = "chat")]
use std::{convert::Infallible, sync::Arc};
use std::{env, time::Duration};

use async_graphql::{ObjectType, Request as AsyncGraphQLRequest, Schema, SubscriptionType};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
#[cfg(feature = "chat")]
use axum::{
    body::Bytes,
    extract::Path,
    response::{
        sse::{Event, KeepAlive},
        IntoResponse as _, Sse,
    },
    Json,
};
use axum::{
    body::{boxed, Full},
    extract::{DefaultBodyLimit, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, Response, StatusCode},
    middleware, response,
    routing::{get, post},
    Extension, Router,
};
#[cfg(feature = "chat")]
use futures_util::{stream, StreamExt};
#[cfg(feature = "chat")]
use serde::Deserialize;
#[cfg(feature = "chat")]
use serde_json::json;
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    set_header::SetResponseHeaderLayer,
    timeout::TimeoutLayer,
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::Level;

const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 30_000;

#[cfg(feature = "chat")]
use crate::ix::LAST_EVENT_ID_HEADER;
#[cfg(feature = "chat")]
use crate::ix::{
    IxCancelDisposition, IxCommitEnvelope, IxJwtVerifier, IxReplayCursor, IxRunRequest,
    IxRuntimeService, IxTransportCancelReceipt, IxTransportError, IxTransportPolicy,
    IX_CANCEL_PATH, IX_CANCEL_REQUEST_SCHEMA_VERSION, IX_EVENT_SCHEMA_VERSION, IX_STREAM_PATH,
};
use crate::{
    graphiql,
    host::{RuntimeIngressKind, RuntimeMode},
    observability::{
        annotate_graphql_response, graphql_error_with_context, http_make_span, metrics_hook,
        trace_context_hook, MetricsRegistry, RequestContext, REQUEST_ID_HEADER_NAME,
    },
    security::{rate_limit_hook, RateLimiterState, SecurityConfig},
    RuntimeAuthState, RuntimeJwtExtractor, UserAuth,
};

#[cfg(feature = "chat")]
const IX_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Default)]
pub struct RuntimeRouteSet {
    info: Option<Router>,
    admin: Option<Router>,
    product_ui: Option<Router>,
    #[cfg(feature = "chat")]
    chat: Option<Router>,
    #[cfg(feature = "chat")]
    ix_chat: Option<IxRouteGroup>,
    #[cfg(feature = "mcp")]
    mcp: Option<Router>,
    schemas: Vec<Router>,
}

/// Opaque IX-only route group. Its policy-bound factory is the only way to
/// construct it and `RuntimeRouteSet::with_ix_chat` is the only public way to
/// consume it, preventing legacy credentialed CORS from wrapping IX routes.
#[cfg(feature = "chat")]
pub struct IxRouteGroup {
    router: Router,
    cors: CorsLayer,
}

#[cfg(feature = "chat")]
impl IxRouteGroup {
    pub(crate) fn from_profiled_router(router: Router, cors: CorsLayer) -> Self {
        Self { router, cors }
    }
}

#[cfg(feature = "chat")]
#[derive(Clone)]
struct IxHttpState {
    verifier: Arc<IxJwtVerifier>,
    policy: Arc<IxTransportPolicy>,
    service: Arc<IxRuntimeService>,
}

/// Constructs the sole IX HTTP ingress around the Framework-owned runtime
/// authority. Axum, CORS, request metadata, and SSE framing stay in routing;
/// `ix::transport` remains protocol/service semantics only.
#[cfg(feature = "chat")]
pub fn ix_transport_routes(
    verifier: Arc<IxJwtVerifier>,
    policy: Arc<IxTransportPolicy>,
    service: Arc<IxRuntimeService>,
) -> Result<IxRouteGroup, IxTransportError> {
    if !Arc::ptr_eq(verifier.policy(), &policy) {
        return Err(IxTransportError::InvalidPolicy);
    }
    let cors = ix_cors_layer(&policy)?;
    let state = IxHttpState {
        verifier,
        policy,
        service,
    };
    let router = Router::new()
        .route(IX_STREAM_PATH, post(ix_stream))
        .route(IX_CANCEL_PATH, post(ix_cancel))
        .with_state(state);
    Ok(IxRouteGroup::from_profiled_router(router, cors))
}

#[cfg(feature = "chat")]
fn ix_cors_layer(policy: &IxTransportPolicy) -> Result<CorsLayer, IxTransportError> {
    let origins = policy
        .allowed_origins()
        .map(|origin| {
            origin
                .parse::<HeaderValue>()
                .map_err(|_| IxTransportError::InvalidPolicy)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut cors = CorsLayer::new()
        .allow_methods([axum::http::Method::POST])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            HeaderName::from_static(LAST_EVENT_ID_HEADER),
        ]);
    if !origins.is_empty() {
        cors = cors.allow_origin(origins);
    }
    Ok(cors)
}

#[cfg(feature = "chat")]
async fn ix_stream(
    State(state): State<IxHttpState>,
    headers: HeaderMap,
    body: Bytes,
) -> response::Response {
    let request_context = safe_ix_request_context(&headers);
    let human = match state.verifier.verify(&headers).await {
        Ok(human) => human,
        Err(error) => return ix_transport_error(error, &request_context),
    };
    if let Err(error) = authorize_ix_origin(&state.policy, &headers, &human) {
        return ix_transport_error(error, &request_context);
    }

    let result = match single_ix_header(&headers, LAST_EVENT_ID_HEADER) {
        Some(Ok(cursor_value)) => {
            if !body.is_empty() {
                return ix_transport_error(IxTransportError::InvalidRequest, &request_context);
            }
            let cursor = match IxReplayCursor::parse(cursor_value) {
                Ok(cursor) => cursor,
                Err(error) => return ix_transport_error(error, &request_context),
            };
            state.service.replay(&human, &cursor).await
        }
        Some(Err(error)) => return ix_transport_error(error, &request_context),
        None => {
            let request = match serde_json::from_slice::<IxRunRequest>(&body)
                .map_err(|_| IxTransportError::InvalidRequest)
                .and_then(|request| {
                    request
                        .validate()
                        .map_err(IxTransportError::from)
                        .map(|_| request)
                }) {
                Ok(request) => request,
                Err(error) => return ix_transport_error(error, &request_context),
            };
            state.service.start(&request_context, &human, request).await
        }
    };
    match result {
        Ok(stream) => ix_sse_response(stream),
        Err(error) => ix_transport_error(error, &request_context),
    }
}

#[cfg(feature = "chat")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IxCancelRequest {
    schema_version: String,
    command_id: String,
}

#[cfg(feature = "chat")]
async fn ix_cancel(
    State(state): State<IxHttpState>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> response::Response {
    let request_context = safe_ix_request_context(&headers);
    let human = match state.verifier.verify(&headers).await {
        Ok(human) => human,
        Err(error) => return ix_transport_error(error, &request_context),
    };
    if let Err(error) = authorize_ix_origin(&state.policy, &headers, &human) {
        return ix_transport_error(error, &request_context);
    }
    if uuid::Uuid::parse_str(&run_id)
        .ok()
        .filter(|value| value.to_string() == run_id)
        .is_none()
    {
        return ix_transport_error(IxTransportError::InvalidRequest, &request_context);
    }
    let request = match serde_json::from_slice::<IxCancelRequest>(&body) {
        Ok(request)
            if request.schema_version == IX_CANCEL_REQUEST_SCHEMA_VERSION
                && canonical_ix_metadata(&request.command_id) =>
        {
            request
        }
        _ => return ix_transport_error(IxTransportError::InvalidRequest, &request_context),
    };
    match state
        .service
        .cancel(&human, &run_id, &request.command_id)
        .await
    {
        Ok(receipt) => ix_cancel_response(receipt),
        Err(error) => ix_transport_error(error, &request_context),
    }
}

#[cfg(feature = "chat")]
fn authorize_ix_origin(
    policy: &IxTransportPolicy,
    headers: &HeaderMap,
    human: &crate::ix::IxVerifiedHuman,
) -> Result<(), IxTransportError> {
    let origins = headers
        .get_all(header::ORIGIN)
        .iter()
        .map(|value| value.to_str().map_err(|_| IxTransportError::Forbidden))
        .collect::<Result<Vec<_>, _>>()?;
    policy.authorize_origin(&origins, human)
}

#[cfg(feature = "chat")]
fn safe_ix_request_context(headers: &HeaderMap) -> RequestContext {
    let request_id = single_ix_header(headers, REQUEST_ID_HEADER_NAME)
        .and_then(Result::ok)
        .filter(|value| canonical_uuid(value))
        .map(ToString::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let correlation_id = single_ix_header(headers, "x-correlation-id")
        .and_then(Result::ok)
        .filter(|value| canonical_uuid(value))
        .map(ToString::to_string)
        .unwrap_or_else(|| request_id.clone());
    RequestContext::new(request_id, correlation_id)
}

#[cfg(feature = "chat")]
async fn ix_safe_metadata_hook<B>(
    mut request: axum::http::Request<B>,
    next: middleware::Next<B>,
) -> response::Response {
    let request_id = uuid::Uuid::new_v4().to_string();
    let correlation_id = single_ix_header(request.headers(), "x-correlation-id")
        .and_then(Result::ok)
        .filter(|value| canonical_uuid(value))
        .map(ToString::to_string)
        .unwrap_or_else(|| request_id.clone());
    request.headers_mut().insert(
        HeaderName::from_static(REQUEST_ID_HEADER_NAME),
        HeaderValue::from_str(&request_id).expect("a UUID is a valid request header"),
    );
    request.headers_mut().insert(
        HeaderName::from_static("x-correlation-id"),
        HeaderValue::from_str(&correlation_id).expect("a UUID is a valid correlation header"),
    );
    next.run(request).await
}

#[cfg(feature = "chat")]
fn canonical_uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value)
        .ok()
        .is_some_and(|parsed| parsed.to_string() == value)
}

#[cfg(feature = "chat")]
fn canonical_ix_metadata(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.trim() == value
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'@' | b'-')
        })
}

#[cfg(feature = "chat")]
fn single_ix_header<'a>(
    headers: &'a HeaderMap,
    name: &'static str,
) -> Option<Result<&'a str, IxTransportError>> {
    let values = headers.get_all(name).iter().collect::<Vec<_>>();
    match values.as_slice() {
        [] => None,
        [value] => Some(value.to_str().map_err(|_| IxTransportError::InvalidRequest)),
        _ => Some(Err(IxTransportError::InvalidRequest)),
    }
}

#[cfg(feature = "chat")]
fn ix_sse_response(stream: crate::ix::IxTransportStream) -> response::Response {
    let events = stream
        .into_envelopes()
        .scan((), |_, envelope| async move {
            match envelope {
                Ok(envelope) => Some(ix_envelope_events(envelope)),
                Err(_) => None,
            }
        })
        .flat_map(stream::iter)
        .map(Ok::<Event, Infallible>);
    let mut response = Sse::new(events)
        .keep_alive(
            KeepAlive::new()
                .interval(IX_HEARTBEAT_INTERVAL)
                .text("heartbeat"),
        )
        .into_response();
    response.headers_mut().insert(
        HeaderName::from_static("x-accel-buffering"),
        HeaderValue::from_static("no"),
    );
    response
}

#[cfg(feature = "chat")]
fn ix_envelope_events(envelope: IxCommitEnvelope) -> Vec<Event> {
    let final_index = envelope.events.len().saturating_sub(1);
    envelope
        .events
        .into_iter()
        .enumerate()
        .filter_map(|(index, event)| {
            if event.schema_version != IX_EVENT_SCHEMA_VERSION || event.run_id != envelope.run_id {
                return None;
            }
            let data = serde_json::to_string(&event).ok()?;
            let event = Event::default().data(data);
            Some(if index == final_index {
                event.id(envelope.result_cursor.clone())
            } else {
                event
            })
        })
        .collect()
}

#[cfg(feature = "chat")]
fn ix_cancel_response(receipt: IxTransportCancelReceipt) -> response::Response {
    let status = match receipt.disposition() {
        IxCancelDisposition::Accepted => StatusCode::ACCEPTED,
        IxCancelDisposition::AlreadyRequested | IxCancelDisposition::AlreadyTerminal => {
            StatusCode::OK
        }
        IxCancelDisposition::NotFound => StatusCode::NOT_FOUND,
    };
    (status, Json(receipt)).into_response()
}

#[cfg(feature = "chat")]
fn ix_transport_error(error: IxTransportError, context: &RequestContext) -> response::Response {
    let (status, code) = match error {
        IxTransportError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
        IxTransportError::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
        IxTransportError::InvalidPolicy | IxTransportError::Unavailable => {
            (StatusCode::SERVICE_UNAVAILABLE, "unavailable")
        }
        IxTransportError::InvalidRequest => (StatusCode::BAD_REQUEST, "invalid_request"),
        IxTransportError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
        IxTransportError::Conflict => (StatusCode::CONFLICT, "conflict"),
        IxTransportError::StreamFailed => (StatusCode::INTERNAL_SERVER_ERROR, "stream_failed"),
    };
    (
        status,
        Json(json!({
            "error": code,
            "requestId": context.request_id,
            "correlationId": context.correlation_id
        })),
    )
        .into_response()
}

impl RuntimeRouteSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_info(mut self, router: Router) -> Self {
        self.info = Some(router);
        self
    }

    pub fn with_admin(mut self, router: Router) -> Self {
        self.admin = Some(router);
        self
    }

    pub fn with_product_ui(mut self, router: Router) -> Self {
        self.product_ui = Some(router);
        self
    }

    #[cfg(feature = "chat")]
    pub fn with_chat(mut self, router: Router) -> Self {
        self.chat = Some(router);
        self
    }

    #[cfg(feature = "chat")]
    pub fn with_ix_chat(mut self, routes: IxRouteGroup) -> Self {
        self.ix_chat = Some(routes);
        self
    }

    #[cfg(feature = "mcp")]
    pub fn with_mcp(mut self, router: Router) -> Self {
        self.mcp = Some(router);
        self
    }

    pub fn with_schema(mut self, router: Router) -> Self {
        self.schemas.push(router);
        self
    }
}

pub fn assemble_runtime_router(
    routes: RuntimeRouteSet,
    cors: CorsLayer,
    metrics: MetricsRegistry,
    security: &SecurityConfig,
) -> Router {
    assemble_runtime_router_for_mode(routes, cors, metrics, security, &RuntimeMode::all())
}

pub fn assemble_runtime_router_for_mode(
    routes: RuntimeRouteSet,
    cors: CorsLayer,
    metrics: MetricsRegistry,
    security: &SecurityConfig,
    mode: &RuntimeMode,
) -> Router {
    let RuntimeRouteSet {
        info,
        admin,
        product_ui,
        #[cfg(feature = "chat")]
        chat,
        #[cfg(feature = "chat")]
        ix_chat,
        #[cfg(feature = "mcp")]
        mcp,
        schemas,
    } = routes;
    let mut router = Router::new();
    if mode.enables(RuntimeIngressKind::Http) {
        if let Some(info) = info {
            router = router.merge(info);
        }
        if security.admin_ui_enabled {
            if let Some(admin) = admin {
                router = router.merge(admin);
            }
        }
        for schema in schemas {
            router = router.merge(schema);
        }
        if security.product_ui_enabled {
            if let Some(product_ui) = product_ui {
                router = router.merge(product_ui);
            }
        }
    }
    #[cfg(feature = "chat")]
    {
        if security.chat_enabled && mode.enables(RuntimeIngressKind::Chat) {
            if let Some(chat) = chat {
                router = router.merge(chat);
            }
        }
    }
    #[cfg(feature = "mcp")]
    {
        if security.mcp_enabled && mode.enables(RuntimeIngressKind::Mcp) {
            // MCP currently uses an HTTP transport, but it is an independent
            // runtime ingress surface for mode selection and future worker hosts.
            if let Some(mcp) = mcp {
                router = router.merge(mcp);
            }
        }
    }

    // Legacy routes retain their caller-supplied CORS profile, applied outside
    // the body/rate limiters so 413/429 rejections stay CORS-readable (parity
    // with the pre-IX layer order). IX receives the same common controls
    // independently, then its exact CORS layer wraps the rate-limit/body/error
    // path so preflight and every failure retain IX CORS.
    let timeout = request_timeout();
    let router = apply_common_runtime_layers_with_timeout(
        router,
        Some(cors),
        metrics.clone(),
        security,
        timeout,
    );
    #[cfg(feature = "chat")]
    {
        if security.chat_enabled && mode.enables(RuntimeIngressKind::Chat) {
            if let Some(ix_chat) = ix_chat {
                let ix_router = apply_common_runtime_layers_with_timeout(
                    ix_chat.router,
                    None,
                    metrics,
                    security,
                    timeout,
                )
                .layer(middleware::from_fn(ix_safe_metadata_hook))
                .layer(ix_chat.cors);
                return router.merge(ix_router);
            }
        }
    }
    router
}

pub fn apply_runtime_layers(
    router: Router,
    cors: CorsLayer,
    metrics: MetricsRegistry,
    security: &SecurityConfig,
) -> Router {
    apply_runtime_layers_with_timeout(router, cors, metrics, security, request_timeout())
}

fn apply_runtime_layers_with_timeout(
    router: Router,
    cors: CorsLayer,
    metrics: MetricsRegistry,
    security: &SecurityConfig,
    request_timeout: Duration,
) -> Router {
    apply_common_runtime_layers_with_timeout(router, Some(cors), metrics, security, request_timeout)
}

fn apply_common_runtime_layers_with_timeout(
    router: Router,
    cors: Option<CorsLayer>,
    metrics: MetricsRegistry,
    security: &SecurityConfig,
    request_timeout: Duration,
) -> Router {
    let request_id_header = HeaderName::from_static(REQUEST_ID_HEADER_NAME);
    let mut router = router
        .layer(DefaultBodyLimit::max(security.request_body_limit_bytes))
        .layer(middleware::from_fn_with_state(
            RateLimiterState::new(security.rate_limit_per_second, security.rate_limit_burst),
            rate_limit_hook,
        ));
    if let Some(cors) = cors {
        // Caller-supplied CORS wraps the body/rate limiters so their 413/429
        // rejections carry CORS headers (legacy/product parity with the
        // pre-IX layer order). IX passes None here and pins its exact CORS
        // outermost at the call site instead.
        router = router.layer(cors);
    }
    router = router
        .layer(middleware::from_fn_with_state(metrics, metrics_hook))
        .layer(middleware::from_fn(trace_context_hook))
        .layer(PropagateRequestIdLayer::new(request_id_header.clone()))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(http_make_span)
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(SetRequestIdLayer::new(request_id_header, MakeRequestUuid));

    // Security response headers (applied to every response, including errors).
    for (name, value) in security_response_headers() {
        router = router.layer(SetResponseHeaderLayer::if_not_present(name, value));
    }

    // Bound total request time; layered outside the handler stack so slow or stuck
    // handlers cannot hold a connection open indefinitely.
    router = router.layer(TimeoutLayer::new(request_timeout));

    // Panic isolation must be the OUTERMOST layer so a panic anywhere below it is
    // converted into a clean 500 instead of dropping the connection.
    router.layer(CatchPanicLayer::custom(panic_response))
}

/// Total request timeout, configurable via `APP_REQUEST_TIMEOUT_MS` (default 30000ms).
fn request_timeout() -> Duration {
    let ms = env::var("APP_REQUEST_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_REQUEST_TIMEOUT_MS);
    Duration::from_millis(ms)
}

/// Clean 500 returned when a downstream handler panics. The body is a fixed,
/// secret-free payload (fail-closed: never leak panic messages to clients).
fn panic_response(_err: Box<dyn std::any::Any + Send + 'static>) -> Response<axum::body::BoxBody> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(header::CONTENT_TYPE, "application/json")
        .body(boxed(Full::from(
            r#"{"errors":[{"message":"internal server error"}]}"#,
        )))
        .expect("static panic response is valid")
}

/// Conservative security headers applied to all responses.
///
/// HSTS is gated to managed/TLS environments so local HTTP development is not
/// pinned to HTTPS. Managed environments get a stricter production CSP; local
/// keeps a convenience CSP for GraphiQL/admin exploration.
fn security_response_headers() -> Vec<(HeaderName, HeaderValue)> {
    let mut headers = vec![
        (
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ),
        (header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY")),
        (
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ),
        (
            header::CONTENT_SECURITY_POLICY,
            content_security_policy(is_managed_tls_env()),
        ),
    ];

    if is_managed_tls_env() {
        headers.push((
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        ));
    }

    headers
}

fn content_security_policy(managed_tls: bool) -> HeaderValue {
    if managed_tls {
        HeaderValue::from_static(
            "default-src 'self'; \
             script-src 'self'; \
             style-src 'self'; \
             img-src 'self' data:; \
             font-src 'self' data:; \
             connect-src 'self'; \
             object-src 'none'; \
             base-uri 'self'; \
             frame-ancestors 'none'; \
             form-action 'self'",
        )
    } else {
        HeaderValue::from_static(
            "default-src 'self'; \
             script-src 'self' 'unsafe-inline' 'unsafe-eval' https:; \
             style-src 'self' 'unsafe-inline' https:; \
             img-src 'self' data: https:; \
             font-src 'self' data: https:; \
             connect-src 'self' https: wss:; \
             object-src 'none'; \
             base-uri 'self'; \
             frame-ancestors 'none'",
        )
    }
}

/// True for managed (non-`local`) environments where traffic is served over TLS.
fn is_managed_tls_env() -> bool {
    env::var("ENV_NAME")
        .map(|value| !value.is_empty() && value != "local")
        .unwrap_or(false)
}

pub fn runtime_graphql_schema_routes<Query, Mutation, Subscription>(
    path: &'static str,
    schema: Schema<Query, Mutation, Subscription>,
    introspection_schema: Schema<Query, Mutation, Subscription>,
    app_state: RuntimeAuthState,
    security: SecurityConfig,
) -> Router
where
    Query: ObjectType + 'static,
    Mutation: ObjectType + 'static,
    Subscription: SubscriptionType + 'static,
{
    Router::new()
        .route(path, post(graphql_handler::<Query, Mutation, Subscription>))
        .route(
            path,
            get(move || async move { response::Html(graphiql::html(path)) }),
        )
        .layer(Extension((schema, introspection_schema)))
        .with_state(RuntimeGraphQLState {
            auth: app_state,
            security,
        })
}

#[derive(Clone)]
struct RuntimeGraphQLState {
    auth: RuntimeAuthState,
    security: SecurityConfig,
}

type RuntimeGraphQLSchemas<Query, Mutation, Subscription> = (
    Schema<Query, Mutation, Subscription>,
    Schema<Query, Mutation, Subscription>,
);

async fn graphql_handler<Query, Mutation, Subscription>(
    State(state): State<RuntimeGraphQLState>,
    headers: HeaderMap,
    Extension((schema, introspection_schema)): Extension<
        RuntimeGraphQLSchemas<Query, Mutation, Subscription>,
    >,
    req: GraphQLRequest,
) -> GraphQLResponse
where
    Query: ObjectType + 'static,
    Mutation: ObjectType + 'static,
    Subscription: SubscriptionType + 'static,
{
    let inner_req: AsyncGraphQLRequest = req.into_inner();
    let request_context = RequestContext::from_headers(&headers);
    let is_introspection =
        inner_req.query.contains("__schema") || inner_req.query.contains("__type");
    let jwt_extractor =
        match RuntimeJwtExtractor::new(state.auth.clone(), headers, is_introspection).await {
            Ok(extractor) => {
                if is_introspection {
                    if let Err(error) =
                        authorize_graphql_introspection(&state.security, extractor.user.as_deref())
                    {
                        return GraphQLResponse::from(async_graphql::Response::from_errors(vec![
                            graphql_error_with_context(error.to_string(), &request_context),
                        ]));
                    }
                }
                extractor
            }
            Err(error) => {
                return GraphQLResponse::from(async_graphql::Response::from_errors(vec![
                    graphql_error_with_context(error.to_string(), &request_context),
                ]))
            }
        };
    let req = inner_req.data(jwt_extractor);
    let schema = if is_introspection {
        &introspection_schema
    } else {
        &schema
    };
    annotate_graphql_response(schema.execute(req).await, &request_context).into()
}

fn authorize_graphql_introspection(
    security: &SecurityConfig,
    user: Option<&UserAuth>,
) -> Result<(), crate::RuntimeError> {
    if !security.graphql_introspection_enabled {
        return Err(crate::RuntimeError::NotAuthorized);
    }
    let user = user.ok_or(crate::RuntimeError::NotAuthorized)?;
    if principal_matches(
        user,
        &security.graphql_introspection_required_roles,
        &security.graphql_introspection_required_scopes,
    ) {
        Ok(())
    } else {
        Err(crate::RuntimeError::AccessDenied)
    }
}

fn principal_matches(user: &UserAuth, roles: &[String], scopes: &[String]) -> bool {
    roles
        .iter()
        .any(|required| user.roles.iter().any(|role| role == required))
        || scopes
            .iter()
            .any(|required| user.scopes.iter().any(|scope| scope == required))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "chat")]
    use crate::ix::IxClientProfile;
    use axum::{
        body::Body,
        http::{Method, Request},
        routing::{get, post},
    };
    #[cfg(feature = "chat")]
    use axum::{
        body::HttpBody as _,
        response::sse::{Event, Sse},
    };
    use tower::ServiceExt;

    fn security(
        admin_ui_enabled: bool,
        #[allow(unused_variables)] mcp_enabled: bool,
    ) -> SecurityConfig {
        SecurityConfig {
            admin_ui_enabled,
            admin_troubleshooting_enabled: false,
            product_ui_enabled: true,
            #[cfg(feature = "chat")]
            chat_enabled: false,
            #[cfg(feature = "chat")]
            chat_required_roles: vec!["admin".to_string()],
            #[cfg(feature = "chat")]
            chat_required_scopes: vec![],
            #[cfg(feature = "chat")]
            chat_prompt_audit_enabled: false,
            #[cfg(feature = "chat")]
            chat_prompt_audit_siem_enabled: false,
            #[cfg(feature = "chat")]
            chat_prompt_audit_sink: None,
            #[cfg(feature = "chat")]
            chat_prompt_audit_retention_days: 90,
            #[cfg(feature = "chat")]
            chat_kill_switch_active: false,
            #[cfg(feature = "mcp")]
            mcp_enabled,
            #[cfg(feature = "mcp")]
            mcp_mutations_enabled: false,
            #[cfg(feature = "mcp")]
            mcp_allowed_origins: vec![],
            #[cfg(feature = "mcp")]
            mcp_max_result_bytes: 256 * 1024,
            #[cfg(feature = "mcp")]
            mcp_max_resource_bytes: 256 * 1024,
            #[cfg(feature = "mcp")]
            mcp_max_batch_items: 20,
            #[cfg(feature = "mcp")]
            mcp_required_roles: vec!["admin".to_string()],
            #[cfg(feature = "mcp")]
            mcp_required_scopes: vec![],
            #[cfg(feature = "mcp")]
            mcp_privileged_tool_roles: vec!["admin".to_string()],
            #[cfg(feature = "mcp")]
            mcp_privileged_tool_scopes: vec!["appfw:mcp.admin".to_string()],
            graphql_introspection_enabled: false,
            graphql_introspection_required_roles: vec!["admin".to_string()],
            graphql_introspection_required_scopes: vec![
                "developer".to_string(),
                "appfw:developer".to_string(),
                "appfw:graphql.introspection".to_string(),
            ],
            graphql_max_depth: 12,
            graphql_max_complexity: 500,
            request_body_limit_bytes: 1024 * 1024,
            rate_limit_per_second: 100,
            rate_limit_burst: 100,
        }
    }

    fn ok_route(path: &'static str) -> Router {
        Router::new().route(path, get(|| async { "ok" }))
    }

    fn body_route(path: &'static str) -> Router {
        Router::new().route(path, post(|_body: String| async { "ok" }))
    }

    #[cfg(feature = "chat")]
    fn ix_browser_policy() -> IxTransportPolicy {
        IxTransportPolicy::new(
            "https://issuer.example.com",
            "release-1",
            [crate::ix::IxClientBinding::new(
                "nexus-browser",
                "api://nexus-ix",
                IxClientProfile::Browser,
                ["https://nexus.example.com".to_string()],
            )
            .expect("browser binding")],
            ["ix.user".to_string()],
            ["ix.run".to_string()],
        )
        .expect("IX transport policy")
    }

    #[cfg(feature = "chat")]
    fn ix_preflight(request_headers: &str) -> Request<Body> {
        Request::builder()
            .method(Method::OPTIONS)
            .uri(IX_STREAM_PATH)
            .header(header::ORIGIN, "https://nexus.example.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .header(header::ACCESS_CONTROL_REQUEST_HEADERS, request_headers)
            .body(Body::empty())
            .expect("preflight request")
    }

    #[cfg(feature = "chat")]
    #[tokio::test]
    async fn ix_post_cors_allows_exact_last_event_id_and_rejects_unapproved_headers() {
        let cors = ix_cors_layer(&ix_browser_policy()).expect("IX CORS policy");
        let router = Router::new()
            .route(IX_STREAM_PATH, post(|| async { StatusCode::NO_CONTENT }))
            .layer(cors);
        let allowed = router
            .clone()
            .oneshot(ix_preflight("authorization, content-type, last-event-id"))
            .await
            .expect("allowed preflight");
        assert_eq!(allowed.status(), StatusCode::OK);
        assert_eq!(
            allowed.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&HeaderValue::from_static("https://nexus.example.com"))
        );
        let allowed_headers = allowed
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_HEADERS)
            .expect("allowed headers")
            .to_str()
            .expect("ASCII headers")
            .to_ascii_lowercase();
        assert!(allowed_headers.contains("authorization"));
        assert!(allowed_headers.contains("content-type"));
        assert!(allowed_headers.contains("last-event-id"));
        assert!(!allowed_headers.contains('*'));
        assert!(allowed
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS)
            .is_none());

        let rejected = router
            .oneshot(ix_preflight("authorization, x-appfw-ix-client-profile"))
            .await
            .expect("closed preflight");
        let rejected_headers = rejected
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_HEADERS)
            .expect("closed allowed-header response")
            .to_str()
            .expect("ASCII headers")
            .to_ascii_lowercase();
        assert!(!rejected_headers.contains("x-appfw-ix-client-profile"));
        assert!(!rejected_headers.contains('*'));
    }

    #[cfg(feature = "chat")]
    #[tokio::test]
    async fn ix_sse_frames_complete_envelope_with_only_final_event_cursor() {
        use crate::{
            extension::RuntimePrincipalType,
            ix::{
                IxCancellationStage, IxEvent, IxEventPayload, IxPrincipalBinding,
                IX_EVENT_SCHEMA_VERSION,
            },
        };

        let run_id = "00000000-0000-4000-8000-000000000001".to_string();
        let actor = IxPrincipalBinding {
            tenant_id: "tenant-1".to_string(),
            subject: "appfw-ix-orchestrator".to_string(),
            principal_type: RuntimePrincipalType::Agent,
            on_behalf_of: None,
        };
        let event = |sequence, stage| IxEvent {
            schema_version: IX_EVENT_SCHEMA_VERSION.to_string(),
            event_id: format!("event-{sequence}"),
            run_id: run_id.clone(),
            sequence,
            occurred_at: "2026-08-15T00:00:00Z".to_string(),
            actor: actor.clone(),
            payload: IxEventPayload::CancellationProgress {
                command_id: "cancel-1".to_string(),
                stage,
            },
        };
        let cursor = format!("ix1.{run_id}.2");
        let envelope = IxCommitEnvelope {
            schema_version: "appfw.ix_commit@1".to_string(),
            commit_id: "commit-1".to_string(),
            committed_by: "appfw-runtime".to_string(),
            run_id: run_id.clone(),
            base_cursor: "ix1.00000000-0000-4000-8000-000000000001.1".to_string(),
            result_cursor: cursor.clone(),
            base_revision: 1,
            result_revision: 2,
            events: vec![
                event(2, IxCancellationStage::Stopping),
                event(3, IxCancellationStage::Draining),
            ],
            next_safe_move: "Allow cancellation to finish.".to_string(),
            base_snapshot_sha256: "base-digest".to_string(),
            result_snapshot_sha256: "result-digest".to_string(),
        };
        let events = ix_envelope_events(envelope)
            .into_iter()
            .map(Ok::<Event, Infallible>);
        let response = Sse::new(stream::iter(events)).into_response();
        let mut body = response.into_body();
        let mut bytes = Vec::new();
        while let Some(chunk) = body.data().await {
            bytes.extend_from_slice(&chunk.expect("SSE frame"));
        }
        let text = String::from_utf8(bytes).expect("UTF-8 SSE");
        let frames = text
            .split("\n\n")
            .filter(|frame| frame.contains("data:"))
            .collect::<Vec<_>>();
        assert_eq!(frames.len(), 2);
        assert!(!frames[0].contains("\nid:"));
        assert_eq!(frames[1].matches("\nid:").count(), 1);
        assert!(frames[1].contains(&format!("\nid:{cursor}")));
        assert_eq!(text.matches("data:").count(), 2);
    }

    #[cfg(feature = "chat")]
    #[tokio::test]
    async fn ix_cors_wraps_preflight_and_rate_limit_failures_without_widening_legacy_get() {
        let mut security = security(false, false);
        security.chat_enabled = true;
        security.rate_limit_per_second = 1;
        security.rate_limit_burst = 1;
        let ix_group = IxRouteGroup::from_profiled_router(
            Router::new().route(IX_STREAM_PATH, post(|| async { StatusCode::NO_CONTENT })),
            ix_cors_layer(&ix_browser_policy()).expect("IX CORS policy"),
        );
        let legacy_cors = CorsLayer::new()
            .allow_methods([Method::GET])
            .allow_origin(HeaderValue::from_static("https://legacy.example.com"));
        let router = assemble_runtime_router_for_mode(
            RuntimeRouteSet::new()
                .with_chat(ok_route(IX_STREAM_PATH))
                .with_ix_chat(ix_group),
            legacy_cors,
            MetricsRegistry::new("test", "test"),
            &security,
            &RuntimeMode::chat(),
        );

        let preflight = router
            .clone()
            .oneshot(ix_preflight("authorization, last-event-id"))
            .await
            .expect("preflight response");
        assert_eq!(preflight.status(), StatusCode::OK);
        assert_eq!(
            preflight.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&HeaderValue::from_static("https://nexus.example.com"))
        );

        let post_request = || {
            Request::builder()
                .method(Method::POST)
                .uri(IX_STREAM_PATH)
                .header(header::ORIGIN, "https://nexus.example.com")
                .header(header::AUTHORIZATION, "Bearer rate-limit-proof")
                .body(Body::empty())
                .expect("IX POST")
        };
        let first = router
            .clone()
            .oneshot(post_request())
            .await
            .expect("first POST");
        let second = router
            .clone()
            .oneshot(post_request())
            .await
            .expect("second POST");
        assert_eq!(first.status(), StatusCode::NO_CONTENT);
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            second.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&HeaderValue::from_static("https://nexus.example.com"))
        );

        let legacy = router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(IX_STREAM_PATH)
                    .header(header::ORIGIN, "https://legacy.example.com")
                    .body(Body::empty())
                    .expect("legacy GET"),
            )
            .await
            .expect("legacy response");
        assert_eq!(legacy.status(), StatusCode::OK);
        assert_ne!(
            legacy.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&HeaderValue::from_static("https://nexus.example.com"))
        );
    }

    #[cfg(feature = "chat")]
    fn panic_route(path: &'static str) -> Router {
        Router::new().route(
            path,
            get(|| async move {
                panic!("chat transport panic proof");
                #[allow(unreachable_code)]
                "never"
            }),
        )
    }

    #[cfg(feature = "chat")]
    fn slow_route(path: &'static str) -> Router {
        Router::new().route(
            path,
            get(|| async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                "slow"
            }),
        )
    }

    #[cfg(feature = "chat")]
    fn sse_soak_route(path: &'static str) -> Router {
        Router::new().route(
            path,
            get(|| async {
                let events = futures_util::stream::unfold(0_u64, |sequence| async move {
                    if sequence > 0 {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    Some((
                        Ok::<Event, std::convert::Infallible>(
                            Event::default()
                                .event("heartbeat")
                                .id(format!("soak:{sequence}"))
                                .data(format!(r#"{{"sequence":{sequence}}}"#)),
                        ),
                        sequence + 1,
                    ))
                });
                Sse::new(events)
            }),
        )
    }

    async fn status(router: Router, path: &str) -> axum::http::StatusCode {
        router
            .oneshot(
                Request::builder()
                    .uri(path)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response")
            .status()
    }

    async fn request_status(router: Router, request: Request<Body>) -> axum::http::StatusCode {
        router.oneshot(request).await.expect("response").status()
    }

    #[cfg(feature = "chat")]
    async fn next_body_chunk(body: &mut axum::body::BoxBody) -> String {
        let bytes = body
            .data()
            .await
            .expect("next body chunk")
            .expect("body chunk is successful");
        String::from_utf8(bytes.to_vec()).expect("body chunk is utf-8")
    }

    #[tokio::test]
    async fn mounts_info_and_schema_routes_without_optional_surfaces() {
        let security = security(false, false);
        let route_set = RuntimeRouteSet::new()
            .with_info(ok_route("/info"))
            .with_admin(ok_route("/admin"))
            .with_schema(ok_route("/crm"));
        #[cfg(feature = "mcp")]
        let route_set = route_set.with_mcp(ok_route("/mcp"));
        let router = assemble_runtime_router(
            route_set,
            CorsLayer::new(),
            MetricsRegistry::new("test", "test"),
            &security,
        );

        assert_eq!(status(router.clone(), "/info").await, 200);
        assert_eq!(status(router.clone(), "/crm").await, 200);
        assert_eq!(status(router.clone(), "/admin").await, 404);
        assert_eq!(status(router, "/mcp").await, 404);
    }

    #[cfg(feature = "mcp")]
    #[tokio::test]
    async fn mounts_admin_and_mcp_only_when_enabled() {
        let security = security(true, true);
        let router = assemble_runtime_router(
            RuntimeRouteSet::new()
                .with_admin(ok_route("/admin"))
                .with_mcp(ok_route("/mcp")),
            CorsLayer::new(),
            MetricsRegistry::new("test", "test"),
            &security,
        );

        assert_eq!(status(router.clone(), "/admin").await, 200);
        assert_eq!(status(router, "/mcp").await, 200);
    }

    #[cfg(feature = "mcp")]
    #[tokio::test]
    async fn runtime_mode_can_disable_mcp_route() {
        let security = security(true, true);
        let router = assemble_runtime_router_for_mode(
            RuntimeRouteSet::new()
                .with_info(ok_route("/info"))
                .with_mcp(ok_route("/mcp"))
                .with_schema(ok_route("/crm")),
            CorsLayer::new(),
            MetricsRegistry::new("test", "test"),
            &security,
            &RuntimeMode::http(),
        );

        assert_eq!(status(router.clone(), "/info").await, 200);
        assert_eq!(status(router.clone(), "/crm").await, 200);
        assert_eq!(status(router, "/mcp").await, 404);
    }

    #[cfg(feature = "mcp")]
    #[tokio::test]
    async fn runtime_mode_can_mount_only_mcp_transport() {
        let security = security(true, true);
        let router = assemble_runtime_router_for_mode(
            RuntimeRouteSet::new()
                .with_info(ok_route("/info"))
                .with_admin(ok_route("/admin"))
                .with_mcp(ok_route("/mcp"))
                .with_schema(ok_route("/crm")),
            CorsLayer::new(),
            MetricsRegistry::new("test", "test"),
            &security,
            &RuntimeMode::mcp(),
        );

        assert_eq!(status(router.clone(), "/info").await, 404);
        assert_eq!(status(router.clone(), "/admin").await, 404);
        assert_eq!(status(router.clone(), "/crm").await, 404);
        assert_eq!(status(router, "/mcp").await, 200);
    }

    #[cfg(feature = "chat")]
    #[tokio::test]
    async fn mounts_chat_only_when_enabled() {
        let mut security = security(false, false);
        let disabled = assemble_runtime_router_for_mode(
            RuntimeRouteSet::new().with_chat(ok_route("/chat/stream")),
            CorsLayer::new(),
            MetricsRegistry::new("test", "test"),
            &security,
            &RuntimeMode::chat(),
        );
        assert_eq!(status(disabled, "/chat/stream").await, 404);

        security.chat_enabled = true;
        let enabled = assemble_runtime_router_for_mode(
            RuntimeRouteSet::new().with_chat(ok_route("/chat/stream")),
            CorsLayer::new(),
            MetricsRegistry::new("test", "test"),
            &security,
            &RuntimeMode::chat(),
        );
        assert_eq!(status(enabled, "/chat/stream").await, 200);
    }

    #[cfg(feature = "chat")]
    #[tokio::test]
    async fn runtime_mode_can_disable_chat_transport() {
        let mut security = security(false, false);
        security.chat_enabled = true;
        let router = assemble_runtime_router_for_mode(
            RuntimeRouteSet::new()
                .with_info(ok_route("/info"))
                .with_chat(ok_route("/chat/stream")),
            CorsLayer::new(),
            MetricsRegistry::new("test", "test"),
            &security,
            &RuntimeMode::http(),
        );

        assert_eq!(status(router.clone(), "/info").await, 200);
        assert_eq!(status(router, "/chat/stream").await, 404);
    }

    #[tokio::test]
    async fn runtime_router_applies_configured_request_body_limit() {
        let mut security = security(false, false);
        security.request_body_limit_bytes = 8;
        let router = assemble_runtime_router(
            RuntimeRouteSet::new().with_schema(body_route("/crm")),
            CorsLayer::new(),
            MetricsRegistry::new("test", "test"),
            &security,
        );

        let status = request_status(
            router,
            Request::builder()
                .method(Method::POST)
                .uri("/crm")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"query":"{ value }"}"#))
                .expect("request"),
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn runtime_router_applies_configured_rate_limit() {
        let mut security = security(false, false);
        security.rate_limit_per_second = 1;
        security.rate_limit_burst = 1;
        let router = assemble_runtime_router(
            RuntimeRouteSet::new().with_schema(ok_route("/crm")),
            CorsLayer::new(),
            MetricsRegistry::new("test", "test"),
            &security,
        );

        let first = request_status(
            router.clone(),
            Request::builder()
                .method(Method::GET)
                .uri("/crm")
                .header("authorization", "Bearer runtime-router-rate-limit")
                .body(Body::empty())
                .expect("request"),
        )
        .await;
        let second = request_status(
            router,
            Request::builder()
                .method(Method::GET)
                .uri("/crm")
                .header("authorization", "Bearer runtime-router-rate-limit")
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(first, axum::http::StatusCode::OK);
        assert_eq!(second, axum::http::StatusCode::TOO_MANY_REQUESTS);
    }

    /// Review condition (b-leaf-a-local-frozen-candidate-review-20260816,
    /// Finding 1): the caller-supplied CORS layer must wrap the body/rate
    /// limiters so legacy/product-route 429 and 413 rejections stay
    /// CORS-readable in browsers, on both `assemble_runtime_router` and the
    /// public `apply_runtime_layers` path. IX routes pin their own CORS
    /// failure-path behavior separately.
    #[tokio::test]
    async fn legacy_rate_and_body_limit_rejections_carry_caller_cors() {
        let legacy_origin = HeaderValue::from_static("https://legacy.example.com");
        let legacy_cors = || {
            CorsLayer::new()
                .allow_methods([Method::GET, Method::POST])
                .allow_origin(legacy_origin.clone())
        };

        // 429 short-circuited by the rate limiter carries the caller CORS header.
        let mut rate_security = security(false, false);
        rate_security.rate_limit_per_second = 1;
        rate_security.rate_limit_burst = 1;
        let rate_router = assemble_runtime_router(
            RuntimeRouteSet::new().with_schema(ok_route("/crm")),
            legacy_cors(),
            MetricsRegistry::new("test", "test"),
            &rate_security,
        );
        let rate_request = || {
            Request::builder()
                .method(Method::GET)
                .uri("/crm")
                .header(header::ORIGIN, "https://legacy.example.com")
                .header(header::AUTHORIZATION, "Bearer legacy-cors-rate-limit")
                .body(Body::empty())
                .expect("rate-limited request")
        };
        let first = rate_router
            .clone()
            .oneshot(rate_request())
            .await
            .expect("first response");
        assert_eq!(first.status(), StatusCode::OK);
        let limited = rate_router
            .oneshot(rate_request())
            .await
            .expect("rate-limited response");
        assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            limited.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&legacy_origin)
        );

        // 413 from the configured body limit carries the caller CORS header.
        let mut body_security = security(false, false);
        body_security.request_body_limit_bytes = 8;
        let body_router = assemble_runtime_router(
            RuntimeRouteSet::new().with_schema(body_route("/crm")),
            legacy_cors(),
            MetricsRegistry::new("test", "test"),
            &body_security,
        );
        let oversized = body_router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/crm")
                    .header(header::ORIGIN, "https://legacy.example.com")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"query":"{ value }"}"#))
                    .expect("oversized request"),
            )
            .await
            .expect("body-limited response");
        assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(
            oversized.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&legacy_origin)
        );

        // The public `apply_runtime_layers` product path keeps the same parity.
        let mut layered_security = security(false, false);
        layered_security.rate_limit_per_second = 1;
        layered_security.rate_limit_burst = 1;
        let layered = apply_runtime_layers(
            ok_route("/product"),
            legacy_cors(),
            MetricsRegistry::new("test", "test"),
            &layered_security,
        );
        let layered_request = || {
            Request::builder()
                .method(Method::GET)
                .uri("/product")
                .header(header::ORIGIN, "https://legacy.example.com")
                .header(header::AUTHORIZATION, "Bearer layered-cors-rate-limit")
                .body(Body::empty())
                .expect("layered request")
        };
        let first_layered = layered
            .clone()
            .oneshot(layered_request())
            .await
            .expect("first layered response");
        assert_eq!(first_layered.status(), StatusCode::OK);
        let layered_limited = layered
            .oneshot(layered_request())
            .await
            .expect("layered rate-limited response");
        assert_eq!(layered_limited.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            layered_limited
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&legacy_origin)
        );
    }

    #[cfg(feature = "chat")]
    #[tokio::test]
    async fn runtime_layers_rate_limit_chat_transport() {
        let mut security = security(false, false);
        security.chat_enabled = true;
        security.rate_limit_per_second = 1;
        security.rate_limit_burst = 1;
        let router = assemble_runtime_router_for_mode(
            RuntimeRouteSet::new().with_chat(ok_route("/chat/stream")),
            CorsLayer::new(),
            MetricsRegistry::new("test", "test"),
            &security,
            &RuntimeMode::chat(),
        );

        let first = request_status(
            router.clone(),
            Request::builder()
                .method(Method::GET)
                .uri("/chat/stream")
                .header("authorization", "Bearer runtime-router-chat-rate-limit")
                .body(Body::empty())
                .expect("request"),
        )
        .await;
        let second = request_status(
            router,
            Request::builder()
                .method(Method::GET)
                .uri("/chat/stream")
                .header("authorization", "Bearer runtime-router-chat-rate-limit")
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(first, axum::http::StatusCode::OK);
        assert_eq!(second, axum::http::StatusCode::TOO_MANY_REQUESTS);
    }

    #[cfg(feature = "chat")]
    #[tokio::test]
    async fn runtime_layers_catch_chat_transport_panics() {
        let mut security = security(false, false);
        security.chat_enabled = true;
        let router = assemble_runtime_router_for_mode(
            RuntimeRouteSet::new().with_chat(panic_route("/chat/stream")),
            CorsLayer::new(),
            MetricsRegistry::new("test", "test"),
            &security,
            &RuntimeMode::chat(),
        );

        assert_eq!(
            status(router, "/chat/stream").await,
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[cfg(feature = "chat")]
    #[tokio::test]
    async fn runtime_layers_timeout_chat_transport() {
        let mut security = security(false, false);
        security.chat_enabled = true;
        let router = apply_runtime_layers_with_timeout(
            slow_route("/chat/stream"),
            CorsLayer::new(),
            MetricsRegistry::new("test", "test"),
            &security,
            Duration::from_millis(1),
        );

        assert_eq!(
            status(router, "/chat/stream").await,
            axum::http::StatusCode::REQUEST_TIMEOUT
        );
    }

    #[cfg(feature = "chat")]
    #[tokio::test]
    async fn runtime_layers_allow_long_running_sse_body_after_response_head() {
        let mut security = security(false, false);
        security.chat_enabled = true;
        let router = apply_runtime_layers_with_timeout(
            sse_soak_route("/chat/stream"),
            CorsLayer::new(),
            MetricsRegistry::new("test", "test"),
            &security,
            Duration::from_millis(1),
        );

        let mut response = router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/chat/stream")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert!(next_body_chunk(response.body_mut())
            .await
            .contains(r#""sequence":0"#));
        assert!(tokio::time::timeout(
            Duration::from_millis(100),
            next_body_chunk(response.body_mut())
        )
        .await
        .expect("second SSE event arrives after response-head timeout")
        .contains(r#""sequence":1"#));
        assert!(tokio::time::timeout(
            Duration::from_millis(100),
            next_body_chunk(response.body_mut())
        )
        .await
        .expect("third SSE event arrives after response-head timeout")
        .contains(r#""sequence":2"#));
    }

    fn test_user(roles: &[&str], scopes: &[&str]) -> UserAuth {
        UserAuth::human(
            "tenant-1",
            "casey",
            "UTC",
            roles.iter().map(|role| (*role).to_string()).collect(),
            scopes.iter().map(|scope| (*scope).to_string()).collect(),
            "test-token",
        )
    }

    #[test]
    fn graphql_introspection_rejects_no_auth_when_disabled_or_missing_user() {
        let security = security(false, false);

        assert!(matches!(
            authorize_graphql_introspection(&security, None),
            Err(crate::RuntimeError::NotAuthorized)
        ));

        let mut enabled = security;
        enabled.graphql_introspection_enabled = true;
        assert!(matches!(
            authorize_graphql_introspection(&enabled, None),
            Err(crate::RuntimeError::NotAuthorized)
        ));
    }

    #[test]
    fn graphql_introspection_rejects_fake_bearer_or_unprivileged_user() {
        let mut security = security(false, false);
        security.graphql_introspection_enabled = true;

        assert!(matches!(
            authorize_graphql_introspection(&security, None),
            Err(crate::RuntimeError::NotAuthorized)
        ));
        assert!(matches!(
            authorize_graphql_introspection(&security, Some(&test_user(&["analyst"], &[]))),
            Err(crate::RuntimeError::AccessDenied)
        ));
    }

    #[test]
    fn graphql_introspection_allows_admin_or_developer_scope() {
        let mut security = security(false, false);
        security.graphql_introspection_enabled = true;

        authorize_graphql_introspection(&security, Some(&test_user(&["admin"], &[])))
            .expect("admin role should allow introspection");
        authorize_graphql_introspection(
            &security,
            Some(&test_user(&["analyst"], &["appfw:developer"])),
        )
        .expect("developer scope should allow introspection");
    }

    #[test]
    fn managed_csp_excludes_local_graphiql_conveniences() {
        let header = content_security_policy(true);
        let csp = header.to_str().expect("static CSP should be valid");

        assert!(!csp.contains("'unsafe-inline'"));
        assert!(!csp.contains("'unsafe-eval'"));
        assert!(csp.contains("script-src 'self'"));
        assert!(csp.contains("connect-src 'self'"));
    }

    #[test]
    fn local_csp_keeps_graphiql_conveniences() {
        let header = content_security_policy(false);
        let csp = header.to_str().expect("static CSP should be valid");

        assert!(csp.contains("'unsafe-inline'"));
        assert!(csp.contains("'unsafe-eval'"));
        assert!(csp.contains("wss:"));
    }
}
