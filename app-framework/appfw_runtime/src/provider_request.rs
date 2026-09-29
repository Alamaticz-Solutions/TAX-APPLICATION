use crate::{extension::UserAuth, PolicyAccess};

#[derive(Clone, Debug)]
pub struct RuntimeProviderPlanInput<'a, P> {
    pub plan: P,
    pub user: &'a UserAuth,
    pub access: &'a PolicyAccess,
}

impl<'a, P> RuntimeProviderPlanInput<'a, P> {
    pub fn new(plan: P, user: &'a UserAuth, access: &'a PolicyAccess) -> Self {
        Self { plan, user, access }
    }

    pub fn plan(&self) -> &P {
        &self.plan
    }

    pub fn user(&self) -> &'a UserAuth {
        self.user
    }

    pub fn access(&self) -> &'a PolicyAccess {
        self.access
    }

    pub fn into_parts(self) -> (P, &'a UserAuth, &'a PolicyAccess) {
        (self.plan, self.user, self.access)
    }

    pub fn map_plan<Q>(self, map: impl FnOnce(P) -> Q) -> RuntimeProviderPlanInput<'a, Q> {
        RuntimeProviderPlanInput {
            plan: map(self.plan),
            user: self.user,
            access: self.access,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn user() -> UserAuth {
        UserAuth::human(
            "tenant-1",
            "casey",
            "UTC",
            vec!["admin".to_string()],
            vec!["appfw:data.read".to_string()],
            "token",
        )
    }

    #[test]
    fn provider_plan_input_preserves_runtime_context() {
        let user = user();
        let access = PolicyAccess::allow_with_filter(json!({ "tenant_id": { "_eq": "tenant-1" } }));
        let input = RuntimeProviderPlanInput::new("query-plan", &user, &access);

        assert_eq!(input.plan(), &"query-plan");
        assert_eq!(input.user().user_name, "casey");
        assert_eq!(
            input.access().filter,
            Some(json!({ "tenant_id": { "_eq": "tenant-1" } }))
        );

        let mapped = input.map_plan(|plan| format!("{plan}:runtime"));
        let (plan, mapped_user, mapped_access) = mapped.into_parts();
        assert_eq!(plan, "query-plan:runtime");
        assert_eq!(mapped_user.tenant_id, "tenant-1");
        assert!(mapped_access.allow);
    }
}
