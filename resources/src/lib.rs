use std::sync::Arc;
use std::time::Duration;

use kube::api::{Patch, PatchParams};
use kube::runtime::controller::Action;
use kube::{Client, CustomResource};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

#[derive(CustomResource, Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[kube(
    group = "pontifex.dev",
    version = "v1",
    kind = "Echo",
    namespaced,
    status = "EchoStatus"
)]
pub struct EchoSpec {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema, Default)]
pub struct EchoStatus {
    pub echoed_message: Option<String>,
    pub echoed: bool,
}

#[derive(Clone)]
pub struct Context {
    pub client: Client,
}

impl Echo {
    #[instrument(skip(self, ctx))]
    pub async fn reconcile(&self, ctx: Arc<Context>) -> Result<Action, kube::Error> {
        info!(
            "Reconciling Echo: {}",
            self.metadata.name.as_deref().unwrap_or("<unknown>")
        );
        if let Some(status) = &self.status
            && status.echoed
            && let Some(previous_message) = status.echoed_message.as_deref()
            && previous_message == self.spec.message
        {
            println!(
                "Echo: {} has already been echoed.",
                self.metadata.name.as_deref().unwrap_or("<unknown>")
            );
            return Ok(Action::requeue(Duration::from_mins(5)));
        }
        println!("Echoing message: {}", self.spec.message);

        // Update resource status with EchoStatus
        let api: kube::Api<Echo> = if let Some(ns) = &self.metadata.namespace {
            kube::Api::namespaced(ctx.client.clone(), ns)
        } else {
            kube::Api::default_namespaced(ctx.client.clone())
        };

        let status = EchoStatus {
            echoed: true,
            echoed_message: Some(self.spec.message.clone()),
        };
        api.patch_status(
            self.metadata
                .name
                .as_deref()
                .expect("Echo resource must have a name"),
            &PatchParams::default(),
            &Patch::Merge(&serde_json::json!({ "status": status })),
        )
        .await?;

        Ok(Action::requeue(std::time::Duration::from_secs(300)))
    }

    #[instrument(skip(self, _ctx))]
    pub async fn cleanup(&self, _ctx: Arc<Context>) -> Result<Action, kube::Error> {
        info!(
            "Cleaning up Echo: {}",
            self.metadata.name.as_deref().unwrap_or("<unknown>")
        );

        Ok(Action::await_change())
    }
}
