use appfw_runtime::ingress::{IxRouteGroup, RuntimeRouteSet};

fn assemble(group: IxRouteGroup) {
    let _ = RuntimeRouteSet::new().with_ix_chat(group);
}

fn main() {
    let _ = assemble as fn(IxRouteGroup);
}
