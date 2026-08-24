use std::{fs, io::Read};

use crate::clock;

/// Hex from the operating system's randomness, falling back to the clock and the
/// process id if that is unavailable. The fallback is weak on purpose: it keeps a
/// machine id unique enough to name a file, and anything that needs a secret
/// checks the length it asked for rather than assuming this succeeded.
pub fn hex(bytes: usize) -> String {
    let mut buffer = vec![0_u8; bytes];
    let filled = fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut buffer))
        .is_ok();

    if !filled {
        let seed = (clock::now() as u64) ^ (u64::from(std::process::id()) << 32);
        for (index, slot) in buffer.iter_mut().enumerate() {
            *slot = ((seed >> (index % 8 * 8)) as u8) ^ (index as u8);
        }
    }

    buffer
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}
