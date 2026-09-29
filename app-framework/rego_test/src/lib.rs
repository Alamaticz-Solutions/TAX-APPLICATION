use std::path::{Path, PathBuf};

pub use appfw_test::policy::{
    evaluate_access, evaluate_access_rule, load_policy, AccessAction, AccessInput, AccessResult,
    AccessUser, PDS_TENANT_ID,
};

pub fn fixture_policy_path(policy_path: impl AsRef<Path>) -> PathBuf {
    appfw_test::policy::policy_fixture_path(env!("CARGO_MANIFEST_DIR"), policy_path)
}
