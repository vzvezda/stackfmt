//  ( /   @ @    ()  Formats data to a string into a buffer on the stack
//   \  __| |__  /   (c) 2019 - present, Vladimir Zvezda
//    -/   "   \-    based on Stefan SO answer: https://stackoverflow.com/a/50201632/601298
//
use core::fmt;
use core::str::from_utf8_unchecked;

/// Impl of [core::fmt::Write] stream that writes formatted string into provided u8 buffer.
///
/// ```
/// use core::fmt;
///
/// let mut buffer = [0u8; 16];
/// let mut w = stackfmt::WriteTo::new(&mut buffer);
/// match fmt::write(&mut w, format_args!("The answer is {}", 42)) {
///    Ok(_) => w.as_str(),
///    Err(_) => "",
/// };
/// assert_eq!(buffer, "The answer is 42".as_bytes());
/// ```
pub struct WriteTo<'a> {
    buffer: &'a mut [u8],
    used: usize,    // Possition inside buffer where the written string ends
    overflow: bool, // If formatted data was truncated
}

// Construction and string access
impl<'a> WriteTo<'a> {
    /// Creates new stream.
    pub fn new(buffer: &'a mut [u8]) -> Self {
        WriteTo {
            buffer,
            used: 0,
            overflow: false,
        }
    }

    /// Returns buffer view as &str
    pub fn as_str(self) -> &'a str {
        unsafe { from_utf8_unchecked(&self.buffer[..self.used]) }
    }
}

// true if byte pattern is 10xx'xxxx (e.g. if this is not a start of utf8 char)
fn is_not_first_utf8(ch: u8) -> bool {
    (ch & 0xC0) == 0x80
}

// max_len < raw_string.len()
fn find_closest_boundary(raw_string: &[u8], max_len: usize) -> usize {
    debug_assert!(
        max_len < raw_string.len(),
        "find_closest_boundary precondition failed"
    );

    if max_len == 0 {
        0 // no data can be added to buffer because it is empty
    } else if !is_not_first_utf8(raw_string[max_len]) {
        max_len // next byte is start of new char, so we can truncate on max_len
    } else {
        let mut res_len = max_len;
        loop {
            if !is_not_first_utf8(raw_string[res_len - 1]) {
                break res_len - 1;
            }
            res_len -= 1;
        }
    }
}

// Makes the WriteTo<'a> target for core::fmt::write() method.
impl<'a> fmt::Write for WriteTo<'a> {
    // Write that data fmt::write() feeds into a buffer and truncate if needed.
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.overflow {
            return Ok(()); // skip further inputs
        }

        let remaining_buf = &mut self.buffer[self.used..];
        let raw_s = s.as_bytes();

        if remaining_buf.len() >= raw_s.len() {
            // The whole input string fits into the buffer, just copy it
            remaining_buf[..raw_s.len()].copy_from_slice(raw_s);
            self.used += raw_s.len();
        } else {
            // The whole input string does not fit into the buffer.
            self.overflow = true;
            let boundary_size = find_closest_boundary(raw_s, remaining_buf.len());
            remaining_buf[..boundary_size].copy_from_slice(&raw_s[..boundary_size]);
            self.used += boundary_size;
        }
        Ok(())
    }
}

/// Writes formatted string into the buffer truncating if needed making the result valid utf8.
/// 
/// Example:
/// ```rust
/// let mut buf = [0u8; 6];
/// let formatted: &str = stackfmt::fmt_truncate(&mut buf, format_args!("Hello{}", 42));
/// assert_eq!(formatted, "Hello4");
/// ```
pub fn fmt_truncate<'a>(buffer: &'a mut [u8], args: fmt::Arguments) -> &'a str {
    let mut w = WriteTo::new(buffer);
    match fmt::write(&mut w, args) {
        Ok(_) => w.as_str(),
        Err(_) => "",
    }
}

#[cfg(test)]
pub mod tests {
    #[test]
    /// Test for is_not_first_utf8(), e.g. if given byte is a non-start byte of UTF8
    fn is_not_first_utf8_test() {
        // U+20AC  = E2 82 AC
        assert!(!super::is_not_first_utf8(0xE2));
        assert!(super::is_not_first_utf8(0x82));
        assert!(super::is_not_first_utf8(0xAC));
        // U+10348 = F0 90 8D 88
        assert!(!super::is_not_first_utf8(0xF0));
        assert!(super::is_not_first_utf8(0x90));
        assert!(super::is_not_first_utf8(0x8D));
        assert!(super::is_not_first_utf8(0x88));
        // space = 0x20
        assert!(!super::is_not_first_utf8(0x20));
    }

    #[test]
    fn find_closest_boundary_test_ascii() {
        // Truncate for ASCII string always truncates just when requested
        let buf = b"Hello";
        assert_eq!(super::find_closest_boundary(buf, 0), 0);
        assert_eq!(super::find_closest_boundary(buf, 1), 1);
        assert_eq!(super::find_closest_boundary(buf, 2), 2);
        assert_eq!(super::find_closest_boundary(buf, 3), 3);
        assert_eq!(super::find_closest_boundary(buf, 4), 4);
    }
    #[test]
    fn find_closest_boundary_test_unicode() {
        // U+10348 = F0 90 8D 88
        // U+20AC  = E2 82 AC
        let buf = [
            0xF0u8, 0x90, 0x8D, 0x88, /* next char */ 0xE2, 0x82, 0xAC,
            /* ascii */ 0x20,
        ];
        assert_eq!(super::find_closest_boundary(&buf, 0), 0);
        assert_eq!(super::find_closest_boundary(&buf, 1), 0);
        assert_eq!(super::find_closest_boundary(&buf, 2), 0);
        assert_eq!(super::find_closest_boundary(&buf, 3), 0);
        assert_eq!(super::find_closest_boundary(&buf, 4), 4);
        assert_eq!(super::find_closest_boundary(&buf, 5), 4);
        assert_eq!(super::find_closest_boundary(&buf, 6), 4);
        assert_eq!(super::find_closest_boundary(&buf, 7), 7);
    }

    #[test]
    fn format_ok() {
        let mut buf = [0u8; 64];
        let formatted: &str = super::fmt_truncate(&mut buf, format_args!("Hello{}", 42));
        assert_eq!(formatted, "Hello42");
    }

    #[test]
    fn format_truncate_ascii() {
        let mut buf = [0u8; 4];
        let formatted: &str = super::fmt_truncate(&mut buf, format_args!("Hello{}", 42));
        assert_eq!(formatted, "Hell");
    }

    #[test]
    fn format_truncate_ascii_second() {
        let mut buf = [0u8; 6];
        let formatted: &str = super::fmt_truncate(&mut buf, format_args!("Hello{}", 42));
        assert_eq!(formatted, "Hello4");
    }

    #[test]
    fn format_zero() {
        let mut buf = [0u8; 0];
        let formatted: &str = super::fmt_truncate(&mut buf, format_args!("Hello{}", 42));
        assert_eq!(formatted, "");
    }

    #[test]
    fn format_truncate_unicode() {
        let mut buf = [0u8; 4];
        // U+10348 = F0 90 8D 88
        // U+20AC  = E2 82 AC
        let formatted: &str = super::fmt_truncate(&mut buf, format_args!("Add{}", "\u{20AC}"));
        assert_eq!(formatted, "Add");
    }
}

