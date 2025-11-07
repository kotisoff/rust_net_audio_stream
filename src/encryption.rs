use aes::Aes256;
use cbc::cipher::generic_array::GenericArray;
use cbc::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use cbc::{Decryptor, Encryptor};

type Aes256Enc = Encryptor<Aes256>;
type Aes256Dec = Decryptor<Aes256>;

pub struct AudioEncryptor {
    key: [u8; 32],
    iv: [u8; 16],
}

impl AudioEncryptor {
    pub fn new(key: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        if key.len() != 32 {
            return Err("Key must be 32 bytes".into());
        }

        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(key);

        // Используем фиксированный IV для простоты
        let iv = [0u8; 16];

        Ok(AudioEncryptor { key: key_array, iv })
    }

    pub fn encrypt(&self, data: &[i16]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Конвертируем i16 в u8 байты
        let byte_data =
            unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2) };

        // Добавляем padding чтобы длина была кратной 16 байтам
        let block_size = 16;
        let padded_len = ((byte_data.len() + block_size - 1) / block_size) * block_size;
        let mut buffer = vec![0u8; padded_len];
        buffer[..byte_data.len()].copy_from_slice(byte_data);

        let key = GenericArray::from_slice(&self.key);
        let iv = GenericArray::from_slice(&self.iv);
        let mut encryptor = Aes256Enc::new(key, iv);

        for chunk in buffer.chunks_mut(16) {
            let block = GenericArray::from_mut_slice(chunk);
            encryptor.encrypt_block_mut(block);
        }

        Ok(buffer)
    }

    pub fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<i16>, Box<dyn std::error::Error>> {
        let mut buffer = encrypted_data.to_vec();

        let key = GenericArray::from_slice(&self.key);
        let iv = GenericArray::from_slice(&self.iv);
        let mut decryptor = Aes256Dec::new(key, iv);

        for chunk in buffer.chunks_mut(16) {
            let block = GenericArray::from_mut_slice(chunk);
            decryptor.decrypt_block_mut(block);
        }

        // Конвертируем u8 байты обратно в i16
        let result =
            unsafe { std::slice::from_raw_parts(buffer.as_ptr() as *const i16, buffer.len() / 2) }
                .to_vec();

        Ok(result)
    }
}
