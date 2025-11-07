mod client;
mod config;
mod devices;
mod encryption;
mod server;

use clap::{Arg, Command};
use config::AppConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("Audio Transfer")
        .version("1.0")
        .about("Клиент-серверное приложение для передачи аудио")
        .arg(
            Arg::new("mode")
                .short('m')
                .long("mode")
                .value_parser(["client", "server", "devices"])
                .required(true)
                .help("Режим работы: client, server или devices"),
        )
        .get_matches();

    match matches.get_one::<String>("mode").map(|s| s.as_str()) {
        Some("server") => {
            println!("Starting server...");
            let config = AppConfig::load()?;
            server::run_server(config).await?;
        }
        Some("client") => {
            println!("Starting client...");
            let config = AppConfig::load()?;
            client::run_client(config).await?;
        }
        Some("devices") => {
            println!("Audio devices list:");
            devices::list_audio_devices()?;
        }
        _ => unreachable!(),
    }

    Ok(())
}
