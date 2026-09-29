use std::{collections::HashMap, future::Future, pin::Pin};

use crate::{provider_keys::FrameworkProvider, RuntimeError};

pub type RuntimeProviderFactoryFuture<P, E> = Pin<Box<dyn Future<Output = Result<P, E>> + Send>>;

type RuntimeProviderFactory<P, E> =
    dyn Fn(String) -> RuntimeProviderFactoryFuture<P, E> + Send + Sync;

pub struct RuntimeProviderRegistry<P, E> {
    factories: HashMap<FrameworkProvider, Box<RuntimeProviderFactory<P, E>>>,
}

impl<P, E> RuntimeProviderRegistry<P, E> {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    pub fn register<F, Fut>(mut self, provider: FrameworkProvider, factory: F) -> Self
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<P, E>> + Send + 'static,
    {
        self.factories.insert(
            provider,
            Box::new(move |data_source_name| Box::pin(factory(data_source_name))),
        );
        self
    }

    pub fn has_provider(&self, provider: FrameworkProvider) -> bool {
        self.factories.contains_key(&provider)
    }

    pub fn registered_providers(&self) -> impl Iterator<Item = FrameworkProvider> + '_ {
        self.factories.keys().copied()
    }

    pub async fn create(
        &self,
        provider: FrameworkProvider,
        data_source_name: impl Into<String>,
    ) -> Result<P, E>
    where
        E: From<RuntimeError>,
    {
        let factory = self.factories.get(&provider).ok_or_else(|| {
            RuntimeError::DataAccess(format!(
                "provider factory is not registered for {}",
                provider.key()
            ))
        })?;
        factory(data_source_name.into()).await
    }
}

impl<P, E> Default for RuntimeProviderRegistry<P, E> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn provider_registry_creates_registered_provider_clients() {
        let registry = RuntimeProviderRegistry::<String, RuntimeError>::new()
            .register(FrameworkProvider::Mssql, |data_source_name| async move {
                Ok(format!("mssql:{data_source_name}"))
            });

        assert!(registry.has_provider(FrameworkProvider::Mssql));
        let providers: Vec<_> = registry.registered_providers().collect();
        assert_eq!(providers, vec![FrameworkProvider::Mssql]);

        let client = registry
            .create(FrameworkProvider::Mssql, "crm_primary")
            .await
            .expect("registered provider client");
        assert_eq!(client, "mssql:crm_primary");
    }

    #[tokio::test]
    async fn provider_registry_rejects_unregistered_provider_clients() {
        let registry = RuntimeProviderRegistry::<String, RuntimeError>::new();

        let error = registry
            .create(FrameworkProvider::Snowflake, "analytics")
            .await
            .expect_err("unregistered provider");

        assert!(matches!(
            error,
            RuntimeError::DataAccess(message)
                if message == "provider factory is not registered for snowflake"
        ));
    }
}
