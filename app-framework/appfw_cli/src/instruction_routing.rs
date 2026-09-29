use serde_json::{json, Value};
use std::{fmt, str::FromStr};

pub const INSTRUCTION_ROUTE_SCHEMA: &str = "appfw_instruction_route@1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Namespace {
    Framework,
    Product,
}

impl Namespace {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Framework => "framework",
            Self::Product => "product",
        }
    }
}

impl FromStr for Namespace {
    type Err = InstructionRouteError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "framework" => Ok(Self::Framework),
            "product" => Ok(Self::Product),
            _ => Err(InstructionRouteError::unknown(
                "namespace",
                value,
                &["framework", "product"],
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeClass {
    A,
    B,
    C,
    D,
}

impl ChangeClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
        }
    }
}

impl FromStr for ChangeClass {
    type Err = InstructionRouteError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "A" => Ok(Self::A),
            "B" => Ok(Self::B),
            "C" => Ok(Self::C),
            "D" => Ok(Self::D),
            _ => Err(InstructionRouteError::unknown(
                "change_class",
                value,
                &["A", "B", "C", "D"],
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskRoute {
    FrameworkDocsIa,
    FrameworkGenerator,
    FrameworkRuntimeIngress,
    FrameworkProviderCertification,
    ProductBootstrap,
    ProductSchemaModeling,
    ProductFrontend,
}

impl TaskRoute {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FrameworkDocsIa => "framework-docs-ia",
            Self::FrameworkGenerator => "framework-generator",
            Self::FrameworkRuntimeIngress => "framework-runtime-ingress",
            Self::FrameworkProviderCertification => "framework-provider-certification",
            Self::ProductBootstrap => "product-bootstrap",
            Self::ProductSchemaModeling => "product-schema-modeling",
            Self::ProductFrontend => "product-frontend",
        }
    }

    const fn manifest(self) -> RouteManifest {
        match self {
            Self::FrameworkDocsIa => RouteManifest {
                namespace: Namespace::Framework,
                minimum_class: ChangeClass::A,
                skill_path: "agent_skills/framework-docs-ia/SKILL.md",
                skill_heading: "Framework Docs IA",
                skill_reason: "Apply the repository procedure for agent-facing docs, skills, and command contracts.",
                references: &[
                    InstructionReference {
                        path: "docs/start/spec-driven-change-harness.md",
                        heading: "Spec-Driven Change Harness",
                        reason: "Preserve intent and proof for a framework CLI contract change.",
                    },
                    InstructionReference {
                        path: "docs/reference/cli.md",
                        heading: "App Framework CLI Reference",
                        reason: "Keep the public command and JSON behavior canonical.",
                    },
                    InstructionReference {
                        path: "docs/architecture/concerns/maintainability.md",
                        heading: "Maintainability Command Center",
                        reason: "Preserve command discoverability and avoid duplicate guidance.",
                    },
                ],
            },
            Self::FrameworkGenerator => RouteManifest {
                namespace: Namespace::Framework,
                minimum_class: ChangeClass::B,
                skill_path: "agent_skills/framework-generator/SKILL.md",
                skill_heading: "Framework Generator",
                skill_reason: "Apply the source-of-generation workflow and deterministic drift checks.",
                references: &[
                    InstructionReference {
                        path: "docs/start/generated-ownership.md",
                        heading: "Generated Ownership",
                        reason: "Distinguish generator source from generated product output.",
                    },
                    InstructionReference {
                        path: "docs/reference/codegen-api.md",
                        heading: "Codegen API",
                        reason: "Preserve the generator contract and supported extension points.",
                    },
                ],
            },
            Self::FrameworkRuntimeIngress => RouteManifest {
                namespace: Namespace::Framework,
                minimum_class: ChangeClass::C,
                skill_path: "agent_skills/framework-runtime-ingress/SKILL.md",
                skill_heading: "Framework Runtime Ingress",
                skill_reason: "Apply the shared runtime ingress and operation-dispatch procedure.",
                references: &[
                    InstructionReference {
                        path: "docs/architecture/overview.md",
                        heading: "Architecture",
                        reason: "Preserve runtime module boundaries and shared invocation flow.",
                    },
                    InstructionReference {
                        path: "docs/runtime/mcp.md",
                        heading: "Agentic MCP Server",
                        reason: "Preserve ingress isolation and authorization expectations.",
                    },
                ],
            },
            Self::FrameworkProviderCertification => RouteManifest {
                namespace: Namespace::Framework,
                minimum_class: ChangeClass::C,
                skill_path: "agent_skills/framework-provider-certification/SKILL.md",
                skill_heading: "Framework Provider Certification",
                skill_reason: "Apply provider parity, capability, and certification controls.",
                references: &[
                    InstructionReference {
                        path: "docs/runtime/provider-certification.md",
                        heading: "Relationship-Aware Provider Certification",
                        reason: "Use the canonical provider evidence and parity contract.",
                    },
                    InstructionReference {
                        path: "docs/runtime/provider-sdk.md",
                        heading: "Provider SDK Rules",
                        reason: "Keep capability declarations aligned with provider implementation.",
                    },
                ],
            },
            Self::ProductBootstrap => RouteManifest {
                namespace: Namespace::Product,
                minimum_class: ChangeClass::A,
                skill_path: "agent_skills/product-bootstrap/SKILL.md",
                skill_heading: "Product Bootstrap",
                skill_reason: "Apply the product repository bootstrap and clean-intent workflow.",
                references: &[
                    InstructionReference {
                        path: "docs/lifecycle/product-golden-path.md",
                        heading: "Product Developer Golden Path",
                        reason: "Follow the canonical downstream product lifecycle.",
                    },
                    InstructionReference {
                        path: "docs/start/cli-quickstart.md",
                        heading: "CLI Quickstart",
                        reason: "Use the supported product namespace and first command path.",
                    },
                ],
            },
            Self::ProductSchemaModeling => RouteManifest {
                namespace: Namespace::Product,
                minimum_class: ChangeClass::B,
                skill_path: "agent_skills/product-schema-modeling/SKILL.md",
                skill_heading: "Product Schema Modeling",
                skill_reason: "Apply the product-owned model source and validation workflow.",
                references: &[
                    InstructionReference {
                        path: "docs/model/schema-design.md",
                        heading: "Schema Design",
                        reason: "Use the canonical entity and relationship modeling contract.",
                    },
                    InstructionReference {
                        path: "docs/start/generated-ownership.md",
                        heading: "Generated Ownership",
                        reason: "Keep model source distinct from generated artifacts.",
                    },
                ],
            },
            Self::ProductFrontend => RouteManifest {
                namespace: Namespace::Product,
                minimum_class: ChangeClass::A,
                skill_path: "agent_skills/product-frontend/SKILL.md",
                skill_heading: "Product Frontend",
                skill_reason: "Apply the enterprise product frontend workflow and evidence gates.",
                references: &[
                    InstructionReference {
                        path: "docs/frontend/product-frontend.md",
                        heading: "Frontend Starter Contract",
                        reason: "Follow the canonical frontend ownership and verification contract.",
                    },
                    InstructionReference {
                        path: "docs/frontend/pds-health-design-system.md",
                        heading: "PDS Health Enterprise Design System",
                        reason: "Reuse the governed design-system primitives and accessibility expectations.",
                    },
                ],
            },
        }
    }
}

impl FromStr for TaskRoute {
    type Err = InstructionRouteError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "framework-docs-ia" => Ok(Self::FrameworkDocsIa),
            "framework-generator" => Ok(Self::FrameworkGenerator),
            "framework-runtime-ingress" => Ok(Self::FrameworkRuntimeIngress),
            "framework-provider-certification" => Ok(Self::FrameworkProviderCertification),
            "product-bootstrap" => Ok(Self::ProductBootstrap),
            "product-schema-modeling" => Ok(Self::ProductSchemaModeling),
            "product-frontend" => Ok(Self::ProductFrontend),
            _ => Err(InstructionRouteError::unknown(
                "task",
                value,
                &[
                    "framework-docs-ia",
                    "framework-generator",
                    "framework-provider-certification",
                    "framework-runtime-ingress",
                    "product-bootstrap",
                    "product-frontend",
                    "product-schema-modeling",
                ],
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProducerRole {
    CodingAgent,
    ArchitectAgent,
    XoAgent,
    WorkstreamAnalyst,
}

impl ProducerRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CodingAgent => "coding-agent",
            Self::ArchitectAgent => "architect-agent",
            Self::XoAgent => "xo-agent",
            Self::WorkstreamAnalyst => "workstream-analyst",
        }
    }
}

impl FromStr for ProducerRole {
    type Err = InstructionRouteError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "coding-agent" => Ok(Self::CodingAgent),
            "architect-agent" => Ok(Self::ArchitectAgent),
            "xo-agent" => Ok(Self::XoAgent),
            "workstream-analyst" => Ok(Self::WorkstreamAnalyst),
            _ => Err(InstructionRouteError::unknown(
                "role",
                value,
                &[
                    "architect-agent",
                    "coding-agent",
                    "workstream-analyst",
                    "xo-agent",
                ],
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionReference {
    pub path: &'static str,
    pub heading: &'static str,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RouteManifest {
    namespace: Namespace,
    minimum_class: ChangeClass,
    skill_path: &'static str,
    skill_heading: &'static str,
    skill_reason: &'static str,
    references: &'static [InstructionReference],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentRoute {
    pub role: &'static str,
    pub profile: &'static str,
    pub capability_tier: &'static str,
    pub reasoning_effort: &'static str,
    pub review_depth: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionRouteEnvelope {
    pub namespace: Namespace,
    pub task: TaskRoute,
    pub producer_role: ProducerRole,
    pub change_class: ChangeClass,
    pub role_card: InstructionReference,
    pub skill: InstructionReference,
    pub canonical_references: Vec<InstructionReference>,
    pub implementer_route: AgentRoute,
    pub independent_reviewer_route: AgentRoute,
}

impl InstructionRouteEnvelope {
    pub fn to_json(&self) -> Value {
        json!({
            "change_class": self.change_class.as_str(),
            "fallback_policy": {
                "mode": "fail_closed",
                "silent_substitution": "prohibited",
            },
            "implementer_route": agent_route_json(&self.implementer_route),
            "independent_reviewer_route": agent_route_json(&self.independent_reviewer_route),
            "instructions": {
                "canonical_references": self.canonical_references.iter().map(reference_json).collect::<Vec<_>>(),
                "producer_role_card": reference_json(&self.role_card),
                "skill": reference_json(&self.skill),
            },
            "namespace": self.namespace.as_str(),
            "ok": true,
            "producer_role": self.producer_role.as_str(),
            "route_policy": {
                "adapter_resolution": "named_project_adapter_required",
                "generic_inheritance": "prohibited",
                "mandatory_rules": "preserve",
                "self_certification": "prohibited",
            },
            "schema": INSTRUCTION_ROUTE_SCHEMA,
            "task": self.task.as_str(),
        })
    }

    pub fn to_json_line(&self) -> String {
        format!("{}\n", self.to_json())
    }
}

pub fn resolve(
    namespace: &str,
    task: &str,
    role: &str,
    change_class: &str,
) -> Result<InstructionRouteEnvelope, InstructionRouteError> {
    let namespace = Namespace::from_str(namespace)?;
    let task = TaskRoute::from_str(task)?;
    let producer_role = ProducerRole::from_str(role)?;
    let change_class = ChangeClass::from_str(change_class)?;
    let manifest = task.manifest();

    if namespace != manifest.namespace {
        return Err(InstructionRouteError::mismatch(
            "namespace_task_mismatch",
            format!(
                "task {} belongs to the {} namespace",
                task.as_str(),
                manifest.namespace.as_str()
            ),
        ));
    }
    if producer_role != ProducerRole::CodingAgent {
        return Err(InstructionRouteError::mismatch(
            "role_task_mismatch",
            format!("task {} requires producer role coding-agent", task.as_str()),
        ));
    }
    if change_class < manifest.minimum_class {
        return Err(InstructionRouteError::mismatch(
            "change_class_below_route_minimum",
            format!(
                "task {} requires change class {} or higher",
                task.as_str(),
                manifest.minimum_class.as_str()
            ),
        ));
    }

    let (implementer_route, independent_reviewer_route) = route_profiles(namespace, change_class);
    Ok(InstructionRouteEnvelope {
        namespace,
        task,
        producer_role,
        change_class,
        role_card: InstructionReference {
            path: "docs/start/agent-role-cards.md",
            heading: "Coding Agent",
            reason: "Apply the sole producer authority, collaboration, and stop boundaries for implementation work.",
        },
        skill: InstructionReference {
            path: manifest.skill_path,
            heading: manifest.skill_heading,
            reason: manifest.skill_reason,
        },
        canonical_references: manifest.references.to_vec(),
        implementer_route,
        independent_reviewer_route,
    })
}

pub fn resolve_arguments(
    namespace: &str,
    args: &[String],
) -> Result<InstructionRouteEnvelope, InstructionRouteError> {
    let mut task = None;
    let mut role = None;
    let mut change_class = None;
    let mut json = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--task" => {
                set_once(&mut task, "task", args.get(index + 1))?;
                index += 2;
            }
            "--role" => {
                set_once(&mut role, "role", args.get(index + 1))?;
                index += 2;
            }
            "--change-class" => {
                set_once(&mut change_class, "change_class", args.get(index + 1))?;
                index += 2;
            }
            "--json" => {
                if json {
                    return Err(InstructionRouteError::ambiguous("json"));
                }
                json = true;
                index += 1;
            }
            value => return Err(InstructionRouteError::unexpected(value)),
        }
    }

    if !json {
        return Err(InstructionRouteError::missing("json"));
    }
    resolve(
        namespace,
        task.ok_or_else(|| InstructionRouteError::missing("task"))?,
        role.ok_or_else(|| InstructionRouteError::missing("role"))?,
        change_class.ok_or_else(|| InstructionRouteError::missing("change_class"))?,
    )
}

fn set_once<'a>(
    target: &mut Option<&'a str>,
    field: &'static str,
    value: Option<&'a String>,
) -> Result<(), InstructionRouteError> {
    if target.is_some() {
        return Err(InstructionRouteError::ambiguous(field));
    }
    let value = value.ok_or_else(|| InstructionRouteError::missing(field))?;
    if value.starts_with("--") {
        return Err(InstructionRouteError::missing(field));
    }
    *target = Some(value);
    Ok(())
}

fn route_profiles(namespace: Namespace, change_class: ChangeClass) -> (AgentRoute, AgentRoute) {
    let (capability_tier, reasoning_effort) = match change_class {
        ChangeClass::A | ChangeClass::B => ("standard", "medium"),
        ChangeClass::C => ("strong", "high"),
        ChangeClass::D => ("maximum", "xhigh"),
    };
    let (review_capability_tier, review_reasoning_effort, review_depth) = match change_class {
        ChangeClass::A | ChangeClass::B => ("standard", "medium", "focused"),
        ChangeClass::C => ("maximum", "xhigh", "comprehensive"),
        ChangeClass::D => ("maximum", "max", "comprehensive_adversarial"),
    };
    let (review_role, review_profile) = match namespace {
        Namespace::Framework => ("framework-pr-review-agent", "framework-pr-review"),
        Namespace::Product => ("product-pr-review-agent", "product-pr-review"),
    };

    (
        AgentRoute {
            role: "coding-agent",
            profile: "implementation",
            capability_tier,
            reasoning_effort,
            review_depth: "not_applicable",
        },
        AgentRoute {
            role: review_role,
            profile: review_profile,
            capability_tier: review_capability_tier,
            reasoning_effort: review_reasoning_effort,
            review_depth,
        },
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionRouteError {
    pub code: &'static str,
    pub field: Option<&'static str>,
    pub value: Option<String>,
    pub allowed: Vec<&'static str>,
    pub detail: String,
}

impl InstructionRouteError {
    pub fn missing(field: &'static str) -> Self {
        Self {
            code: "missing_instruction_route_input",
            field: Some(field),
            value: None,
            allowed: Vec::new(),
            detail: format!("missing required --{} value", field.replace('_', "-")),
        }
    }

    pub fn ambiguous(field: &'static str) -> Self {
        Self {
            code: "ambiguous_instruction_route_input",
            field: Some(field),
            value: None,
            allowed: Vec::new(),
            detail: format!("--{} may be provided exactly once", field.replace('_', "-")),
        }
    }

    pub fn unexpected(_value: &str) -> Self {
        Self {
            code: "unknown_instruction_route_option",
            field: Some("option"),
            value: None,
            allowed: vec!["--task", "--role", "--change-class", "--json"],
            detail: "unknown instruction route option".to_owned(),
        }
    }

    fn unknown(field: &'static str, _value: &str, allowed: &[&'static str]) -> Self {
        Self {
            code: "unknown_instruction_route_input",
            field: Some(field),
            value: None,
            allowed: allowed.to_vec(),
            detail: format!("unsupported value for {field}"),
        }
    }

    fn mismatch(code: &'static str, detail: String) -> Self {
        Self {
            code,
            field: None,
            value: None,
            allowed: Vec::new(),
            detail,
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "error": {
                "allowed": self.allowed,
                "code": self.code,
                "detail": self.detail,
                "field": self.field,
                "value": self.value,
            },
            "ok": false,
            "schema": INSTRUCTION_ROUTE_SCHEMA,
        })
    }

    pub fn to_json_line(&self) -> String {
        format!("{}\n", self.to_json())
    }
}

impl fmt::Display for InstructionRouteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

fn reference_json(reference: &InstructionReference) -> Value {
    json!({
        "heading": reference.heading,
        "path": reference.path,
        "reason": reference.reason,
    })
}

fn agent_route_json(route: &AgentRoute) -> Value {
    json!({
        "capability_tier": route.capability_tier,
        "profile": route.profile,
        "reasoning_effort": route.reasoning_effort,
        "review_depth": route.review_depth,
        "role": route.role,
    })
}
