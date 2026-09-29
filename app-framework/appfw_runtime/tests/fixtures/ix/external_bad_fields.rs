use appfw_runtime::ingress::IxRouteGroup;

fn extract(group: IxRouteGroup) {
    let _ = group.router;
}

fn main() {
    let _ = extract as fn(IxRouteGroup);
}
