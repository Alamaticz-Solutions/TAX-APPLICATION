use appfw_runtime::ix::{IxPrincipalBinding, IxRuntimeIntentPolicy};

fn inject_actor(mut policy: IxRuntimeIntentPolicy, actor: IxPrincipalBinding) {
    policy.runner = actor;
}

fn main() {
    let _ = inject_actor as fn(IxRuntimeIntentPolicy, IxPrincipalBinding);
}
