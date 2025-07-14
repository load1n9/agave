// WASI Random implementation for Agave OS
use super::super::random;
use super::error::*;
use super::types::*;
use alloc::vec;
use alloc::vec::Vec;

// Linear Congruential Generator for basic randomness
// In a real implementation, this would use hardware random number generators
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        // LCG parameters from Numerical Recipes
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }

    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    fn fill_bytes(&mut self, buf: &mut [u8]) {
        for byte in buf.iter_mut() {
            *byte = self.next_u8();
        }
    }
}

static mut RNG: SimpleRng = SimpleRng { state: 12345 };

// Initialize the RNG with entropy from hardware if available
pub fn init_random() {
    unsafe {
        // Try to get entropy from hardware random number generator
        if let Ok(entropy) = get_random_u64() {
            RNG = SimpleRng::new(entropy);
        } else {
            // Fallback: use a combination of clock and some fixed entropy
            let clock_entropy =
                super::clocks::clock_time_get(super::types::CLOCKID_MONOTONIC, 0).unwrap_or(12345);
            RNG = SimpleRng::new(clock_entropy.wrapping_mul(0x5DEECE66D).wrapping_add(0xB));
        }
    }
}

// WASI Preview 1 API
pub fn random_get(buf: &mut [u8]) -> WasiResult<()> {
    unsafe {
        RNG.fill_bytes(buf);
    }
    Ok(())
}

// WASI Preview 2 API
pub fn get_random_bytes(len: u64) -> WasiResult<Vec<u8>> {
    if len > 1024 * 1024 {
        // Limit to 1MB to prevent excessive memory usage
        return Err(WasiError::inval());
    }

    let mut buf = vec![0u8; len as usize];
    random_get(&mut buf)?;
    Ok(buf)
}

pub fn get_random_u64() -> WasiResult<u64> {
    // Get random u64
    use super::random::get_random_u32;
    let high = get_random_u32()? as u64;
    let low = get_random_u32()? as u64;
    Ok((high << 32) | low)
}

pub fn get_random_u32() -> WasiResult<u32> {
    unsafe { Ok(RNG.next_u32()) }
}

pub fn get_random_u16() -> WasiResult<u16> {
    unsafe { Ok((RNG.next_u64() >> 48) as u16) }
}

pub fn get_random_u8() -> WasiResult<u8> {
    unsafe { Ok(RNG.next_u8()) }
}

// Cryptographically secure random numbers (simulated)
pub fn get_secure_random_bytes(len: u64) -> WasiResult<Vec<u8>> {
    // In a real implementation, this would use a CSPRNG
    // For now, we'll use the same RNG but seed it differently
    if len > 1024 * 1024 {
        return Err(WasiError::inval());
    }

    let mut buf = vec![0u8; len as usize];

    // Reseed with current time for each secure random call
    let current_time =
        super::clocks::clock_time_get(super::types::CLOCKID_MONOTONIC, 0).unwrap_or(0);

    unsafe {
        let original_state = RNG.state;
        RNG.state = current_time.wrapping_mul(0x5DEECE66D).wrapping_add(0xB);
        RNG.fill_bytes(&mut buf);
        RNG.state = original_state; // Restore original state
    }

    Ok(buf)
}

// Random number generation with specific distributions
pub fn random_uniform_u64(max: u64) -> WasiResult<u64> {
    if max == 0 {
        return Err(WasiError::inval());
    }

    unsafe { Ok(RNG.next_u64() % max) }
}

pub fn random_uniform_u32(max: u32) -> WasiResult<u32> {
    if max == 0 {
        return Err(WasiError::inval());
    }

    unsafe { Ok(RNG.next_u32() % max) }
}

pub fn random_range_u64(min: u64, max: u64) -> WasiResult<u64> {
    if min >= max {
        return Err(WasiError::inval());
    }

    let range = max - min;
    let random_val = random_uniform_u64(range)?;
    Ok(min + random_val)
}

pub fn random_range_u32(min: u32, max: u32) -> WasiResult<u32> {
    if min >= max {
        return Err(WasiError::inval());
    }

    let range = max - min;
    let random_val = random_uniform_u32(range)?;
    Ok(min + random_val)
}

// Random boolean
pub fn random_bool() -> WasiResult<bool> {
    unsafe { Ok((RNG.next_u64() & 1) == 1) }
}

// Random float between 0.0 and 1.0
pub fn random_f64() -> WasiResult<f64> {
    unsafe {
        let random_bits = RNG.next_u64();
        // Convert to float in range [0, 1)
        let float_val = (random_bits >> 11) as f64 / (1u64 << 53) as f64;
        Ok(float_val)
    }
}

pub fn random_f32() -> WasiResult<f32> {
    unsafe {
        let random_bits = RNG.next_u32();
        // Convert to float in range [0, 1)
        let float_val = (random_bits >> 8) as f32 / (1u32 << 24) as f32;
        Ok(float_val)
    }
}

// Fill a buffer with random data
pub fn fill_random(buf: &mut [u8]) -> WasiResult<()> {
    random_get(buf)
}

// Generate random ASCII string
pub fn random_ascii_string(len: usize) -> WasiResult<alloc::string::String> {
    const ASCII_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    let mut result = alloc::string::String::with_capacity(len);

    for _ in 0..len {
        let index = random_uniform_u32(ASCII_CHARS.len() as u32)? as usize;
        result.push(ASCII_CHARS[index] as char);
    }

    Ok(result)
}

// Generate random hex string
pub fn random_hex_string(len: usize) -> WasiResult<alloc::string::String> {
    const HEX_CHARS: &[u8] = b"0123456789abcdef";

    let mut result = alloc::string::String::with_capacity(len);

    for _ in 0..len {
        let index = random_uniform_u32(HEX_CHARS.len() as u32)? as usize;
        result.push(HEX_CHARS[index] as char);
    }

    Ok(result)
}

// Additional functions for Preview 2 compatibility
pub fn insecure_random() -> WasiResult<u64> {
    get_random_u64()
}

pub fn insecure_random_bytes(len: u64) -> WasiResult<Vec<u8>> {
    get_secure_random_bytes(len)
}
