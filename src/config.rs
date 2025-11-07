use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ClientConfig {
    pub server_address: String,
    pub input_device: String,
    pub db_threshold: f32,
    // Параметры будут определяться автоматически
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
    pub output_device: String,
    // Параметры будут определяться автоматически
}

#[derive(Debug, Deserialize)]
pub struct EncryptionConfig {
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub client: ClientConfig,
    pub server: ServerConfig,
    pub encryption: EncryptionConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name("config"))
            .build()?;

        settings.try_deserialize()
    }
}
