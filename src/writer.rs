//! A module to manage protobuf serialization

use std::io::Write;

use errors::Result;
use message::MessageWrite;

use byteorder::WriteBytesExt;
use byteorder::LittleEndian as LE;

/// A struct to write protobuf messages
///
/// # Examples
///
/// ```rust
/// // an automatically generated module which is in a separate file in general
/// mod foo_bar {
///     # use quick_protobuf::{MessageWrite, Writer, Result};
///     # use std::borrow::Cow;
///     # use std::io::Write;
///     pub struct Foo<'a> { pub name: Option<Cow<'a, str>>, }
///     pub struct Bar { pub id: Option<u32> }
///     pub struct FooBar<'a> { pub foos: Vec<Foo<'a>>, pub bars: Vec<Bar>, }
///     impl<'a> MessageWrite for FooBar<'a> {
///         // implements
///         // fn get_size(&self) -> usize { ... }
///         // fn write_message<W: Write>(&self, r: &mut Writer<W>) -> Result<()> { ... }
///         # fn get_size(&self) -> usize { 0 }
///         # fn write_message<W: Write>(&self, _: &mut Writer<W>) -> Result<()> { Ok(()) }
///     }
/// }
///
/// // FooBar is a message generated from a proto file
/// // in parcicular it contains a `write_message` function
/// use foo_bar::{FooBar, Foo, Bar};
/// use std::borrow::Cow;
/// use quick_protobuf::Writer;
///
/// fn main() {
///     // let mut r = File::create("...").expect("Cannot create file");
///     // for the sake of example, we'll use a simpler struct which impl `Write`
///     let mut r = Vec::new();
///     let mut writer = Writer::new(&mut r);
///
///     // manually generates a FooBar for the example
///     let foobar = FooBar {
///         foos: vec![Foo { name: Some(Cow::Borrowed("test!")) }, Foo { name: None }],
///         bars: vec![Bar { id: Some(43) }, Bar { id: None }],
///     };
///
///     // now using the generated module
///     writer.write_message(&foobar).expect("Cannot write FooBar");
/// }
/// ```
pub struct Writer<W> {
    inner: W,
}

impl<W: Write> Writer<W> {

    /// Creates a new `ProtobufWriter`
    pub fn new(w: W) -> Writer<W> {
        Writer { inner: w }
    }

    /// Writes a `varint` (compacted `u64`)
    pub fn write_varint(&mut self, mut v: u64) -> Result<()> {
        while v > 0x7F {
            self.inner.write_u8(((v as u8) & 0x7F) | 0x80)?;
            v >>= 7;
        }
        self.inner.write_u8(v as u8).map_err(|e| e.into())
    }

    /// Writes a tag, which represents both the field number and the wire type
    pub fn write_tag(&mut self, tag: u32) -> Result<()> {
        self.write_varint(tag as u64)
    }

    /// Writes a `int32` which is internally coded as a `varint`
    pub fn write_int32(&mut self, v: i32) -> Result<()> {
        self.write_varint(v as u64)
    }

    /// Writes a `int64` which is internally coded as a `varint`
    pub fn write_int64(&mut self, v: i64) -> Result<()> {
        self.write_varint(v as u64)
    }

    /// Writes a `uint32` which is internally coded as a `varint`
    pub fn write_uint32(&mut self, v: u32) -> Result<()> {
        self.write_varint(v as u64)
    }

    /// Writes a `uint64` which is internally coded as a `varint`
    pub fn write_uint64(&mut self, v: u64) -> Result<()> {
        self.write_varint(v)
    }

    /// Writes a `sint32` which is internally coded as a `varint`
    pub fn write_sint32(&mut self, v: i32) -> Result<()> {
        self.write_varint(((v << 1) ^ (v >> 31)) as u64)
    }

    /// Writes a `sint64` which is internally coded as a `varint`
    pub fn write_sint64(&mut self, v: i64) -> Result<()> {
        self.write_varint(((v << 1) ^ (v >> 63)) as u64)
    }

    /// Writes a `fixed64` which is little endian coded `u64`
    pub fn write_fixed64(&mut self, v: u64) -> Result<()> {
        self.inner.write_u64::<LE>(v).map_err(|e| e.into())
    }

    /// Writes a `fixed32` which is little endian coded `u32`
    pub fn write_fixed32(&mut self, v: u32) -> Result<()> {
        self.inner.write_u32::<LE>(v).map_err(|e| e.into())
    }

    /// Writes a `sfixed64` which is little endian coded `i64`
    pub fn write_sfixed64(&mut self, v: i64) -> Result<()> {
        self.inner.write_i64::<LE>(v).map_err(|e| e.into())
    }

    /// Writes a `sfixed32` which is little endian coded `i32`
    pub fn write_sfixed32(&mut self, v: i32) -> Result<()> {
        self.inner.write_i32::<LE>(v).map_err(|e| e.into())
    }

    /// Writes a `float`
    pub fn write_float(&mut self, v: f32) -> Result<()> {
        self.inner.write_f32::<LE>(v).map_err(|e| e.into())
    }

    /// Writes a `double`
    pub fn write_double(&mut self, v: f64) -> Result<()> {
        self.inner.write_f64::<LE>(v).map_err(|e| e.into())
    }

    /// Writes a `bool` 1 = true, 0 = false
    pub fn write_bool(&mut self, v: bool) -> Result<()> {
        self.write_varint(if v { 1 } else { 0 })
    }

    /// Writes an `enum` converting it to a `i32` first
    pub fn write_enum(&mut self, v: i32) -> Result<()> {
        self.write_int32(v)
    }

    /// Writes `bytes`: length first then the chunk of data
    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.write_varint(bytes.len() as u64)?;
        self.inner.write_all(bytes).map_err(|e| e.into())
    }

    /// Writes `string`: length first then the chunk of data
    pub fn write_string(&mut self, s: &str) -> Result<()> {
        self.write_bytes(s.as_bytes())
    }

    /// Writes packed repeated field: length first then the chunk of data
    pub fn write_packed_repeated_field<M, F, S>(&mut self, v: &[M], mut write: F, size: &S) -> Result<()>
        where F: FnMut(&mut Self, &M) -> Result<()>,
              S: Fn(&M) -> usize,
    {
        if v.is_empty() {
            return Ok(());
        }
        let len: usize = v.iter().map(|m| size(m)).sum();
        self.write_varint(len as u64)?;
        for m in v {
            write(self, m)?;
        }
        Ok(())
    }

    /// Writes packed repeated field when we know the size of items
    ///
    /// `item_size` is internally used to compute the total length
    /// As the length is fixed (and the same as rust internal representation, we can directly dump
    /// all data at once
    pub fn write_packed_fixed_size<M>(&mut self, v: &[M], item_size: usize) -> Result<()> {
        let len = v.len() * item_size;
        let bytes = unsafe { ::std::slice::from_raw_parts(v as *const [M] as *const M as *const u8, len) };
        self.write_bytes(bytes)
    }

    /// Writes a message which implements `MessageWrite`
    pub fn write_message<M: MessageWrite>(&mut self, m: &M) -> Result<()> {
        let len = m.get_size();
        self.write_varint(len as u64)?;
        m.write_message(self)
    }

    /// Writes tag then `int32`
    pub fn write_int32_with_tag(&mut self, tag: u32, v: i32) -> Result<()> {
        self.write_tag(tag)?;
        self.write_varint(v as u64)
    }

    /// Writes tag then `int64`
    pub fn write_int64_with_tag(&mut self, tag: u32, v: i64) -> Result<()> {
        self.write_tag(tag)?;
        self.write_varint(v as u64)
    }

    /// Writes tag then `uint32`
    pub fn write_uint32_with_tag(&mut self, tag: u32, v: u32) -> Result<()> {
        self.write_tag(tag)?;
        self.write_varint(v as u64)
    }

    /// Writes tag then `uint64`
    pub fn write_uint64_with_tag(&mut self, tag: u32, v: u64) -> Result<()> {
        self.write_tag(tag)?;
        self.write_varint(v)
    }

    /// Writes tag then `sint32`
    pub fn write_sint32_with_tag(&mut self, tag: u32, v: i32) -> Result<()> {
        self.write_tag(tag)?;
        self.write_sint32(v)
    }

    /// Writes tag then `sint64`
    pub fn write_sint64_with_tag(&mut self, tag: u32, v: i64) -> Result<()> {
        self.write_tag(tag)?;
        self.write_sint64(v)
    }

    /// Writes tag then `fixed64`
    pub fn write_fixed64_with_tag(&mut self, tag: u32, v: u64) -> Result<()> {
        self.write_tag(tag)?;
        self.inner.write_u64::<LE>(v).map_err(|e| e.into())
    }

    /// Writes tag then `fixed32`
    pub fn write_fixed32_with_tag(&mut self, tag: u32, v: u32) -> Result<()> {
        self.write_tag(tag)?;
        self.inner.write_u32::<LE>(v).map_err(|e| e.into())
    }

    /// Writes tag then `sfixed64`
    pub fn write_sfixed64_with_tag(&mut self, tag: u32, v: i64) -> Result<()> {
        self.write_tag(tag)?;
        self.inner.write_i64::<LE>(v).map_err(|e| e.into())
    }

    /// Writes tag then `sfixed32`
    pub fn write_sfixed32_with_tag(&mut self, tag: u32, v: i32) -> Result<()> {
        self.write_tag(tag)?;
        self.inner.write_i32::<LE>(v).map_err(|e| e.into())
    }

    /// Writes tag then `float`
    pub fn write_float_with_tag(&mut self, tag: u32, v: f32) -> Result<()> {
        self.write_tag(tag)?;
        self.inner.write_f32::<LE>(v).map_err(|e| e.into())
    }

    /// Writes tag then `double`
    pub fn write_double_with_tag(&mut self, tag: u32, v: f64) -> Result<()> {
        self.write_tag(tag)?;
        self.inner.write_f64::<LE>(v).map_err(|e| e.into())
    }

    /// Writes tag then `bool`
    pub fn write_bool_with_tag(&mut self, tag: u32, v: bool) -> Result<()> {
        self.write_tag(tag)?;
        self.write_varint(if v { 1 } else { 0 })
    }

    /// Writes tag then `bytes`
    pub fn write_bytes_with_tag(&mut self, tag: u32, bytes: &[u8]) -> Result<()> {
        self.write_tag(tag)?;
        self.write_varint(bytes.len() as u64)?;
        self.inner.write_all(bytes).map_err(|e| e.into())
    }

    /// Writes tag then `string`
    pub fn write_string_with_tag(&mut self, tag: u32, s: &str) -> Result<()> {
        self.write_tag(tag)?;
        self.write_bytes(s.as_bytes())
    }

    /// Writes tag then repeated field
    ///
    /// If array is empty, then do nothing (do not even write the tag)
    pub fn write_packed_repeated_field_with_tag<M, F, S>(&mut self, 
                                                         tag: u32, 
                                                         v: &[M], 
                                                         mut write: F, 
                                                         size: &S) -> Result<()>
        where F: FnMut(&mut Self, &M) -> Result<()>,
              S: Fn(&M) -> usize,
    {
        if v.is_empty() {
            return Ok(());
        }

        self.write_tag(tag)?;
        let len: usize = v.iter().map(|m| size(m)).sum();
        self.write_varint(len as u64)?;
        for m in v {
            write(self, m)?;
        }
        Ok(())
    }

    /// Writes tag then repeated field with fixed length item size
    ///
    /// If array is empty, then do nothing (do not even write the tag)
    pub fn write_packed_fixed_size_with_tag<M>(&mut self, 
                                               tag: u32, 
                                               v: &[M], 
                                               item_size: usize) -> Result<()> {
        if v.is_empty() {
            return Ok(());
        }
        self.write_tag(tag)?;
        let len = v.len() * item_size;
        let bytes = unsafe { ::std::slice::from_raw_parts(v as *const [M] as *const M as *const u8, len) };
        self.write_bytes(bytes)
    }

    /// Writes tag then message
    pub fn write_message_with_tag<M: MessageWrite>(&mut self, tag: u32, m: &M) -> Result<()> {
        self.write_tag(tag)?;
        self.write_message(m)
    }

    /// Writes tag then enum
    pub fn write_enum_with_tag(&mut self, tag: u32, v: i32) -> Result<()> {
        self.write_tag(tag)?;
        self.write_int32(v)
    }
}
