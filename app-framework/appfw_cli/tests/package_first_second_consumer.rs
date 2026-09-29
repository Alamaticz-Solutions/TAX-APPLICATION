use appfw_cli::lifecycle::{inspect_package, run};

#[test]
fn second_consumer_fixture_accepts_a_hashable_package_shape() {
    let root = std::env::temp_dir().join(format!(
        "appfw-second-consumer-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    std::fs::create_dir_all(root.join("bin")).expect("create bin");
    std::fs::create_dir_all(root.join("scripts")).expect("create scripts");
    std::fs::create_dir_all(root.join("app_gen/_templates")).expect("create templates");
    std::fs::create_dir_all(root.join("docs/start")).expect("create delivery policy directory");
    std::fs::write(
        root.join("app-framework-package.json"),
        r#"{"version":"0.1.1"}"#,
    )
    .expect("write identity");
    std::fs::write(root.join("scripts/appfw"), "#!/usr/bin/env bash\n").expect("write wrapper");
    std::fs::write(
        root.join("docs/start/delivery-profiles.json"),
        "{\"schema_version\":1}\n",
    )
    .expect("write delivery policy");
    for helper in ["appfw", "app_gen", "appfw_introspect", "database"] {
        std::fs::write(root.join("bin").join(helper), "").expect("write helper");
    }

    let contract = inspect_package(&root, "second-consumer".to_string()).expect("package contract");

    assert_eq!(contract.version_identity, "0.1.1");
    assert_eq!(
        contract.helpers,
        ["appfw", "app_gen", "appfw_introspect", "database"]
    );
    assert!(contract.no_checkout && contract.deterministic_generation);
    assert!(contract.upgrade_guidance.contains("generate --check"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn public_recover_rejects_malformed_journal_without_mutating_consumer() {
    let root = std::env::temp_dir().join(format!(
        "appfw-public-recover-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    let model = root.join(".appfw/model/schemas/consumer/entity_types/work_item.yaml");
    std::fs::create_dir_all(model.parent().expect("model parent")).expect("create model parent");
    std::fs::write(&model, "original model\n").expect("write model");
    let journal = root.join(".appfw/package-first-upgrade.pending.json");
    std::fs::write(&journal, "{ malformed journal\n").expect("write journal");
    let before_model = std::fs::read(&model).expect("read model");
    let before_journal = std::fs::read(&journal).expect("read journal");

    let error = run(&[
        "recover".to_string(),
        "--consumer-root".to_string(),
        root.display().to_string(),
    ])
    .expect_err("public command rejects malformed journal");

    assert!(error.contains("pending lifecycle journal is malformed"));
    assert_eq!(std::fs::read(&model).expect("read model"), before_model);
    assert_eq!(
        std::fs::read(&journal).expect("read journal"),
        before_journal
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn public_recover_rejects_journal_tamper_matrix_without_mutation() {
    let root = std::env::temp_dir().join(format!(
        "appfw-public-recover-matrix-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    let model = root.join(".appfw/model/schemas/consumer/entity_types/work_item.yaml");
    std::fs::create_dir_all(model.parent().expect("model parent")).expect("create model parent");
    std::fs::write(&model, "original model\n").expect("write model");
    let journal = root.join(".appfw/package-first-upgrade.pending.json");
    let base = serde_json::json!({
        "schema": "appfw_package_first_pending@2",
        "source": "app_gen/_golden/downstream_apps/second-consumer/starter/.appfw/model/schemas/consumer/entity_types/work_item.yaml",
        "target": ".appfw/model/schemas/consumer/entity_types/work_item.yaml",
        "original_model_hex": "6f726967696e616c206d6f64656c0a",
        "generated_files": [], "generated_dirs": [], "completed_state": null,
    });
    let cases = [
        (
            "unsupported-schema",
            serde_json::json!({"schema":"appfw_package_first_pending@3","source":base["source"],"target":base["target"],"original_model_hex":base["original_model_hex"],"generated_files":[],"generated_dirs":[],"completed_state":null}),
        ),
        (
            "path-escape",
            serde_json::json!({"schema":base["schema"],"source":base["source"],"target":"../escape","original_model_hex":base["original_model_hex"],"generated_files":[],"generated_dirs":[],"completed_state":null}),
        ),
        (
            "bad-hex",
            serde_json::json!({"schema":base["schema"],"source":base["source"],"target":base["target"],"original_model_hex":"zz","generated_files":[],"generated_dirs":[],"completed_state":null}),
        ),
        (
            "duplicate-file",
            serde_json::json!({"schema":base["schema"],"source":base["source"],"target":base["target"],"original_model_hex":base["original_model_hex"],"generated_files":[{"path":".appfw/model/schemas/consumer/gql_enum_types/_res.yaml","bytes_hex":"61"},{"path":".appfw/model/schemas/consumer/gql_enum_types/_res.yaml","bytes_hex":"61"}],"generated_dirs":[".appfw/model/schemas/consumer/gql_enum_types"],"completed_state":null}),
        ),
    ];
    for (name, payload) in cases {
        let bytes = serde_json::to_vec(&payload).expect("serialize journal");
        std::fs::write(&journal, &bytes).expect("write journal");
        let before_model = std::fs::read(&model).expect("read model");
        let error = run(&[
            "recover".to_string(),
            "--consumer-root".to_string(),
            root.display().to_string(),
        ])
        .expect_err(name);
        assert!(
            error.contains("pending lifecycle journal")
                || error.contains("unsupported")
                || error.contains("outside")
                || error.contains("duplicated")
        );
        assert_eq!(
            std::fs::read(&model).expect("read model"),
            before_model,
            "{name}"
        );
        assert_eq!(
            std::fs::read(&journal).expect("read journal"),
            bytes,
            "{name}"
        );
    }
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn public_recover_rejects_symlinked_generated_leaf_without_mutation() {
    use std::os::unix::fs::symlink;
    let root = std::env::temp_dir().join(format!(
        "appfw-public-recover-symlink-{}",
        std::process::id()
    ));
    let model = root.join(".appfw/model/schemas/consumer/entity_types/work_item.yaml");
    let generated = root.join(".appfw/model/schemas/consumer/gql_enum_types/_res.yaml");
    std::fs::create_dir_all(model.parent().expect("model parent")).expect("create model parent");
    std::fs::create_dir_all(generated.parent().expect("generated parent"))
        .expect("create generated parent");
    std::fs::write(&model, "original model\n").expect("write model");
    let external = std::env::temp_dir().join(format!(
        "appfw-public-recover-external-{}",
        std::process::id()
    ));
    std::fs::write(&external, "external\n").expect("write external");
    symlink(&external, &generated).expect("create generated symlink");
    let journal = root.join(".appfw/package-first-upgrade.pending.json");
    let bytes = serde_json::to_vec(&serde_json::json!({
        "schema":"appfw_package_first_pending@2", "source":"app_gen/_golden/downstream_apps/second-consumer/starter/.appfw/model/schemas/consumer/entity_types/work_item.yaml", "target":".appfw/model/schemas/consumer/entity_types/work_item.yaml", "original_model_hex":"6f726967696e616c206d6f64656c0a", "generated_files":[{"path":".appfw/model/schemas/consumer/gql_enum_types/_res.yaml","bytes_hex":"61"}], "generated_dirs":[".appfw/model/schemas/consumer/gql_enum_types"], "completed_state":null
    })).expect("serialize journal");
    std::fs::write(&journal, &bytes).expect("write journal");
    let error = run(&[
        "recover".to_string(),
        "--consumer-root".to_string(),
        root.display().to_string(),
    ])
    .expect_err("reject symlink leaf");
    assert!(error.contains("invalid leaf type"));
    assert_eq!(
        std::fs::read(&model).expect("read model"),
        b"original model\n"
    );
    assert_eq!(std::fs::read(&journal).expect("read journal"), bytes);
    assert_eq!(
        std::fs::read(&external).expect("read external"),
        b"external\n"
    );
    let _ = std::fs::remove_file(&generated);
    let _ = std::fs::remove_file(&external);
    let _ = std::fs::remove_dir_all(root);
}
