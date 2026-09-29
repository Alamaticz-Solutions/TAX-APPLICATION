use appfw_runtime::ingress::{IxRouteGroup, RuntimeRouteSet};

fn legacy(group: IxRouteGroup) {
    let _ = RuntimeRouteSet::new().with_chat(group);
}

fn main() {
    let _ = legacy as fn(IxRouteGroup);
}
