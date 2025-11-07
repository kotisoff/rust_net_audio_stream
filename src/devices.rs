use cpal::traits::{DeviceTrait, HostTrait};

pub fn list_audio_devices() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();

    println!("=== Input devices ===");
    let input_devices = host.input_devices()?;
    for device in input_devices {
        match device.name() {
            Ok(name) => println!(" {}", name),
            Err(e) => println!(" Unknown device (Error: {})", e),
        }

        // if let Ok(configs) = device.supported_input_configs() {
        //     for config in configs {
        //         println!(
        //             "  Поддерживает: {} каналов, {:?}",
        //             config.channels(),
        //             config.max_sample_rate()
        //         );
        //     }
        // }
    }

    println!("\n=== Output devices ===");
    let output_devices = host.output_devices()?;
    for device in output_devices {
        match device.name() {
            Ok(name) => println!(" {}", name),
            Err(..) => println!(" Unknown device"),
        }

        // if let Ok(configs) = device.supported_output_configs() {
        //     for config in configs {
        //         println!(
        //             "  Поддерживает: {} каналов, {:?}",
        //             config.channels(),
        //             config.max_sample_rate()
        //         );
        //     }
        // }
    }

    Ok(())
}
