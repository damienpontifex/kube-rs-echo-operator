use clap::Parser;
use futures::StreamExt;
use kube::runtime::watcher::Config;
use resources::{Context, Echo};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, instrument};

use kube::runtime::controller::Action;
use kube::runtime::{
    Controller,
    finalizer::{Event as Finalizer, finalizer},
};
use kube::{Api, Client};

/// A simple Kubernetes operator that watches Echo custom resources and echoes their messages.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The namespace to watch for Echo resources
    #[clap(short, long, default_value = "default", env = "WATCH_NAMESPACE")]
    namespace: Option<String>,
    /// If should watch all namespaces
    #[clap(long, env = "ALL_NAMESPACES")]
    all_namespaces: bool,
}

const ECHO_FINALIZER: &str = "echo.pontifex.dev/finalizer";

#[instrument(skip(echo, ctx))]
async fn reconcile(echo: Arc<Echo>, ctx: Arc<Context>) -> Result<Action, kube::Error> {
    info!("Starting reconciliation");
    let echos_api = Api::<Echo>::namespaced(
        ctx.client.clone(),
        echo.metadata.namespace.as_deref().unwrap_or("default"),
    );
    finalizer(&echos_api, ECHO_FINALIZER, echo, |event| async {
        match event {
            Finalizer::Apply(echo) => echo.reconcile(ctx).await,
            Finalizer::Cleanup(echo) => echo.cleanup(ctx).await,
        }
    })
    .await
    .map_err(|_| kube::Error::LinesCodecMaxLineLengthExceeded)
}

#[instrument(skip(_echo, _ctx), fields(name = _echo.metadata.name.as_deref().unwrap_or("<unknown>"), namespace = _echo.metadata.namespace.as_deref().unwrap_or("<unknown>")))]
fn error_policy(_echo: Arc<Echo>, _error: &kube::Error, _ctx: Arc<Context>) -> Action {
    error!("Reconciliation error occurred: {}", _error);
    Action::requeue(Duration::from_secs(5))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let client = Client::try_default()
        .await
        .expect("Failed to create kube client");
    let echo_api: Api<Echo> = if cli.all_namespaces {
        Api::all(client.clone())
    } else {
        Api::namespaced(
            client.clone(),
            cli.namespace.as_deref().unwrap_or("default"),
        )
    };
    Controller::new(echo_api, Config::default().any_semantic())
        .shutdown_on_signal()
        .run(reconcile, error_policy, Arc::new(Context { client }))
        .filter_map(|x| async move { Result::ok(x) })
        .for_each(|_| futures::future::ready(()))
        .await;
}
