use std::ffi::OsStr;

pub(super) fn sanitize_lock_file_name(name: &OsStr) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        percent_encode_bytes(name.as_bytes())
    }

    #[cfg(windows)]
    {
        use std::fmt::Write as _;
        use std::os::windows::ffi::OsStrExt;

        let mut out = String::new();
        for unit in name.encode_wide() {
            if let Ok(ascii) = u8::try_from(unit) {
                if is_safe_ascii_lock_byte(ascii) {
                    out.push(char::from(ascii));
                    continue;
                }
            }
            let _ = write!(&mut out, "%u{unit:04X}");
        }
        out
    }

    #[cfg(all(not(unix), not(windows)))]
    {
        name.to_string_lossy().to_string()
    }
}

#[cfg(unix)]
fn percent_encode_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    let mut out = String::with_capacity(bytes.len() * 3);
    for byte in bytes {
        if matches!(byte, b'.' | b'-' | b'_')
            || byte.is_ascii_digit()
            || byte.is_ascii_uppercase()
            || byte.is_ascii_lowercase()
        {
            out.push(char::from(*byte));
        } else {
            out.push('%');
            out.push(char::from(HEX[(byte >> 4) as usize]));
            out.push(char::from(HEX[(byte & 0x0F) as usize]));
        }
    }
    out
}

#[cfg(windows)]
fn is_safe_ascii_lock_byte(byte: u8) -> bool {
    matches!(byte, b'.' | b'-' | b'_')
        || byte.is_ascii_digit()
        || byte.is_ascii_uppercase()
        || byte.is_ascii_lowercase()
}
