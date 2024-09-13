use color_eyre::eyre;
use incipit::Config;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    setup_tracing_and_eyre()?;

    let config = Config::new()?;
    tracing::info!("Loaded config {config:#?}");

    incipit::run(config).await
}

fn setup_tracing_and_eyre() -> eyre::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "incipit=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    color_eyre::install()?;

    Ok(())
}
