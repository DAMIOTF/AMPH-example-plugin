//! Minimal, dependency-free AMPH functionality plugin.
//!
//! The module is built for `wasm32-unknown-unknown` with `#![no_std]`, so it
//! has no WASI, no `std`, no allocator, no host imports and no start function.
//! It exports exactly the AMPH ABI v1 surface:
//!
//! ```text
//! memory
//! amph_abi_version() -> i32          // must return 1
//! amph_alloc(i32) -> i32
//! amph_analyze(i32, i32) -> i64
//! ```
//!
//! AMPH calls a functionality plugin only when the user explicitly confirms
//! the `a` action for the selected enabled plugin. The request is:
//!
//! ```json
//! {
//!   "api": 1,
//!   "op": "analyze",
//!   "snapshot": {
//!     "scanned_at": "2026-10-25 00:00",
//!     "packages": [
//!       {
//!         "name": "linux",
//!         "version": "6.12.1",
//!         "size": 123456789,
//!         "explicit": true,
//!         "foreign": false,
//!         "depends": ["glibc"],
//!         "provides": []
//!       }
//!     ]
//!   }
//! }
//! ```
//!
//! The response is one JSON object with a human-readable `summary` and a
//! bounded `findings` array. The TUI displays those fields; the host only
//! guarantees JSON validity, size limits and the sandbox around execution.

#![cfg_attr(not(test), no_std)]

use core::cell::UnsafeCell;

#[cfg(not(test))]
use core::panic::PanicInfo;

/// AMPH ABI version implemented by this module.
const ABI_VERSION: i32 = 1;

/// Largest request body this plugin accepts. AMPH itself allows 1 MiB.
const INPUT_CAP: usize = 1024 * 1024;

/// Static response buffer.
const OUTPUT_CAP: usize = 256 * 1024;

/// Maximum nesting accepted while skipping unknown JSON values.
const MAX_JSON_DEPTH: usize = 32;

/// JSON returned for every malformed request.
const ERROR_JSON: &[u8] = br#"{"summary":"Invalid AMPH analyze request","detail":"expected api=1, op=analyze and snapshot.packages"}"#;

/// A static buffer with interior mutability.
///
/// WebAssembly is single-threaded here and the host calls the ABI
/// sequentially, so sharing the buffer is sound for this ABI. `UnsafeCell`
/// avoids creating references to a `static mut`.
#[repr(align(8))]
struct Buffer<const N: usize> {
    bytes: UnsafeCell<[u8; N]>,
}

// SAFETY: the host invokes `amph_alloc`/`amph_analyze` synchronously on one
// thread, and only this module accesses the buffers.
unsafe impl<const N: usize> Sync for Buffer<N> {}

static INPUT: Buffer<INPUT_CAP> = Buffer {
    bytes: UnsafeCell::new([0; INPUT_CAP]),
};

static OUTPUT: Buffer<OUTPUT_CAP> = Buffer {
    bytes: UnsafeCell::new([0; OUTPUT_CAP]),
};

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    // The host bounds execution with fuel; a panic must never unwind across
    // the C ABI.
    loop {}
}

/// Return the supported ABI version. AMPH requires the value 1.
#[no_mangle]
pub extern "C" fn amph_abi_version() -> i32 {
    ABI_VERSION
}

/// Reserve space for the host's JSON request.
///
/// The host writes exactly `len` bytes at the returned pointer. This example
/// uses one fixed input buffer, so concurrent calls are not supported (AMPH
/// never makes them). A negative result tells the host that allocation failed.
#[no_mangle]
pub extern "C" fn amph_alloc(len: i32) -> i32 {
    if len < 0 || len as usize > INPUT_CAP {
        return -1;
    }
    let pointer = INPUT.bytes.get() as usize;
    if pointer > i32::MAX as usize {
        return -1;
    }
    pointer as i32
}

/// Analyze one AMPH snapshot request and return packed
/// `(pointer << 32) | length`.
///
/// The result bytes must be one JSON value. Returning a JSON error object is
/// the only error channel available in ABI v1.
#[no_mangle]
pub extern "C" fn amph_analyze(pointer: i32, length: i32) -> i64 {
    let output_length =
        match input_slice(pointer, length).and_then(|input| parse_request(input).ok()) {
            Some(analysis) => render_analysis(&analysis, output_slice()).unwrap_or_else(copy_error),
            None => copy_error(),
        };
    pack_output(output_length)
}

/// Borrow the host-written request from the static input buffer.
fn input_slice(pointer: i32, length: i32) -> Option<&'static [u8]> {
    if pointer < 0 || length < 0 {
        return None;
    }
    let base = INPUT.bytes.get() as usize;
    let offset = (pointer as usize).checked_sub(base)?;
    let length = length as usize;
    let end = offset.checked_add(length)?;
    if end > INPUT_CAP {
        return None;
    }
    // SAFETY: the range was checked against the static input buffer, and the
    // returned slice is only used for the duration of `amph_analyze`.
    Some(unsafe { core::slice::from_raw_parts((base as *const u8).add(offset), length) })
}

/// Mutable view of the static output buffer.
fn output_slice() -> &'static mut [u8] {
    // SAFETY: single-threaded, sequential ABI calls; the slice is not retained
    // after `amph_analyze` returns.
    unsafe { core::slice::from_raw_parts_mut(OUTPUT.bytes.get().cast::<u8>(), OUTPUT_CAP) }
}

/// Write the static error object into the output buffer.
fn copy_error() -> usize {
    let output = output_slice();
    if output.len() < ERROR_JSON.len() {
        return 0;
    }
    output[..ERROR_JSON.len()].copy_from_slice(ERROR_JSON);
    ERROR_JSON.len()
}

/// Pack the output pointer and length exactly as ABI v1 requires.
fn pack_output(length: usize) -> i64 {
    let pointer = OUTPUT.bytes.get() as usize as u64;
    ((pointer << 32) | length as u64) as i64
}

/// A bounded, copyable JSON string. The raw JSON interior is retained so
/// escaped content can be written back without re-encoding.
#[derive(Clone, Copy)]
struct Text<const N: usize> {
    bytes: [u8; N],
    length: usize,
    truncated: bool,
}

impl<const N: usize> Default for Text<N> {
    fn default() -> Self {
        Self {
            bytes: [0; N],
            length: 0,
            truncated: false,
        }
    }
}

impl<const N: usize> Text<N> {
    fn set(&mut self, raw: &[u8]) {
        if raw.len() <= N {
            self.bytes[..raw.len()].copy_from_slice(raw);
            self.length = raw.len();
            self.truncated = false;
        } else {
            self.bytes = [0; N];
            self.length = 0;
            self.truncated = true;
        }
    }

    fn is_empty(&self) -> bool {
        self.length == 0 && !self.truncated
    }

    fn as_bytes(&self) -> &[u8] {
        if self.truncated {
            b"<truncated>"
        } else {
            &self.bytes[..self.length]
        }
    }
}

/// Aggregate fields for one package in the snapshot.
#[derive(Clone, Copy, Default)]
struct PackageSummary {
    name: Text<128>,
    version: Text<64>,
    size: u64,
    explicit: bool,
    foreign: bool,
    depends: u64,
    provides: u64,
}

/// Aggregates calculated from the snapshot.
#[derive(Clone, Copy, Default)]
struct Analysis {
    count: u64,
    total_bytes: u128,
    explicit: u64,
    foreign: u64,
    dependency_links: u64,
    provide_links: u64,
    largest: Option<PackageSummary>,
}

impl Analysis {
    fn add(&mut self, package: PackageSummary) -> Result<(), ()> {
        self.count = self.count.checked_add(1).ok_or(())?;
        self.total_bytes = self
            .total_bytes
            .checked_add(u128::from(package.size))
            .ok_or(())?;
        if package.explicit {
            self.explicit = self.explicit.checked_add(1).ok_or(())?;
        }
        if package.foreign {
            self.foreign = self.foreign.checked_add(1).ok_or(())?;
        }
        self.dependency_links = self
            .dependency_links
            .checked_add(package.depends)
            .ok_or(())?;
        self.provide_links = self.provide_links.checked_add(package.provides).ok_or(())?;

        if self
            .largest
            .as_ref()
            .is_none_or(|largest| package.size > largest.size)
        {
            self.largest = Some(package);
        }
        Ok(())
    }
}

/// Parse the AMPH analyze envelope.
///
/// Only the stable host-owned fields are interpreted. Unknown object fields are
/// skipped so a later host can add metadata without breaking this plugin.
/// `scanned_at` is validated as a string but intentionally not echoed.
fn parse_request(input: &[u8]) -> Result<Analysis, ()> {
    let mut cursor = Cursor::new(input);
    let mut analysis = Analysis::default();
    let mut api_seen = false;
    let mut op_seen = false;
    let mut snapshot_seen = false;

    cursor.skip_whitespace();
    cursor.take(b'{')?;
    cursor.skip_whitespace();
    if cursor.peek() == Some(b'}') {
        return Err(());
    }

    loop {
        cursor.skip_whitespace();
        if cursor.peek() == Some(b'}') {
            cursor.position += 1;
            break;
        }

        let key = cursor.string_raw()?;
        cursor.skip_whitespace();
        cursor.take(b':')?;
        cursor.skip_whitespace();

        match key {
            b"api" => {
                if cursor.number_u64()? != 1 {
                    return Err(());
                }
                api_seen = true;
            }
            b"op" => {
                if cursor.string_raw()? != b"analyze" {
                    return Err(());
                }
                op_seen = true;
            }
            b"snapshot" => {
                cursor.snapshot(&mut analysis)?;
                snapshot_seen = true;
            }
            _ => cursor.skip_value(0)?,
        }

        cursor.skip_whitespace();
        match cursor.peek() {
            Some(b',') => cursor.position += 1,
            Some(b'}') => {
                cursor.position += 1;
                break;
            }
            _ => return Err(()),
        }
    }

    cursor.skip_whitespace();
    if cursor.position != input.len() || !api_seen || !op_seen || !snapshot_seen {
        return Err(());
    }
    Ok(analysis)
}

/// Small JSON cursor for the AMPH analyze envelope.
struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn skip_whitespace(&mut self) {
        while let Some(byte) = self.bytes.get(self.position) {
            if matches!(byte, b' ' | b'\n' | b'\r' | b'\t') {
                self.position += 1;
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn take(&mut self, expected: u8) -> Result<(), ()> {
        if self.peek() == Some(expected) {
            self.position += 1;
            Ok(())
        } else {
            Err(())
        }
    }

    /// Read and validate a JSON string, returning its raw interior.
    fn string_raw(&mut self) -> Result<&'a [u8], ()> {
        self.take(b'"')?;
        let start = self.position;
        while let Some(byte) = self.peek() {
            match byte {
                b'"' => {
                    let end = self.position;
                    self.position += 1;
                    return Ok(&self.bytes[start..end]);
                }
                b'\\' => {
                    self.position += 1;
                    let escaped = self.peek().ok_or(())?;
                    self.position += 1;
                    match escaped {
                        b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => {}
                        b'u' => {
                            for _ in 0..4 {
                                let digit = self.peek().ok_or(())?;
                                if !digit.is_ascii_hexdigit() {
                                    return Err(());
                                }
                                self.position += 1;
                            }
                        }
                        _ => return Err(()),
                    }
                }
                byte if byte < 0x20 => return Err(()),
                _ => self.position += 1,
            }
        }
        Err(())
    }

    fn number_u64(&mut self) -> Result<u64, ()> {
        let start = self.position;
        while self
            .bytes
            .get(self.position)
            .is_some_and(|byte| byte.is_ascii_digit())
        {
            self.position += 1;
        }
        if start == self.position {
            return Err(());
        }
        let digits = &self.bytes[start..self.position];
        if digits.len() > 1 && digits[0] == b'0' {
            return Err(());
        }

        let mut value = 0u64;
        for digit in digits {
            value = value
                .checked_mul(10)
                .and_then(|value| value.checked_add(u64::from(digit - b'0')))
                .ok_or(())?;
        }
        Ok(value)
    }

    fn boolean(&mut self) -> Result<bool, ()> {
        if self.consume(b"true") {
            Ok(true)
        } else if self.consume(b"false") {
            Ok(false)
        } else {
            Err(())
        }
    }

    fn consume(&mut self, literal: &[u8]) -> bool {
        if self.bytes.get(self.position..self.position + literal.len()) == Some(literal) {
            self.position += literal.len();
            true
        } else {
            false
        }
    }

    fn string_array_count(&mut self) -> Result<u64, ()> {
        self.take(b'[')?;
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.position += 1;
            return Ok(0);
        }

        let mut count = 0u64;
        loop {
            self.skip_whitespace();
            self.string_raw()?;
            count = count.checked_add(1).ok_or(())?;
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.position += 1,
                Some(b']') => {
                    self.position += 1;
                    return Ok(count);
                }
                _ => return Err(()),
            }
        }
    }

    fn snapshot(&mut self, analysis: &mut Analysis) -> Result<(), ()> {
        self.take(b'{')?;
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            return Err(());
        }

        let mut packages_seen = false;
        loop {
            self.skip_whitespace();
            if self.peek() == Some(b'}') {
                self.position += 1;
                break;
            }

            let key = self.string_raw()?;
            self.skip_whitespace();
            self.take(b':')?;
            self.skip_whitespace();

            match key {
                b"scanned_at" => {
                    self.string_raw()?;
                }
                b"packages" => {
                    self.packages(analysis)?;
                    packages_seen = true;
                }
                _ => self.skip_value(0)?,
            }

            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.position += 1,
                Some(b'}') => {
                    self.position += 1;
                    break;
                }
                _ => return Err(()),
            }
        }

        if packages_seen {
            Ok(())
        } else {
            Err(())
        }
    }

    fn packages(&mut self, analysis: &mut Analysis) -> Result<(), ()> {
        self.take(b'[')?;
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.position += 1;
            return Ok(());
        }

        loop {
            self.skip_whitespace();
            self.package(analysis)?;
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.position += 1,
                Some(b']') => {
                    self.position += 1;
                    return Ok(());
                }
                _ => return Err(()),
            }
        }
    }

    fn package(&mut self, analysis: &mut Analysis) -> Result<(), ()> {
        self.take(b'{')?;
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            return Err(());
        }

        let mut package = PackageSummary::default();
        let mut name_seen = false;
        let mut size_seen = false;

        loop {
            self.skip_whitespace();
            if self.peek() == Some(b'}') {
                self.position += 1;
                break;
            }

            let key = self.string_raw()?;
            self.skip_whitespace();
            self.take(b':')?;
            self.skip_whitespace();

            match key {
                b"name" => {
                    package.name.set(self.string_raw()?);
                    name_seen = true;
                }
                b"version" => {
                    package.version.set(self.string_raw()?);
                }
                b"size" => {
                    package.size = self.number_u64()?;
                    size_seen = true;
                }
                b"explicit" => {
                    package.explicit = self.boolean()?;
                }
                b"foreign" => {
                    package.foreign = self.boolean()?;
                }
                b"depends" => {
                    package.depends = self.string_array_count()?;
                }
                b"provides" => {
                    package.provides = self.string_array_count()?;
                }
                _ => self.skip_value(0)?,
            }

            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.position += 1,
                Some(b'}') => {
                    self.position += 1;
                    break;
                }
                _ => return Err(()),
            }
        }

        if !name_seen || !size_seen || package.name.is_empty() {
            return Err(());
        }
        analysis.add(package)
    }

    fn skip_value(&mut self, depth: usize) -> Result<(), ()> {
        if depth > MAX_JSON_DEPTH {
            return Err(());
        }
        self.skip_whitespace();
        match self.peek() {
            Some(b'"') => {
                self.string_raw()?;
                Ok(())
            }
            Some(b'{') => self.skip_object(depth),
            Some(b'[') => self.skip_array(depth),
            Some(b't') => {
                if self.consume(b"true") {
                    Ok(())
                } else {
                    Err(())
                }
            }
            Some(b'f') => {
                if self.consume(b"false") {
                    Ok(())
                } else {
                    Err(())
                }
            }
            Some(b'n') => {
                if self.consume(b"null") {
                    Ok(())
                } else {
                    Err(())
                }
            }
            Some(byte) if byte == b'-' || byte.is_ascii_digit() => self.skip_number(),
            _ => Err(()),
        }
    }

    fn skip_object(&mut self, depth: usize) -> Result<(), ()> {
        self.take(b'{')?;
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.position += 1;
            return Ok(());
        }
        loop {
            self.skip_whitespace();
            self.string_raw()?;
            self.skip_whitespace();
            self.take(b':')?;
            self.skip_value(depth + 1)?;
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.position += 1,
                Some(b'}') => {
                    self.position += 1;
                    return Ok(());
                }
                _ => return Err(()),
            }
        }
    }

    fn skip_array(&mut self, depth: usize) -> Result<(), ()> {
        self.take(b'[')?;
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.position += 1;
            return Ok(());
        }
        loop {
            self.skip_value(depth + 1)?;
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.position += 1,
                Some(b']') => {
                    self.position += 1;
                    return Ok(());
                }
                _ => return Err(()),
            }
        }
    }

    fn skip_number(&mut self) -> Result<(), ()> {
        let start = self.position;
        while let Some(byte) = self.peek() {
            if byte.is_ascii_digit() || matches!(byte, b'-' | b'+' | b'.' | b'e' | b'E') {
                self.position += 1;
            } else {
                break;
            }
        }
        if start == self.position {
            Err(())
        } else {
            Ok(())
        }
    }
}

/// Render the analysis as one JSON object with `summary` and `findings`.
fn render_analysis(analysis: &Analysis, output: &mut [u8]) -> Option<usize> {
    let mut writer = Writer::new(output);
    let rendered = (|| -> Result<(), ()> {
        writer.raw(b"{\"summary\":\"")?;
        writer.raw(b"Analyzed ")?;
        writer.u64(analysis.count)?;
        if analysis.count == 1 {
            writer.raw(b" package")?;
        } else {
            writer.raw(b" packages")?;
        }

        if analysis.count == 0 {
            writer.raw(b"; the snapshot is empty.")?;
        } else {
            writer.raw(b"; total ")?;
            write_size(&mut writer, analysis.total_bytes)?;
            writer.raw(b"; ")?;
            writer.u64(analysis.explicit)?;
            writer.raw(b" explicit, ")?;
            writer.u64(analysis.foreign)?;
            writer.raw(b" foreign")?;
            if let Some(largest) = &analysis.largest {
                writer.raw(b"; largest ")?;
                writer.raw(largest.name.as_bytes())?;
                if !largest.version.is_empty() {
                    writer.raw(b" ")?;
                    writer.raw(largest.version.as_bytes())?;
                }
                writer.raw(b" (")?;
                write_size(&mut writer, u128::from(largest.size))?;
                writer.raw(b")")?;
            }
            writer.raw(b".")?;
        }

        writer.raw(b"\",\"findings\":[{\"kind\":\"totals\",\"packages\":")?;
        writer.u64(analysis.count)?;
        writer.raw(b",\"bytes\":")?;
        writer.u128(analysis.total_bytes)?;
        writer.raw(b",\"explicit\":")?;
        writer.u64(analysis.explicit)?;
        writer.raw(b",\"foreign\":")?;
        writer.u64(analysis.foreign)?;
        writer.raw(b",\"dependency_links\":")?;
        writer.u64(analysis.dependency_links)?;
        writer.raw(b",\"provide_links\":")?;
        writer.u64(analysis.provide_links)?;
        writer.raw(b"}")?;

        if let Some(largest) = &analysis.largest {
            writer.raw(b",{\"kind\":\"largest\",\"name\":\"")?;
            writer.raw(largest.name.as_bytes())?;
            writer.raw(b"\",\"version\":\"")?;
            writer.raw(largest.version.as_bytes())?;
            writer.raw(b"\",\"bytes\":")?;
            writer.u64(largest.size)?;
            writer.raw(b"}")?;
        }
        writer.raw(b"]}")
    })();

    rendered.ok().map(|()| writer.length)
}

/// Write a compact human-readable size into the summary string.
fn write_size(writer: &mut Writer<'_>, bytes: u128) -> Result<(), ()> {
    const KIB: u128 = 1024;
    const MIB: u128 = 1024 * KIB;
    const GIB: u128 = 1024 * MIB;
    const TIB: u128 = 1024 * GIB;

    if bytes >= TIB {
        writer.u128(bytes / TIB)?;
        writer.raw(b" TiB")
    } else if bytes >= GIB {
        writer.u128(bytes / GIB)?;
        writer.raw(b" GiB")
    } else if bytes >= MIB {
        writer.u128(bytes / MIB)?;
        writer.raw(b" MiB")
    } else if bytes >= KIB {
        writer.u128(bytes / KIB)?;
        writer.raw(b" KiB")
    } else {
        writer.u128(bytes)?;
        writer.raw(b" B")
    }
}

/// Bounded ASCII JSON writer; no formatting machinery and no allocation.
struct Writer<'a> {
    output: &'a mut [u8],
    length: usize,
}

impl<'a> Writer<'a> {
    fn new(output: &'a mut [u8]) -> Self {
        Self { output, length: 0 }
    }

    fn raw(&mut self, bytes: &[u8]) -> Result<(), ()> {
        let end = self.length.checked_add(bytes.len()).ok_or(())?;
        if end > self.output.len() {
            return Err(());
        }
        self.output[self.length..end].copy_from_slice(bytes);
        self.length = end;
        Ok(())
    }

    fn u64(&mut self, mut value: u64) -> Result<(), ()> {
        let mut digits = [0u8; 20];
        let mut index = digits.len();
        loop {
            index -= 1;
            digits[index] = b'0' + (value % 10) as u8;
            value /= 10;
            if value == 0 {
                break;
            }
        }
        self.raw(&digits[index..])
    }

    fn u128(&mut self, mut value: u128) -> Result<(), ()> {
        let mut digits = [0u8; 39];
        let mut index = digits.len();
        loop {
            index -= 1;
            digits[index] = b'0' + (value % 10) as u8;
            value /= 10;
            if value == 0 {
                break;
            }
        }
        self.raw(&digits[index..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_to_string(analysis: &Analysis) -> String {
        let mut output = [0u8; 2048];
        let length = render_analysis(analysis, &mut output).expect("render fits");
        core::str::from_utf8(&output[..length])
            .expect("ASCII output")
            .to_string()
    }

    fn sample_request() -> &'static [u8] {
        br#"{
            "api": 1,
            "op": "analyze",
            "snapshot": {
                "scanned_at": "2026-10-25 00:00",
                "packages": [
                    {
                        "name": "linux",
                        "version": "6.12.1",
                        "size": 1073741824,
                        "explicit": true,
                        "foreign": false,
                        "depends": ["glibc", "bash"],
                        "provides": []
                    },
                    {
                        "name": "example-foreign",
                        "version": "2.0",
                        "size": 1048576,
                        "explicit": false,
                        "foreign": true,
                        "depends": [],
                        "provides": ["example"]
                    }
                ]
            }
        }"#
    }

    #[test]
    fn parses_the_amph_analyze_envelope() {
        let analysis = parse_request(sample_request()).unwrap();
        assert_eq!(analysis.count, 2);
        assert_eq!(analysis.total_bytes, 1074790400);
        assert_eq!(analysis.explicit, 1);
        assert_eq!(analysis.foreign, 1);
        assert_eq!(analysis.dependency_links, 2);
        assert_eq!(analysis.provide_links, 1);
        let largest = analysis.largest.unwrap();
        assert_eq!(largest.name.as_bytes(), b"linux");
        assert_eq!(largest.version.as_bytes(), b"6.12.1");
        assert_eq!(largest.size, 1073741824);
    }

    #[test]
    fn renders_a_useful_summary_and_findings() {
        let analysis = parse_request(sample_request()).unwrap();
        let rendered = render_to_string(&analysis);
        assert_eq!(
            rendered,
            r#"{"summary":"Analyzed 2 packages; total 1 GiB; 1 explicit, 1 foreign; largest linux 6.12.1 (1 GiB).","findings":[{"kind":"totals","packages":2,"bytes":1074790400,"explicit":1,"foreign":1,"dependency_links":2,"provide_links":1},{"kind":"largest","name":"linux","version":"6.12.1","bytes":1073741824}]}"#
        );
    }

    #[test]
    fn accepts_an_empty_snapshot() {
        let analysis = parse_request(
            br#"{"api":1,"op":"analyze","snapshot":{"scanned_at":"now","packages":[]}}"#,
        )
        .unwrap();
        assert_eq!(analysis.count, 0);
        assert!(analysis.largest.is_none());
        assert_eq!(
            render_to_string(&analysis),
            r#"{"summary":"Analyzed 0 packages; the snapshot is empty.","findings":[{"kind":"totals","packages":0,"bytes":0,"explicit":0,"foreign":0,"dependency_links":0,"provide_links":0}]}"#
        );
    }

    #[test]
    fn accepts_reordered_keys_and_skips_unknown_fields() {
        let analysis = parse_request(
            br#"{
                "extra": {"nested": [true, null, 3]},
                "snapshot": {
                    "packages": [{
                        "unknown": "value",
                        "size": 1024,
                        "name": "pkg",
                        "version": "1.0",
                        "explicit": true,
                        "foreign": false,
                        "depends": [],
                        "provides": []
                    }],
                    "scanned_at": "now"
                },
                "op": "analyze",
                "api": 1
            }"#,
        )
        .unwrap();
        assert_eq!(analysis.count, 1);
        assert_eq!(analysis.total_bytes, 1024);
        assert_eq!(analysis.largest.unwrap().name.as_bytes(), b"pkg");
    }

    #[test]
    fn rejects_requests_outside_the_documented_envelope() {
        let cases: &[&[u8]] = &[
            br#"{"api":2,"op":"analyze","snapshot":{"packages":[]}}"#,
            br#"{"api":1,"op":"query","snapshot":{"packages":[]}}"#,
            br#"{"api":1,"op":"analyze"}"#,
            br#"{"api":1,"op":"analyze","snapshot":{}}"#,
            br#"{"api":1,"op":"analyze","snapshot":{"packages":[{}]}}"#,
            br#"{"api":1,"op":"analyze","snapshot":{"packages":[{"name":"pkg","size":-1}]}}"#,
            br#"{"api":1,"op":"analyze","snapshot":{"packages":[{"name":"pkg","size":1.5}]}}"#,
            br#"{"api":1,"op":"analyze","snapshot":{"packages":[{"name":"pkg","size":1,"depends":[1]}]}}"#,
            br#"{"api":1,"op":"analyze","snapshot":{"packages":[{"name":"pkg","size":1}]}} trailing"#,
        ];
        for input in cases {
            assert!(parse_request(input).is_err(), "accepted {input:?}");
        }
    }

    #[test]
    fn handles_maximum_size_without_overflowing_the_total() {
        let analysis = parse_request(
            br#"{"api":1,"op":"analyze","snapshot":{"scanned_at":"now","packages":[{"name":"huge","version":"1","size":18446744073709551615,"explicit":false,"foreign":false,"depends":[],"provides":[]}]}}"#,
        )
        .unwrap();
        assert_eq!(analysis.total_bytes, u128::from(u64::MAX));
        assert!(render_to_string(&analysis).contains(r#""bytes":18446744073709551615"#));
    }

    #[test]
    fn error_json_is_valid() {
        let text = core::str::from_utf8(ERROR_JSON).unwrap();
        assert!(text.starts_with(r#"{"summary":"#));
        assert!(text.ends_with('}'));
    }
}
