#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    appfw_codegen::run_from_args(std::env::args().skip(1))
}
