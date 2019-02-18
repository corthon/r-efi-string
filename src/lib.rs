#![feature(try_from)]

extern crate core;

use core::convert::TryFrom;
use core::ops::Deref;

pub struct Str16 {
    inner: [u16],
}

pub struct String16 {
    inner: Vec<u16>,
}

impl Str16 {

    /// Create Str16 from pointer to u16
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the pointer points to a nul-terminated
    /// native-endian UTF-16 string. The string should either originate in
    /// UEFI, or be restricted to the subset of UTF-16 that the UEFI spec
    /// allows.
    pub unsafe fn from_ptr<'a>(ptr: *const u16) -> &'a Str16 {
        let mut len: usize = 0;

        loop {
            len += 1;
            if ptr.offset(len as isize).read() == 0 {
                break;
            }
        }

        Self::from_u16_slice(std::slice::from_raw_parts(ptr, len))
    }

    /// Create Str16 from slice of u16
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the slice does not contain any 0
    /// characters, except for the last one, which must be 0.
    unsafe fn from_u16_slice<'a>(slice: &[u16]) -> &Str16 {
        &*(slice as *const [u16] as *const Str16)
    }

    pub fn as_ptr(&self) -> *const u16 {
        self.inner.as_ptr()
    }

    /// Create slice of 16-bit unicode codepoints from Str16 slice
    ///
    /// The terminating nul character is not included. Otherwise, this is
    /// just a cast.
    fn as_utf16(&self) -> &[u16] {
        unsafe { std::slice::from_raw_parts(self.inner.as_ptr() as *const u16, self.inner.len() - 1) }
    }
}

impl From<&Str16> for String {

    /// Create an owned String from a Str16 slice
    ///
    /// The UEFI specification only supports a subset of UCS-2 (which again
    /// is a subset of UTF-16). However, we will accept any string given to
    /// us by UEFI as long as it is UTF-16. Note that the converse is not the
    /// case, we only allow turning string slices into Str16 slices if they
    /// only encode unicode codepoints supported by the UEFI specification.
    ///
    /// If the Str16 slice somehow contains invalid UTF-16, we will panic.
    fn from(input: &Str16) -> Self {
        String::from_utf16(input.as_utf16()).unwrap()
    }
}

// The error type returned when a conversion from a string slice to an owned UEFI
// string fails.
#[derive(Debug)]
pub enum TryFromString16Error {
    Surrogate,
    Private,
    OutOfRange,
    Nul,
}

impl TryFrom<&str> for String16 {
    type Error = TryFromString16Error;

    /// Try to create an owned String16 from a string slice
    ///
    /// UEFI only supports a strict subset of Unicode. In particular only
    /// codepoints from the basic multilingual plane, not including the
    /// private use area, and not including nul. If the passed in string
    /// only encodes codepoints that UEFI supports, the conversion
    /// succceds, otherwise it fails.
    fn try_from(input: &str) -> Result<Self, Self::Error> {
        let mut output = Vec::with_capacity(input.len() + 1);

        for c in input.chars() {
            // A Char16 is any Unicode codepoint in the basic multilingual
            // plane, except any surrogate codepoint or a codepoint reserved
            // for private use.
            match c as u32 {
                0x0000            => return Err(TryFromString16Error::Nul),
                0x0001 ... 0xd7ff => output.push(c as u16),
                0xd800 ... 0xdfff => return Err(TryFromString16Error::Surrogate),
                0xe000 ... 0xf8ff => return Err(TryFromString16Error::Private),
                0xf900 ... 0xffff => output.push(c as u16),
                _ => return Err(TryFromString16Error::OutOfRange),
            }
        }

        output.push(0);

        Ok(String16{ inner: output })
    }
}

impl Deref for String16 {
    type Target = Str16;

    fn deref(&self) -> &Str16 {
        unsafe { Str16::from_u16_slice(self.inner.as_slice()) }
    }
}

#[cfg(test)]
mod tests {
    use core::convert::TryInto;
    use core::ops::Deref;

    #[test]
    fn roundtrip() {
        let native = "Hello World!\n";
        let uefi: crate::String16 = native.try_into().unwrap();

        assert_eq!(native, Into::<String>::into(uefi.deref()));
    }
}
