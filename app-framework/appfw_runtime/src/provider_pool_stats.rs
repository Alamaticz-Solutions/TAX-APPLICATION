#[derive(Clone, Debug, serde::Serialize)]
pub struct ProviderPoolStats {
    pub provider: String,
    pub data_source: String,
    pub instrumented: bool,
    pub max_size: Option<u64>,
    pub size: Option<u64>,
    pub available: Option<u64>,
    pub in_use: Option<u64>,
    pub waiting: Option<u64>,
    pub pressure: Option<f64>,
}

impl ProviderPoolStats {
    pub fn instrumented(
        provider: impl Into<String>,
        data_source: impl Into<String>,
        max_size: u64,
        size: u64,
        available: u64,
        waiting: u64,
    ) -> Self {
        let in_use = size.saturating_sub(available);
        let pressure = if max_size == 0 {
            0.0
        } else {
            (in_use + waiting) as f64 / max_size as f64
        };
        Self {
            provider: provider.into(),
            data_source: data_source.into(),
            instrumented: true,
            max_size: Some(max_size),
            size: Some(size),
            available: Some(available),
            in_use: Some(in_use),
            waiting: Some(waiting),
            pressure: Some(pressure),
        }
    }

    pub fn opaque(provider: impl Into<String>, data_source: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            data_source: data_source.into(),
            instrumented: false,
            max_size: None,
            size: None,
            available: None,
            in_use: None,
            waiting: None,
            pressure: None,
        }
    }
}
