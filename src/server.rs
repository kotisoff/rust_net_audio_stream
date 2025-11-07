use crate::config::AppConfig;
use crate::encryption::AudioEncryptor;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use tokio::net::UdpSocket;

pub async fn run_server(config: AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let socket = Arc::new(UdpSocket::bind(&config.server.bind_address).await?);
    println!("Сервер запущен на {}", config.server.bind_address);

    // Инициализация шифрования
    let key_bytes = hex::decode(&config.encryption.key)?;
    let encryptor = Arc::new(AudioEncryptor::new(&key_bytes)?);

    // Настройка аудио вывода
    let host = cpal::default_host();
    let output_device = if config.server.output_device == "default" {
        host.default_output_device()
    } else {
        host.output_devices()?.find(|d| {
            d.name()
                .map(|n| n == config.server.output_device)
                .unwrap_or(false)
        })
    }
    .expect("Не удалось найти устройство вывода");

    println!(
        "Используется устройство вывода: {}",
        output_device.name().unwrap_or("unknown".to_string())
    );

    // Получаем поддерживаемые конфигурации
    let supported_configs = output_device
        .supported_output_configs()
        .expect("Не удалось получить поддерживаемые конфигурации");

    // Выбираем первую поддерживаемую конфигурацию
    let config_output = if let Some(config_range) = supported_configs.into_iter().next() {
        let sample_rate = config_range.min_sample_rate();
        let channels = config_range.channels();
        let buffer_size = cpal::BufferSize::Default;

        println!(
            "Используется конфигурация: {}Hz, {} каналов",
            sample_rate.0, channels
        );

        cpal::StreamConfig {
            channels,
            sample_rate,
            buffer_size,
        }
    } else {
        return Err("Устройство вывода не поддерживает ни одной конфигурации".into());
    };

    // Сохраняем количество каналов для преобразования
    let output_channels = config_output.channels as usize;

    // Буфер для хранения полученных данных
    let audio_buffer = Arc::new(Mutex::new(Vec::new()));
    let audio_buffer_clone = audio_buffer.clone();
    let encryptor_clone = encryptor.clone();
    let socket_clone = socket.clone();

    // Запускаем асинхронную задачу для приема данных
    tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match socket_clone.recv_from(&mut buf).await {
                Ok((size, addr)) => {
                    if let Ok(decrypted_data) = encryptor_clone.decrypt(&buf[..size]) {
                        let mut buffer = audio_buffer_clone.lock().unwrap();
                        buffer.extend_from_slice(&decrypted_data);

                        // Ограничиваем размер буфера - сначала вычисляем длину
                        let current_len = buffer.len();
                        if current_len > 8192 {
                            let drain_count = current_len - 4096;
                            buffer.drain(0..drain_count);
                        }
                        println!("Получено {} samples от {}", decrypted_data.len(), addr);
                    }
                }
                Err(e) => {
                    eprintln!("Ошибка приема: {}", e);
                }
            }
        }
    });

    let audio_buffer_stream = audio_buffer.clone();

    let output_stream = output_device.build_output_stream(
        &config_output,
        move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
            let mut buffer = audio_buffer_stream.lock().unwrap();

            // Преобразуем из моно в многоканальное если нужно
            if output_channels > 1 {
                let mono_len = buffer.len();
                let required_mono_samples = data.len() / output_channels;
                let copy_len = std::cmp::min(mono_len, required_mono_samples);

                if copy_len > 0 {
                    // Копируем моно данные и дублируем по каналам
                    for i in 0..copy_len {
                        let sample = buffer[i];
                        for channel in 0..output_channels {
                            data[i * output_channels + channel] = sample;
                        }
                    }
                    buffer.drain(0..copy_len);

                    // Заполняем остаток нулями если нужно
                    if copy_len < required_mono_samples {
                        for i in copy_len * output_channels..data.len() {
                            data[i] = 0;
                        }
                    }
                } else {
                    data.fill(0);
                }
            } else {
                // Моно вывод - просто копируем данные
                let copy_len = std::cmp::min(buffer.len(), data.len());

                if copy_len > 0 {
                    data[..copy_len].copy_from_slice(&buffer[..copy_len]);
                    buffer.drain(0..copy_len);
                }

                if copy_len < data.len() {
                    data[copy_len..].fill(0);
                }
            }
        },
        move |err| eprintln!("Ошибка аудио вывода: {}", err),
        None,
    )?;

    output_stream.play()?;
    println!("Аудио поток запущен");

    println!("Сервер готов к приему аудио. Нажмите Enter для остановки...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    Ok(())
}
