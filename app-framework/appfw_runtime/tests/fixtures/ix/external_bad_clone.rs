use appfw_runtime::ingress::IxRouteGroup;

fn duplicate(group: IxRouteGroup) {
    let _ = group.clone();
}

fn main() {
    let _ = duplicate as fn(IxRouteGroup);
}
