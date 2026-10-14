//! Honest per-operation capability tiers for this module, using the same
//! 4-status vocabulary as the framework provider docs. Nothing here may claim
//! `live_certified`: no retained live run exists, because no real tenant
//! site/drive/role grant exists yet.
//!
//!   live_certified      - a retained live contract run exists. NONE here.
//!   compiler_contracted - request-plan construction (and, for the write, the
//!                         gate) proven by unit contracts; no retained live run.
//!   planned_gated       - registered, non-executable, named evidence gates.
//!   write_gated         - registered write candidate, non-executable.

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    LiveCertified,
    CompilerContracted,
    PlannedGated,
    WriteGated,
}

#[allow(dead_code)]
impl Tier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LiveCertified => "live_certified",
            Self::CompilerContracted => "compiler_contracted",
            Self::PlannedGated => "planned_gated",
            Self::WriteGated => "write_gated",
        }
    }
}

#[allow(dead_code)]
pub struct OperationContract {
    pub name: &'static str,
    pub kind: &'static str,        // "read" | "write"
    pub sensitivity: &'static str, // "pii" | "none"
    pub tier: Tier,
}

#[allow(dead_code)]
pub const CONTRACTS: &[OperationContract] = &[
    OperationContract {
        name: "list_drive_items",
        kind: "read",
        sensitivity: "pii",
        tier: Tier::CompilerContracted,
    },
    OperationContract {
        name: "get_drive_item",
        kind: "read",
        sensitivity: "pii",
        tier: Tier::CompilerContracted,
    },
    OperationContract {
        name: "list_drive_item_versions",
        kind: "read",
        sensitivity: "none",
        tier: Tier::CompilerContracted,
    },
    OperationContract {
        name: "sharepoint_upload_file",
        kind: "write",
        sensitivity: "pii",
        // The 8-step gate in `writes::execute` runs and is unit-tested with a
        // fake transport, ledger and audit sink. That is what
        // `CompilerContracted` means - NOT "live_certified".
        tier: Tier::CompilerContracted,
    },
];

/// Writes allow-listed to claim `CompilerContracted` instead of `WriteGated`.
#[allow(dead_code)]
const GATED_WRITES: &[&str] = &["sharepoint_upload_file"];

#[allow(dead_code)]
pub fn assert_honest() -> Result<(), String> {
    for c in CONTRACTS {
        if c.tier == Tier::LiveCertified {
            return Err(format!("{} claims live_certified with no retained run", c.name));
        }
        if c.kind == "write" {
            let is_gated = GATED_WRITES.contains(&c.name);
            match c.tier {
                Tier::WriteGated => {}
                Tier::CompilerContracted if is_gated => {}
                _ => {
                    return Err(format!(
                        "{} is a write but neither write_gated nor an allow-listed gated write",
                        c.name
                    ))
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contracts_are_honest() {
        assert_honest().expect("vendor contract must stay honest");
    }

    #[test]
    fn nothing_is_live_certified() {
        assert!(CONTRACTS.iter().all(|c| c.tier != Tier::LiveCertified));
    }

    #[test]
    fn contract_names_match_the_registry() {
        let reads: Vec<_> = CONTRACTS
            .iter()
            .filter(|c| c.kind == "read")
            .map(|c| c.name)
            .collect();
        assert_eq!(
            reads,
            vec!["list_drive_items", "get_drive_item", "list_drive_item_versions"]
        );
    }
}
