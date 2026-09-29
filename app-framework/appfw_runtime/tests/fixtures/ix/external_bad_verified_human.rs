use appfw_runtime::ix::IxVerifiedHuman;

fn fabricate() -> IxVerifiedHuman {
    IxVerifiedHuman {}
}

fn main() {
    let _ = fabricate as fn() -> IxVerifiedHuman;
}
