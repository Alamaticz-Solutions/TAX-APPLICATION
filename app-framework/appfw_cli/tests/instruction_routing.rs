use appfw_cli::instruction_routing::{
    resolve, resolve_arguments, InstructionReference, INSTRUCTION_ROUTE_SCHEMA,
};
use std::{fs, path::Path};

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn class_c_framework_route_is_source_linked_and_comprehensive() {
    let route = resolve("framework", "framework-docs-ia", "coding-agent", "C").expect("route");
    let value = route.to_json();

    assert_eq!(value["schema"], INSTRUCTION_ROUTE_SCHEMA);
    assert_eq!(value["ok"], true);
    assert_eq!(
        value["instructions"]["producer_role_card"]["path"],
        "docs/start/agent-role-cards.md"
    );
    assert_eq!(
        value["instructions"]["producer_role_card"]["heading"],
        "Coding Agent"
    );
    assert_eq!(
        value["instructions"]["skill"]["path"],
        "agent_skills/framework-docs-ia/SKILL.md"
    );
    assert_eq!(
        value["instructions"]["canonical_references"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(value["implementer_route"]["role"], "coding-agent");
    assert_eq!(
        value["independent_reviewer_route"]["role"],
        "framework-pr-review-agent"
    );
    assert_eq!(
        value["independent_reviewer_route"]["review_depth"],
        "comprehensive"
    );
    assert_eq!(value["route_policy"]["generic_inheritance"], "prohibited");
}

#[test]
fn all_manifest_routes_resolve_only_in_their_explicit_namespace() {
    let cases = [
        ("framework", "framework-docs-ia", "A"),
        ("framework", "framework-generator", "B"),
        ("framework", "framework-runtime-ingress", "C"),
        ("framework", "framework-provider-certification", "C"),
        ("product", "product-bootstrap", "A"),
        ("product", "product-schema-modeling", "B"),
        ("product", "product-frontend", "A"),
    ];

    for (namespace, task, change_class) in cases {
        let route = resolve(namespace, task, "coding-agent", change_class).expect(task);
        assert_eq!(route.namespace.as_str(), namespace);
        assert_eq!(route.task.as_str(), task);
        assert_ne!(
            route.implementer_route.role,
            route.independent_reviewer_route.role
        );
        assert!(!route.canonical_references.is_empty());
    }
}

#[test]
fn emitted_reference_paths_and_headings_match_canonical_markdown() {
    let cases = [
        ("framework", "framework-docs-ia", "A"),
        ("framework", "framework-generator", "B"),
        ("framework", "framework-runtime-ingress", "C"),
        ("framework", "framework-provider-certification", "C"),
        ("product", "product-bootstrap", "A"),
        ("product", "product-schema-modeling", "B"),
        ("product", "product-frontend", "A"),
    ];
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();

    for (namespace, task, change_class) in cases {
        let route = resolve(namespace, task, "coding-agent", change_class).expect(task);
        let mut references = vec![route.role_card, route.skill];
        references.extend(route.canonical_references);

        for reference in references {
            assert_markdown_heading(repository_root, reference, task);
        }
    }
}

fn assert_markdown_heading(repository_root: &Path, reference: InstructionReference, task: &str) {
    let source = fs::read_to_string(repository_root.join(reference.path))
        .unwrap_or_else(|error| panic!("{task}: could not read {}: {error}", reference.path));
    let found = source.lines().any(|line| {
        let trimmed = line.trim_start();
        let Some(heading) = trimmed.strip_prefix('#') else {
            return false;
        };
        heading.trim_start_matches('#').trim_start() == reference.heading
    });

    assert!(
        found,
        "{task}: heading {:?} does not exist in {}",
        reference.heading, reference.path
    );
}

#[test]
fn output_is_byte_stable_and_contains_no_runtime_context() {
    let first = resolve("product", "product-frontend", "coding-agent", "C")
        .expect("first")
        .to_json_line();
    let second = resolve("product", "product-frontend", "coding-agent", "C")
        .expect("second")
        .to_json_line();

    assert_eq!(first, second);
    for forbidden in [
        "timestamp",
        "generated_at",
        "cwd",
        "environment",
        "/private/",
    ] {
        assert!(
            !first.contains(forbidden),
            "unexpected runtime field: {forbidden}"
        );
    }
}

#[test]
fn unknown_mismatched_and_below_minimum_routes_fail_closed() {
    let unknown = resolve("framework", "/private/secret", "coding-agent", "C").unwrap_err();
    assert_eq!(unknown.code, "unknown_instruction_route_input");
    assert!(!unknown.to_json_line().contains("/private/secret"));

    let namespace = resolve("product", "framework-docs-ia", "coding-agent", "C").unwrap_err();
    assert_eq!(namespace.code, "namespace_task_mismatch");

    let role = resolve("framework", "framework-docs-ia", "architect-agent", "C").unwrap_err();
    assert_eq!(role.code, "role_task_mismatch");

    let class = resolve(
        "framework",
        "framework-runtime-ingress",
        "coding-agent",
        "B",
    )
    .unwrap_err();
    assert_eq!(class.code, "change_class_below_route_minimum");
}

#[test]
fn duplicate_and_missing_cli_inputs_are_ambiguous_or_incomplete() {
    let duplicate = resolve_arguments(
        "framework",
        &strings(&[
            "--task",
            "framework-docs-ia",
            "--task",
            "framework-generator",
            "--role",
            "coding-agent",
            "--change-class",
            "C",
            "--json",
        ]),
    )
    .unwrap_err();
    assert_eq!(duplicate.code, "ambiguous_instruction_route_input");

    let unsupported = resolve_arguments(
        "framework",
        &strings(&[
            "--task",
            "framework-docs-ia",
            "--role",
            "coding-agent",
            "--change-class",
            "C",
            "--json",
            "--check",
        ]),
    )
    .unwrap_err();
    assert_eq!(unsupported.code, "unknown_instruction_route_option");

    let missing = resolve_arguments(
        "framework",
        &strings(&[
            "--task",
            "framework-docs-ia",
            "--role",
            "coding-agent",
            "--json",
        ]),
    )
    .unwrap_err();
    assert_eq!(missing.code, "missing_instruction_route_input");
    assert_eq!(missing.field, Some("change_class"));

    let positional_missing = resolve_arguments(
        "framework",
        &strings(&[
            "--task",
            "--json",
            "framework-docs-ia",
            "--role",
            "coding-agent",
            "--change-class",
            "C",
        ]),
    )
    .unwrap_err();
    assert_eq!(positional_missing.code, "missing_instruction_route_input");
    assert_eq!(positional_missing.field, Some("task"));
}
