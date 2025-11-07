use crate::config::AppConfig;
use crate::encryption::AudioEncryptor;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;
use tokio::net::UdpSocket;

pub async fn run_client(config: AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
    socket.connect(&config.client.server_address).await?;
    println!("Клиент подключен к {}", config.client.server_address);

    // Инициализация шифрования
    let key_bytes = hex::decode(&config.encryption.key)?;
    let encryptor = Arc::new(AudioEncryptor::new(&key_bytes)?);

    // Настройка аудио ввода
    let host = cpal::default_host();
    let input_device = if config.client.input_device == "default" {
        host.default_input_device()
    } else {
        host.input_devices()?.find(|d| {
            d.name()
                .map(|n| n == config.client.input_device)
                .unwrap_or(false)
        })
    }
    .expect("Не удалось найти устройство ввода");

    println!(
        "Используется устройство ввода: {}",
        input_device.name().unwrap_or("unknown".to_string())
    );

    // Получаем поддерживаемые конфигурации
    let supported_configs = input_device
        .supported_input_configs()
        .expect("Не удалось получить поддерживаемые конфигурации");

    // Выбираем первую поддерживаемую конфигурацию
    let config_input = if let Some(config_range) = supported_configs.into_iter().next() {
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
        return Err("Устройство ввода не поддерживает ни одной конфигурации".into());
    };

    let socket_clone = socket.clone();
    let encryptor_clone = encryptor.clone();

    // Сохраняем количество каналов для преобразования
    let input_channels = config_input.channels as usize;

    // Порог громкости в децибелах (например -55 dB)
    let db_threshold = config.client.db_threshold;

    // Создаем канал для передачи аудиоданных в асинхронную задачу
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(100);

    // Запускаем асинхронную задачу для отправки данных
    tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if let Err(e) = socket_clone.send(&data).await {
                eprintln!("Ошибка отправки: {}", e);
            }
        }
    });

    let input_stream = input_device.build_input_stream(
        &config_input,
        move |data: &[i16], _: &cpal::InputCallbackInfo| {
            // Вычисляем громкость в dB
            let db_level = calculate_volume_db(data);

            // Преобразуем в моно если нужно
            let mono_data = if input_channels > 1 {
                convert_to_mono(data, input_channels)
            } else {
                data.to_vec()
            };

            // Отправляем только если громкость выше порога
            if db_level >= db_threshold {
                if let Ok(encrypted_data) = encryptor_clone.encrypt(&mono_data) {
                    // Используем блокирующую отправку в канал
                    let tx = tx.clone();
                    if let Err(e) = tx.try_send(encrypted_data) {
                        eprintln!("Ошибка отправки в канал: {}", e);
                    }
                }
                // Для отладки можно выводить только когда есть звук:
                // println!("Отправка: {:.1} dB", db_level);
            }
        },
        move |err| eprintln!("Ошибка аудио ввода: {}", err),
        None,
    )?;

    input_stream.play()?;
    println!("Аудио поток запущен");
    println!("Порог громкости: {} dB", db_threshold);
    println!("Клиент начал передачу аудио. Нажмите Enter для остановки...");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    Ok(())
}

// Функция для вычисления громкости в децибелах (dB FS)
fn calculate_volume_db(data: &[i16]) -> f32 {
    if data.is_empty() {
        return f32::NEG_INFINITY;
    }

    let sum_squares: f64 = data
        .iter()
        .map(|&sample| {
            let normalized = sample as f64 / i16::MAX as f64;
            normalized * normalized
        })
        .sum();

    let rms = (sum_squares / data.len() as f64).sqrt();

    // Преобразуем RMS в dB FS (Full Scale)
    // 0 dB FS = максимальная громкость, отрицательные значения для всего остального
    if rms <= 0.0 {
        f32::NEG_INFINITY
    } else {
        (20.0 * rms.log10()) as f32
    }
}

// Функция для преобразования многоканального аудио в моно
fn convert_to_mono(data: &[i16], channels: usize) -> Vec<i16> {
    if channels == 1 {
        return data.to_vec();
    }

    let mut mono_data = Vec::with_capacity(data.len() / channels);

    for chunk in data.chunks(channels) {
        let sum: i32 = chunk.iter().map(|&sample| sample as i32).sum();
        let avg = (sum / channels as i32) as i16;
        mono_data.push(avg);
    }

    mono_data
}
