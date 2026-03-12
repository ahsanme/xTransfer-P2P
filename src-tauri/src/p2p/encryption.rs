use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};

/// Generate an ephemeral X25519 keypair for a single transfer session
pub fn generate_ephemeral_keypair() -> (EphemeralSecret, [u8; 32]) {
    let secret = EphemeralSecret::random_from_rng(rand::thread_rng());
    let pubkey_bytes = PublicKey::from(&secret).to_bytes();
    (secret, pubkey_bytes)
}

/// Derive an AES-256-GCM session key from an ephemeral X25519 shared secret
/// using HKDF-SHA256.
pub fn derive_session_key(
    our_secret: EphemeralSecret,
    their_pubkey_bytes: &[u8; 32],
) -> Result<[u8; 32]> {
    let their_pubkey = PublicKey::from(*their_pubkey_bytes);
    let shared = our_secret.diffie_hellman(&their_pubkey);
    let hk = Hkdf::<Sha256>::new(None, shared.as_bytes());
    let mut key = [0u8; 32];
    hk.expand(b"xtransfer-file-v1", &mut key)
        .map_err(|_| anyhow!("HKDF expand failed"))?;
    Ok(key)
}

/// Derive a session key on the receiver side using a static secret
pub fn derive_session_key_receiver(
    our_static_secret: &StaticSecret,
    their_ephemeral_pubkey_bytes: &[u8; 32],
) -> Result<[u8; 32]> {
    let their_pubkey = PublicKey::from(*their_ephemeral_pubkey_bytes);
    let shared = our_static_secret.diffie_hellman(&their_pubkey);
    let hk = Hkdf::<Sha256>::new(None, shared.as_bytes());
    let mut key = [0u8; 32];
    hk.expand(b"xtransfer-file-v1", &mut key)
        .map_err(|_| anyhow!("HKDF expand failed"))?;
    Ok(key)
}

/// Build nonce from chunk_index (8 bytes LE) + first 4 bytes of transfer_id
fn build_nonce(chunk_index: u64, transfer_id_prefix: &[u8; 4]) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[..8].copy_from_slice(&chunk_index.to_le_bytes());
    nonce[8..12].copy_from_slice(transfer_id_prefix);
    nonce
}

/// Encrypt a chunk with AES-256-GCM.
/// Returns ciphertext + 16-byte GCM authentication tag.
pub fn encrypt_chunk(
    key: &[u8; 32],
    chunk_index: u64,
    transfer_id_prefix: &[u8; 4],
    plaintext: &[u8],
) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| anyhow!("AES key error: {e}"))?;
    let nonce_bytes = build_nonce(chunk_index, transfer_id_prefix);
    let nonce = Nonce::from_slice(&nonce_bytes);
    cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| anyhow!("encryption failed: {e}"))
}

/// Decrypt a chunk with AES-256-GCM and verify its authentication tag.
pub fn decrypt_chunk(
    key: &[u8; 32],
    chunk_index: u64,
    transfer_id_prefix: &[u8; 4],
    ciphertext: &[u8],
) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| anyhow!("AES key error: {e}"))?;
    let nonce_bytes = build_nonce(chunk_index, transfer_id_prefix);
    let nonce = Nonce::from_slice(&nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("decryption/auth failed: {e}"))
}
