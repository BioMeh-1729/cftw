/*!
This crate provides convenience methods for encoding and decoding numbers in
either [big-endian or little-endian order].

The organization of the crate is pretty simple. A trait, [`ByteOrder`], specifies
byte conversion methods for each type of number in Rust (sans numbers that have
a platform dependent size like `usize` and `isize`). Two types, [`BigEndian`]
and [`LittleEndian`] implement these methods. Finally, [`ReadBytesExt`] and
[`WriteBytesExt`] provide convenience methods available to all types that
implement [`Read`] and [`Write`].

An alias, [`NetworkEndian`], for [`BigEndian`] is provided to help improve
code clarity.

An additional alias, [`NativeEndian`], is provided for the endianness of the
local platform. This is convenient when serializing data for use and
conversions are not desired.

# Examples

Read unsigned 16 bit big-endian integers from a [`Read`] type:

```rust
use std::io::Cursor;
use byteorder::{BigEndian, ReadBytesExt};

let mut rdr = Cursor::new(vec![2, 5, 3, 0]);
// Note that we use type parameters to indicate which kind of byte order
// we want!
assert_eq!(517, rdr.read_u16::<BigEndian>().unwrap());
assert_eq!(768, rdr.read_u16::<BigEndian>().unwrap());
```

Write unsigned 16 bit little-endian integers to a [`Write`] type:

```rust
use byteorder::{LittleEndian, WriteBytesExt};

let mut wtr = vec![];
wtr.write_u16::<LittleEndian>(517).unwrap();
wtr.write_u16::<LittleEndian>(768).unwrap();
assert_eq!(wtr, vec![5, 2, 0, 3]);
```

# Optional Features

This crate optionally provides support for 128 bit values (`i128` and `u128`)
when built with the `i128` feature enabled.

This crate can also be used without the standard library.

# Alternatives

Note that as of Rust 1.32, the standard numeric types provide built-in methods
like `to_le_bytes` and `from_le_bytes`, which support some of the same use
cases.

[big-endian or little-endian order]: https://en.wikipedia.org/wiki/Endianness
[`ByteOrder`]: trait.ByteOrder.html
[`BigEndian`]: enum.BigEndian.html
[`LittleEndian`]: enum.LittleEndian.html
[`ReadBytesExt`]: trait.ReadBytesExt.html
[`WriteBytesExt`]: trait.WriteBytesExt.html
[`NetworkEndian`]: type.NetworkEndian.html
[`NativeEndian`]: type.NativeEndian.html
[`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
[`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
*/

#![deny(missing_docs)]
// When testing under miri, we disable tests that take too long. But this
// provokes lots of dead code warnings. So we just squash them.
#![cfg_attr(miri, allow(dead_code, unused_macros))]

use core::{
	convert::TryInto, fmt::Debug, hash::Hash, mem::align_of, ptr::copy_nonoverlapping, slice,
};

pub use io::{ReadBytesExt, WriteBytesExt};

mod io {
	use std::{
		io::{self, Result},
		slice,
	};

	use super::ByteOrder;

	/// Extends [`Read`] with methods for reading numbers. (For `std::io`.)
	///
	/// Most of the methods defined here have an unconstrained type parameter that
	/// must be explicitly instantiated. Typically, it is instantiated with either
	/// the [`BigEndian`] or [`LittleEndian`] types defined in this crate.
	///
	/// # Examples
	///
	/// Read unsigned 16 bit big-endian integers from a [`Read`]:
	///
	/// ```rust
	/// use std::io::Cursor;
	/// use byteorder::{BigEndian, ReadBytesExt};
	///
	/// let mut rdr = Cursor::new(vec![2, 5, 3, 0]);
	/// assert_eq!(517, rdr.read_u16::<BigEndian>().unwrap());
	/// assert_eq!(768, rdr.read_u16::<BigEndian>().unwrap());
	/// ```
	///
	/// [`BigEndian`]: enum.BigEndian.html
	/// [`LittleEndian`]: enum.LittleEndian.html
	/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
	pub trait ReadBytesExt: io::Read {
		/// Reads an unsigned 8 bit integer from the underlying reader.
		///
		/// Note that since this reads a single byte, no byte order conversions
		/// are used. It is included for completeness.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read unsigned 8 bit integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::ReadBytesExt;
		///
		/// let mut rdr = Cursor::new(vec![2, 5]);
		/// assert_eq!(2, rdr.read_u8().unwrap());
		/// assert_eq!(5, rdr.read_u8().unwrap());
		/// ```
		#[inline]
		fn read_u8(&mut self) -> Result<u8> {
			let mut buf = [0; 1];
			self.read_exact(&mut buf)?;
			Ok(buf[0])
		}

		/// Reads a signed 8 bit integer from the underlying reader.
		///
		/// Note that since this reads a single byte, no byte order conversions
		/// are used. It is included for completeness.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read signed 8 bit integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::ReadBytesExt;
		///
		/// let mut rdr = Cursor::new(vec![0x02, 0xfb]);
		/// assert_eq!(2, rdr.read_i8().unwrap());
		/// assert_eq!(-5, rdr.read_i8().unwrap());
		/// ```
		#[inline]
		fn read_i8(&mut self) -> Result<i8> {
			let mut buf = [0; 1];
			self.read_exact(&mut buf)?;
			Ok(buf[0] as i8)
		}

		/// Reads an unsigned 16 bit integer from the underlying reader.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read unsigned 16 bit big-endian integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![2, 5, 3, 0]);
		/// assert_eq!(517, rdr.read_u16::<BigEndian>().unwrap());
		/// assert_eq!(768, rdr.read_u16::<BigEndian>().unwrap());
		/// ```
		#[inline]
		fn read_u16<T: ByteOrder>(&mut self) -> Result<u16> {
			let mut buf = [0; 2];
			self.read_exact(&mut buf)?;
			Ok(T::read_u16(&buf))
		}

		/// Reads a signed 16 bit integer from the underlying reader.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read signed 16 bit big-endian integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![0x00, 0xc1, 0xff, 0x7c]);
		/// assert_eq!(193, rdr.read_i16::<BigEndian>().unwrap());
		/// assert_eq!(-132, rdr.read_i16::<BigEndian>().unwrap());
		/// ```
		#[inline]
		fn read_i16<T: ByteOrder>(&mut self) -> Result<i16> {
			let mut buf = [0; 2];
			self.read_exact(&mut buf)?;
			Ok(T::read_i16(&buf))
		}

		/// Reads an unsigned 24 bit integer from the underlying reader.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read unsigned 24 bit big-endian integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![0x00, 0x01, 0x0b]);
		/// assert_eq!(267, rdr.read_u24::<BigEndian>().unwrap());
		/// ```
		#[inline]
		fn read_u24<T: ByteOrder>(&mut self) -> Result<u32> {
			let mut buf = [0; 3];
			self.read_exact(&mut buf)?;
			Ok(T::read_u24(&buf))
		}

		/// Reads a signed 24 bit integer from the underlying reader.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read signed 24 bit big-endian integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![0xff, 0x7a, 0x33]);
		/// assert_eq!(-34253, rdr.read_i24::<BigEndian>().unwrap());
		/// ```
		#[inline]
		fn read_i24<T: ByteOrder>(&mut self) -> Result<i32> {
			let mut buf = [0; 3];
			self.read_exact(&mut buf)?;
			Ok(T::read_i24(&buf))
		}

		/// Reads an unsigned 32 bit integer from the underlying reader.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read unsigned 32 bit big-endian integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![0x00, 0x00, 0x01, 0x0b]);
		/// assert_eq!(267, rdr.read_u32::<BigEndian>().unwrap());
		/// ```
		#[inline]
		fn read_u32<T: ByteOrder>(&mut self) -> Result<u32> {
			let mut buf = [0; 4];
			self.read_exact(&mut buf)?;
			Ok(T::read_u32(&buf))
		}

		/// Reads a signed 32 bit integer from the underlying reader.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read signed 32 bit big-endian integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![0xff, 0xff, 0x7a, 0x33]);
		/// assert_eq!(-34253, rdr.read_i32::<BigEndian>().unwrap());
		/// ```
		#[inline]
		fn read_i32<T: ByteOrder>(&mut self) -> Result<i32> {
			let mut buf = [0; 4];
			self.read_exact(&mut buf)?;
			Ok(T::read_i32(&buf))
		}

		/// Reads an unsigned 48 bit integer from the underlying reader.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read unsigned 48 bit big-endian integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![0xb6, 0x71, 0x6b, 0xdc, 0x2b, 0x31]);
		/// assert_eq!(200598257150769, rdr.read_u48::<BigEndian>().unwrap());
		/// ```
		#[inline]
		fn read_u48<T: ByteOrder>(&mut self) -> Result<u64> {
			let mut buf = [0; 6];
			self.read_exact(&mut buf)?;
			Ok(T::read_u48(&buf))
		}

		/// Reads a signed 48 bit integer from the underlying reader.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read signed 48 bit big-endian integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![0x9d, 0x71, 0xab, 0xe7, 0x97, 0x8f]);
		/// assert_eq!(-108363435763825, rdr.read_i48::<BigEndian>().unwrap());
		/// ```
		#[inline]
		fn read_i48<T: ByteOrder>(&mut self) -> Result<i64> {
			let mut buf = [0; 6];
			self.read_exact(&mut buf)?;
			Ok(T::read_i48(&buf))
		}

		/// Reads an unsigned 64 bit integer from the underlying reader.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read an unsigned 64 bit big-endian integer from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![0x00, 0x03, 0x43, 0x95, 0x4d, 0x60, 0x86, 0x83]);
		/// assert_eq!(918733457491587, rdr.read_u64::<BigEndian>().unwrap());
		/// ```
		#[inline]
		fn read_u64<T: ByteOrder>(&mut self) -> Result<u64> {
			let mut buf = [0; 8];
			self.read_exact(&mut buf)?;
			Ok(T::read_u64(&buf))
		}

		/// Reads a signed 64 bit integer from the underlying reader.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read a signed 64 bit big-endian integer from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![0x80, 0, 0, 0, 0, 0, 0, 0]);
		/// assert_eq!(i64::min_value(), rdr.read_i64::<BigEndian>().unwrap());
		/// ```
		#[inline]
		fn read_i64<T: ByteOrder>(&mut self) -> Result<i64> {
			let mut buf = [0; 8];
			self.read_exact(&mut buf)?;
			Ok(T::read_i64(&buf))
		}

		/// Reads an unsigned 128 bit integer from the underlying reader.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read an unsigned 128 bit big-endian integer from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![
		///     0x00, 0x03, 0x43, 0x95, 0x4d, 0x60, 0x86, 0x83,
		///     0x00, 0x03, 0x43, 0x95, 0x4d, 0x60, 0x86, 0x83
		/// ]);
		/// assert_eq!(16947640962301618749969007319746179, rdr.read_u128::<BigEndian>().unwrap());
		/// ```
		#[inline]
		fn read_u128<T: ByteOrder>(&mut self) -> Result<u128> {
			let mut buf = [0; 16];
			self.read_exact(&mut buf)?;
			Ok(T::read_u128(&buf))
		}

		/// Reads a signed 128 bit integer from the underlying reader.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read a signed 128 bit big-endian integer from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
		/// assert_eq!(i128::min_value(), rdr.read_i128::<BigEndian>().unwrap());
		/// ```
		#[inline]
		fn read_i128<T: ByteOrder>(&mut self) -> Result<i128> {
			let mut buf = [0; 16];
			self.read_exact(&mut buf)?;
			Ok(T::read_i128(&buf))
		}

		/// Reads an unsigned n-bytes integer from the underlying reader.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read an unsigned n-byte big-endian integer from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![0x80, 0x74, 0xfa]);
		/// assert_eq!(8418554, rdr.read_uint::<BigEndian>(3).unwrap());
		#[inline]
		fn read_uint<T: ByteOrder>(
			&mut self,
			nbytes: usize,
		) -> Result<u64> {
			let mut buf = [0; 8];
			self.read_exact(&mut buf[..nbytes])?;
			Ok(T::read_uint(&buf[..nbytes], nbytes))
		}

		/// Reads a signed n-bytes integer from the underlying reader.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read an unsigned n-byte big-endian integer from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![0xc1, 0xff, 0x7c]);
		/// assert_eq!(-4063364, rdr.read_int::<BigEndian>(3).unwrap());
		#[inline]
		fn read_int<T: ByteOrder>(
			&mut self,
			nbytes: usize,
		) -> Result<i64> {
			let mut buf = [0; 8];
			self.read_exact(&mut buf[..nbytes])?;
			Ok(T::read_int(&buf[..nbytes], nbytes))
		}

		/// Reads an unsigned n-bytes integer from the underlying reader.
		#[inline]
		fn read_uint128<T: ByteOrder>(
			&mut self,
			nbytes: usize,
		) -> Result<u128> {
			let mut buf = [0; 16];
			self.read_exact(&mut buf[..nbytes])?;
			Ok(T::read_uint128(&buf[..nbytes], nbytes))
		}

		/// Reads a signed n-bytes integer from the underlying reader.
		#[inline]
		fn read_int128<T: ByteOrder>(
			&mut self,
			nbytes: usize,
		) -> Result<i128> {
			let mut buf = [0; 16];
			self.read_exact(&mut buf[..nbytes])?;
			Ok(T::read_int128(&buf[..nbytes], nbytes))
		}

		/// Reads a IEEE754 single-precision (4 bytes) floating point number from
		/// the underlying reader.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read a big-endian single-precision floating point number from a `Read`:
		///
		/// ```rust
		/// use std::f32;
		/// use std::io::Cursor;
		///
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![
		///     0x40, 0x49, 0x0f, 0xdb,
		/// ]);
		/// assert_eq!(f32::consts::PI, rdr.read_f32::<BigEndian>().unwrap());
		/// ```
		#[inline]
		fn read_f32<T: ByteOrder>(&mut self) -> Result<f32> {
			let mut buf = [0; 4];
			self.read_exact(&mut buf)?;
			Ok(T::read_f32(&buf))
		}

		/// Reads a IEEE754 double-precision (8 bytes) floating point number from
		/// the underlying reader.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read a big-endian double-precision floating point number from a `Read`:
		///
		/// ```rust
		/// use std::f64;
		/// use std::io::Cursor;
		///
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![
		///     0x40, 0x09, 0x21, 0xfb, 0x54, 0x44, 0x2d, 0x18,
		/// ]);
		/// assert_eq!(f64::consts::PI, rdr.read_f64::<BigEndian>().unwrap());
		/// ```
		#[inline]
		fn read_f64<T: ByteOrder>(&mut self) -> Result<f64> {
			let mut buf = [0; 8];
			self.read_exact(&mut buf)?;
			Ok(T::read_f64(&buf))
		}

		/// Reads a sequence of unsigned 16 bit integers from the underlying
		/// reader.
		///
		/// The given buffer is either filled completely or an error is returned.
		/// If an error is returned, the contents of `dst` are unspecified.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read a sequence of unsigned 16 bit big-endian integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![2, 5, 3, 0]);
		/// let mut dst = [0; 2];
		/// rdr.read_u16_into::<BigEndian>(&mut dst).unwrap();
		/// assert_eq!([517, 768], dst);
		/// ```
		#[inline]
		fn read_u16_into<T: ByteOrder>(
			&mut self,
			dst: &mut [u16],
		) -> Result<()> {
			{
				let buf = unsafe { slice_to_u8_mut(dst) };
				self.read_exact(buf)?;
			}
			T::from_slice_u16(dst);
			Ok(())
		}

		/// Reads a sequence of unsigned 32 bit integers from the underlying
		/// reader.
		///
		/// The given buffer is either filled completely or an error is returned.
		/// If an error is returned, the contents of `dst` are unspecified.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read a sequence of unsigned 32 bit big-endian integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![0, 0, 2, 5, 0, 0, 3, 0]);
		/// let mut dst = [0; 2];
		/// rdr.read_u32_into::<BigEndian>(&mut dst).unwrap();
		/// assert_eq!([517, 768], dst);
		/// ```
		#[inline]
		fn read_u32_into<T: ByteOrder>(
			&mut self,
			dst: &mut [u32],
		) -> Result<()> {
			{
				let buf = unsafe { slice_to_u8_mut(dst) };
				self.read_exact(buf)?;
			}
			T::from_slice_u32(dst);
			Ok(())
		}

		/// Reads a sequence of unsigned 64 bit integers from the underlying
		/// reader.
		///
		/// The given buffer is either filled completely or an error is returned.
		/// If an error is returned, the contents of `dst` are unspecified.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read a sequence of unsigned 64 bit big-endian integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![
		///     0, 0, 0, 0, 0, 0, 2, 5,
		///     0, 0, 0, 0, 0, 0, 3, 0,
		/// ]);
		/// let mut dst = [0; 2];
		/// rdr.read_u64_into::<BigEndian>(&mut dst).unwrap();
		/// assert_eq!([517, 768], dst);
		/// ```
		#[inline]
		fn read_u64_into<T: ByteOrder>(
			&mut self,
			dst: &mut [u64],
		) -> Result<()> {
			{
				let buf = unsafe { slice_to_u8_mut(dst) };
				self.read_exact(buf)?;
			}
			T::from_slice_u64(dst);
			Ok(())
		}

		/// Reads a sequence of unsigned 128 bit integers from the underlying
		/// reader.
		///
		/// The given buffer is either filled completely or an error is returned.
		/// If an error is returned, the contents of `dst` are unspecified.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read a sequence of unsigned 128 bit big-endian integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![
		///     0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 5,
		///     0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0,
		/// ]);
		/// let mut dst = [0; 2];
		/// rdr.read_u128_into::<BigEndian>(&mut dst).unwrap();
		/// assert_eq!([517, 768], dst);
		/// ```
		#[inline]
		fn read_u128_into<T: ByteOrder>(
			&mut self,
			dst: &mut [u128],
		) -> Result<()> {
			{
				let buf = unsafe { slice_to_u8_mut(dst) };
				self.read_exact(buf)?;
			}
			T::from_slice_u128(dst);
			Ok(())
		}

		/// Reads a sequence of signed 8 bit integers from the underlying reader.
		///
		/// The given buffer is either filled completely or an error is returned.
		/// If an error is returned, the contents of `dst` are unspecified.
		///
		/// Note that since each `i8` is a single byte, no byte order conversions
		/// are used. This method is included because it provides a safe, simple
		/// way for the caller to read into a `&mut [i8]` buffer. (Without this
		/// method, the caller would have to either use `unsafe` code or convert
		/// each byte to `i8` individually.)
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read a sequence of signed 8 bit integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![2, 251, 3]);
		/// let mut dst = [0; 3];
		/// rdr.read_i8_into(&mut dst).unwrap();
		/// assert_eq!([2, -5, 3], dst);
		/// ```
		#[inline]
		fn read_i8_into(
			&mut self,
			dst: &mut [i8],
		) -> Result<()> {
			let buf = unsafe { slice_to_u8_mut(dst) };
			self.read_exact(buf)
		}

		/// Reads a sequence of signed 16 bit integers from the underlying
		/// reader.
		///
		/// The given buffer is either filled completely or an error is returned.
		/// If an error is returned, the contents of `dst` are unspecified.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read a sequence of signed 16 bit big-endian integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![2, 5, 3, 0]);
		/// let mut dst = [0; 2];
		/// rdr.read_i16_into::<BigEndian>(&mut dst).unwrap();
		/// assert_eq!([517, 768], dst);
		/// ```
		#[inline]
		fn read_i16_into<T: ByteOrder>(
			&mut self,
			dst: &mut [i16],
		) -> Result<()> {
			{
				let buf = unsafe { slice_to_u8_mut(dst) };
				self.read_exact(buf)?;
			}
			T::from_slice_i16(dst);
			Ok(())
		}

		/// Reads a sequence of signed 32 bit integers from the underlying
		/// reader.
		///
		/// The given buffer is either filled completely or an error is returned.
		/// If an error is returned, the contents of `dst` are unspecified.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read a sequence of signed 32 bit big-endian integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![0, 0, 2, 5, 0, 0, 3, 0]);
		/// let mut dst = [0; 2];
		/// rdr.read_i32_into::<BigEndian>(&mut dst).unwrap();
		/// assert_eq!([517, 768], dst);
		/// ```
		#[inline]
		fn read_i32_into<T: ByteOrder>(
			&mut self,
			dst: &mut [i32],
		) -> Result<()> {
			{
				let buf = unsafe { slice_to_u8_mut(dst) };
				self.read_exact(buf)?;
			}
			T::from_slice_i32(dst);
			Ok(())
		}

		/// Reads a sequence of signed 64 bit integers from the underlying
		/// reader.
		///
		/// The given buffer is either filled completely or an error is returned.
		/// If an error is returned, the contents of `dst` are unspecified.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read a sequence of signed 64 bit big-endian integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![
		///     0, 0, 0, 0, 0, 0, 2, 5,
		///     0, 0, 0, 0, 0, 0, 3, 0,
		/// ]);
		/// let mut dst = [0; 2];
		/// rdr.read_i64_into::<BigEndian>(&mut dst).unwrap();
		/// assert_eq!([517, 768], dst);
		/// ```
		#[inline]
		fn read_i64_into<T: ByteOrder>(
			&mut self,
			dst: &mut [i64],
		) -> Result<()> {
			{
				let buf = unsafe { slice_to_u8_mut(dst) };
				self.read_exact(buf)?;
			}
			T::from_slice_i64(dst);
			Ok(())
		}

		/// Reads a sequence of signed 128 bit integers from the underlying
		/// reader.
		///
		/// The given buffer is either filled completely or an error is returned.
		/// If an error is returned, the contents of `dst` are unspecified.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read a sequence of signed 128 bit big-endian integers from a `Read`:
		///
		/// ```rust
		/// use std::io::Cursor;
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![
		///     0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 5,
		///     0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0,
		/// ]);
		/// let mut dst = [0; 2];
		/// rdr.read_i128_into::<BigEndian>(&mut dst).unwrap();
		/// assert_eq!([517, 768], dst);
		/// ```
		#[inline]
		fn read_i128_into<T: ByteOrder>(
			&mut self,
			dst: &mut [i128],
		) -> Result<()> {
			{
				let buf = unsafe { slice_to_u8_mut(dst) };
				self.read_exact(buf)?;
			}
			T::from_slice_i128(dst);
			Ok(())
		}

		/// Reads a sequence of IEEE754 single-precision (4 bytes) floating
		/// point numbers from the underlying reader.
		///
		/// The given buffer is either filled completely or an error is returned.
		/// If an error is returned, the contents of `dst` are unspecified.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read a sequence of big-endian single-precision floating point number
		/// from a `Read`:
		///
		/// ```rust
		/// use std::f32;
		/// use std::io::Cursor;
		///
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![
		///     0x40, 0x49, 0x0f, 0xdb,
		///     0x3f, 0x80, 0x00, 0x00,
		/// ]);
		/// let mut dst = [0.0; 2];
		/// rdr.read_f32_into::<BigEndian>(&mut dst).unwrap();
		/// assert_eq!([f32::consts::PI, 1.0], dst);
		/// ```
		#[inline]
		fn read_f32_into<T: ByteOrder>(
			&mut self,
			dst: &mut [f32],
		) -> Result<()> {
			{
				let buf = unsafe { slice_to_u8_mut(dst) };
				self.read_exact(buf)?;
			}
			T::from_slice_f32(dst);
			Ok(())
		}

		/// **DEPRECATED**.
		///
		/// This method is deprecated. Use `read_f32_into` instead.
		///
		/// Reads a sequence of IEEE754 single-precision (4 bytes) floating
		/// point numbers from the underlying reader.
		///
		/// The given buffer is either filled completely or an error is returned.
		/// If an error is returned, the contents of `dst` are unspecified.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read a sequence of big-endian single-precision floating point number
		/// from a `Read`:
		///
		/// ```rust
		/// use std::f32;
		/// use std::io::Cursor;
		///
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![
		///     0x40, 0x49, 0x0f, 0xdb,
		///     0x3f, 0x80, 0x00, 0x00,
		/// ]);
		/// let mut dst = [0.0; 2];
		/// rdr.read_f32_into_unchecked::<BigEndian>(&mut dst).unwrap();
		/// assert_eq!([f32::consts::PI, 1.0], dst);
		/// ```
		#[inline]
		#[deprecated(since = "1.2.0", note = "please use `read_f32_into` instead")]
		fn read_f32_into_unchecked<T: ByteOrder>(
			&mut self,
			dst: &mut [f32],
		) -> Result<()> {
			self.read_f32_into::<T>(dst)
		}

		/// Reads a sequence of IEEE754 double-precision (8 bytes) floating
		/// point numbers from the underlying reader.
		///
		/// The given buffer is either filled completely or an error is returned.
		/// If an error is returned, the contents of `dst` are unspecified.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read a sequence of big-endian single-precision floating point number
		/// from a `Read`:
		///
		/// ```rust
		/// use std::f64;
		/// use std::io::Cursor;
		///
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![
		///     0x40, 0x09, 0x21, 0xfb, 0x54, 0x44, 0x2d, 0x18,
		///     0x3f, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
		/// ]);
		/// let mut dst = [0.0; 2];
		/// rdr.read_f64_into::<BigEndian>(&mut dst).unwrap();
		/// assert_eq!([f64::consts::PI, 1.0], dst);
		/// ```
		#[inline]
		fn read_f64_into<T: ByteOrder>(
			&mut self,
			dst: &mut [f64],
		) -> Result<()> {
			{
				let buf = unsafe { slice_to_u8_mut(dst) };
				self.read_exact(buf)?;
			}
			T::from_slice_f64(dst);
			Ok(())
		}

		/// **DEPRECATED**.
		///
		/// This method is deprecated. Use `read_f64_into` instead.
		///
		/// Reads a sequence of IEEE754 double-precision (8 bytes) floating
		/// point numbers from the underlying reader.
		///
		/// The given buffer is either filled completely or an error is returned.
		/// If an error is returned, the contents of `dst` are unspecified.
		///
		/// # Safety
		///
		/// This method is unsafe because there are no guarantees made about the
		/// floating point values. In particular, this method does not check for
		/// signaling NaNs, which may result in undefined behavior.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Read::read_exact`].
		///
		/// [`Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
		///
		/// # Examples
		///
		/// Read a sequence of big-endian single-precision floating point number
		/// from a `Read`:
		///
		/// ```rust
		/// use std::f64;
		/// use std::io::Cursor;
		///
		/// use byteorder::{BigEndian, ReadBytesExt};
		///
		/// let mut rdr = Cursor::new(vec![
		///     0x40, 0x09, 0x21, 0xfb, 0x54, 0x44, 0x2d, 0x18,
		///     0x3f, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
		/// ]);
		/// let mut dst = [0.0; 2];
		/// rdr.read_f64_into_unchecked::<BigEndian>(&mut dst).unwrap();
		/// assert_eq!([f64::consts::PI, 1.0], dst);
		/// ```
		#[inline]
		#[deprecated(since = "1.2.0", note = "please use `read_f64_into` instead")]
		fn read_f64_into_unchecked<T: ByteOrder>(
			&mut self,
			dst: &mut [f64],
		) -> Result<()> {
			self.read_f64_into::<T>(dst)
		}
	}

	/// All types that implement `Read` get methods defined in `ReadBytesExt`
	/// for free.
	impl<R: io::Read + ?Sized> ReadBytesExt for R {}

	/// Extends [`Write`] with methods for writing numbers. (For `std::io`.)
	///
	/// Most of the methods defined here have an unconstrained type parameter that
	/// must be explicitly instantiated. Typically, it is instantiated with either
	/// the [`BigEndian`] or [`LittleEndian`] types defined in this crate.
	///
	/// # Examples
	///
	/// Write unsigned 16 bit big-endian integers to a [`Write`]:
	///
	/// ```rust
	/// use byteorder::{BigEndian, WriteBytesExt};
	///
	/// let mut wtr = vec![];
	/// wtr.write_u16::<BigEndian>(517).unwrap();
	/// wtr.write_u16::<BigEndian>(768).unwrap();
	/// assert_eq!(wtr, vec![2, 5, 3, 0]);
	/// ```
	///
	/// [`BigEndian`]: enum.BigEndian.html
	/// [`LittleEndian`]: enum.LittleEndian.html
	/// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
	pub trait WriteBytesExt: io::Write {
		/// Writes an unsigned 8 bit integer to the underlying writer.
		///
		/// Note that since this writes a single byte, no byte order conversions
		/// are used. It is included for completeness.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Write::write_all`].
		///
		/// [`Write::write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all
		///
		/// # Examples
		///
		/// Write unsigned 8 bit integers to a `Write`:
		///
		/// ```rust
		/// use byteorder::WriteBytesExt;
		///
		/// let mut wtr = Vec::new();
		/// wtr.write_u8(2).unwrap();
		/// wtr.write_u8(5).unwrap();
		/// assert_eq!(wtr, b"\x02\x05");
		/// ```
		#[inline]
		fn write_u8(
			&mut self,
			n: u8,
		) -> Result<()> {
			self.write_all(&[n])
		}

		/// Writes a signed 8 bit integer to the underlying writer.
		///
		/// Note that since this writes a single byte, no byte order conversions
		/// are used. It is included for completeness.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Write::write_all`].
		///
		/// [`Write::write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all
		///
		/// # Examples
		///
		/// Write signed 8 bit integers to a `Write`:
		///
		/// ```rust
		/// use byteorder::WriteBytesExt;
		///
		/// let mut wtr = Vec::new();
		/// wtr.write_i8(2).unwrap();
		/// wtr.write_i8(-5).unwrap();
		/// assert_eq!(wtr, b"\x02\xfb");
		/// ```
		#[inline]
		fn write_i8(
			&mut self,
			n: i8,
		) -> Result<()> {
			self.write_all(&[n as u8])
		}

		/// Writes an unsigned 16 bit integer to the underlying writer.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Write::write_all`].
		///
		/// [`Write::write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all
		///
		/// # Examples
		///
		/// Write unsigned 16 bit big-endian integers to a `Write`:
		///
		/// ```rust
		/// use byteorder::{BigEndian, WriteBytesExt};
		///
		/// let mut wtr = Vec::new();
		/// wtr.write_u16::<BigEndian>(517).unwrap();
		/// wtr.write_u16::<BigEndian>(768).unwrap();
		/// assert_eq!(wtr, b"\x02\x05\x03\x00");
		/// ```
		#[inline]
		fn write_u16<T: ByteOrder>(
			&mut self,
			n: u16,
		) -> Result<()> {
			let mut buf = [0; 2];
			T::write_u16(&mut buf, n);
			self.write_all(&buf)
		}

		/// Writes a signed 16 bit integer to the underlying writer.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Write::write_all`].
		///
		/// [`Write::write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all
		///
		/// # Examples
		///
		/// Write signed 16 bit big-endian integers to a `Write`:
		///
		/// ```rust
		/// use byteorder::{BigEndian, WriteBytesExt};
		///
		/// let mut wtr = Vec::new();
		/// wtr.write_i16::<BigEndian>(193).unwrap();
		/// wtr.write_i16::<BigEndian>(-132).unwrap();
		/// assert_eq!(wtr, b"\x00\xc1\xff\x7c");
		/// ```
		#[inline]
		fn write_i16<T: ByteOrder>(
			&mut self,
			n: i16,
		) -> Result<()> {
			let mut buf = [0; 2];
			T::write_i16(&mut buf, n);
			self.write_all(&buf)
		}

		/// Writes an unsigned 24 bit integer to the underlying writer.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Write::write_all`].
		///
		/// [`Write::write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all
		///
		/// # Examples
		///
		/// Write unsigned 24 bit big-endian integers to a `Write`:
		///
		/// ```rust
		/// use byteorder::{BigEndian, WriteBytesExt};
		///
		/// let mut wtr = Vec::new();
		/// wtr.write_u24::<BigEndian>(267).unwrap();
		/// wtr.write_u24::<BigEndian>(120111).unwrap();
		/// assert_eq!(wtr, b"\x00\x01\x0b\x01\xd5\x2f");
		/// ```
		#[inline]
		fn write_u24<T: ByteOrder>(
			&mut self,
			n: u32,
		) -> Result<()> {
			let mut buf = [0; 3];
			T::write_u24(&mut buf, n);
			self.write_all(&buf)
		}

		/// Writes a signed 24 bit integer to the underlying writer.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Write::write_all`].
		///
		/// [`Write::write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all
		///
		/// # Examples
		///
		/// Write signed 24 bit big-endian integers to a `Write`:
		///
		/// ```rust
		/// use byteorder::{BigEndian, WriteBytesExt};
		///
		/// let mut wtr = Vec::new();
		/// wtr.write_i24::<BigEndian>(-34253).unwrap();
		/// wtr.write_i24::<BigEndian>(120111).unwrap();
		/// assert_eq!(wtr, b"\xff\x7a\x33\x01\xd5\x2f");
		/// ```
		#[inline]
		fn write_i24<T: ByteOrder>(
			&mut self,
			n: i32,
		) -> Result<()> {
			let mut buf = [0; 3];
			T::write_i24(&mut buf, n);
			self.write_all(&buf)
		}

		/// Writes an unsigned 32 bit integer to the underlying writer.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Write::write_all`].
		///
		/// [`Write::write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all
		///
		/// # Examples
		///
		/// Write unsigned 32 bit big-endian integers to a `Write`:
		///
		/// ```rust
		/// use byteorder::{BigEndian, WriteBytesExt};
		///
		/// let mut wtr = Vec::new();
		/// wtr.write_u32::<BigEndian>(267).unwrap();
		/// wtr.write_u32::<BigEndian>(1205419366).unwrap();
		/// assert_eq!(wtr, b"\x00\x00\x01\x0b\x47\xd9\x3d\x66");
		/// ```
		#[inline]
		fn write_u32<T: ByteOrder>(
			&mut self,
			n: u32,
		) -> Result<()> {
			let mut buf = [0; 4];
			T::write_u32(&mut buf, n);
			self.write_all(&buf)
		}

		/// Writes a signed 32 bit integer to the underlying writer.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Write::write_all`].
		///
		/// [`Write::write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all
		///
		/// # Examples
		///
		/// Write signed 32 bit big-endian integers to a `Write`:
		///
		/// ```rust
		/// use byteorder::{BigEndian, WriteBytesExt};
		///
		/// let mut wtr = Vec::new();
		/// wtr.write_i32::<BigEndian>(-34253).unwrap();
		/// wtr.write_i32::<BigEndian>(1205419366).unwrap();
		/// assert_eq!(wtr, b"\xff\xff\x7a\x33\x47\xd9\x3d\x66");
		/// ```
		#[inline]
		fn write_i32<T: ByteOrder>(
			&mut self,
			n: i32,
		) -> Result<()> {
			let mut buf = [0; 4];
			T::write_i32(&mut buf, n);
			self.write_all(&buf)
		}

		/// Writes an unsigned 48 bit integer to the underlying writer.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Write::write_all`].
		///
		/// [`Write::write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all
		///
		/// # Examples
		///
		/// Write unsigned 48 bit big-endian integers to a `Write`:
		///
		/// ```rust
		/// use byteorder::{BigEndian, WriteBytesExt};
		///
		/// let mut wtr = Vec::new();
		/// wtr.write_u48::<BigEndian>(52360336390828).unwrap();
		/// wtr.write_u48::<BigEndian>(541).unwrap();
		/// assert_eq!(wtr, b"\x2f\x9f\x17\x40\x3a\xac\x00\x00\x00\x00\x02\x1d");
		/// ```
		#[inline]
		fn write_u48<T: ByteOrder>(
			&mut self,
			n: u64,
		) -> Result<()> {
			let mut buf = [0; 6];
			T::write_u48(&mut buf, n);
			self.write_all(&buf)
		}

		/// Writes a signed 48 bit integer to the underlying writer.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Write::write_all`].
		///
		/// [`Write::write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all
		///
		/// # Examples
		///
		/// Write signed 48 bit big-endian integers to a `Write`:
		///
		/// ```rust
		/// use byteorder::{BigEndian, WriteBytesExt};
		///
		/// let mut wtr = Vec::new();
		/// wtr.write_i48::<BigEndian>(-108363435763825).unwrap();
		/// wtr.write_i48::<BigEndian>(77).unwrap();
		/// assert_eq!(wtr, b"\x9d\x71\xab\xe7\x97\x8f\x00\x00\x00\x00\x00\x4d");
		/// ```
		#[inline]
		fn write_i48<T: ByteOrder>(
			&mut self,
			n: i64,
		) -> Result<()> {
			let mut buf = [0; 6];
			T::write_i48(&mut buf, n);
			self.write_all(&buf)
		}

		/// Writes an unsigned 64 bit integer to the underlying writer.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Write::write_all`].
		///
		/// [`Write::write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all
		///
		/// # Examples
		///
		/// Write unsigned 64 bit big-endian integers to a `Write`:
		///
		/// ```rust
		/// use byteorder::{BigEndian, WriteBytesExt};
		///
		/// let mut wtr = Vec::new();
		/// wtr.write_u64::<BigEndian>(918733457491587).unwrap();
		/// wtr.write_u64::<BigEndian>(143).unwrap();
		/// assert_eq!(wtr, b"\x00\x03\x43\x95\x4d\x60\x86\x83\x00\x00\x00\x00\x00\x00\x00\x8f");
		/// ```
		#[inline]
		fn write_u64<T: ByteOrder>(
			&mut self,
			n: u64,
		) -> Result<()> {
			let mut buf = [0; 8];
			T::write_u64(&mut buf, n);
			self.write_all(&buf)
		}

		/// Writes a signed 64 bit integer to the underlying writer.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Write::write_all`].
		///
		/// [`Write::write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all
		///
		/// # Examples
		///
		/// Write signed 64 bit big-endian integers to a `Write`:
		///
		/// ```rust
		/// use byteorder::{BigEndian, WriteBytesExt};
		///
		/// let mut wtr = Vec::new();
		/// wtr.write_i64::<BigEndian>(i64::min_value()).unwrap();
		/// wtr.write_i64::<BigEndian>(i64::max_value()).unwrap();
		/// assert_eq!(wtr, b"\x80\x00\x00\x00\x00\x00\x00\x00\x7f\xff\xff\xff\xff\xff\xff\xff");
		/// ```
		#[inline]
		fn write_i64<T: ByteOrder>(
			&mut self,
			n: i64,
		) -> Result<()> {
			let mut buf = [0; 8];
			T::write_i64(&mut buf, n);
			self.write_all(&buf)
		}

		/// Writes an unsigned 128 bit integer to the underlying writer.
		#[inline]
		fn write_u128<T: ByteOrder>(
			&mut self,
			n: u128,
		) -> Result<()> {
			let mut buf = [0; 16];
			T::write_u128(&mut buf, n);
			self.write_all(&buf)
		}

		/// Writes a signed 128 bit integer to the underlying writer.
		#[inline]
		fn write_i128<T: ByteOrder>(
			&mut self,
			n: i128,
		) -> Result<()> {
			let mut buf = [0; 16];
			T::write_i128(&mut buf, n);
			self.write_all(&buf)
		}

		/// Writes an unsigned n-bytes integer to the underlying writer.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Write::write_all`].
		///
		/// [`Write::write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all
		///
		/// # Panics
		///
		/// If the given integer is not representable in the given number of bytes,
		/// this method panics. If `nbytes > 8`, this method panics.
		///
		/// # Examples
		///
		/// Write unsigned 40 bit big-endian integers to a `Write`:
		///
		/// ```rust
		/// use byteorder::{BigEndian, WriteBytesExt};
		///
		/// let mut wtr = Vec::new();
		/// wtr.write_uint::<BigEndian>(312550384361, 5).unwrap();
		/// wtr.write_uint::<BigEndian>(43, 5).unwrap();
		/// assert_eq!(wtr, b"\x48\xc5\x74\x62\xe9\x00\x00\x00\x00\x2b");
		/// ```
		#[inline]
		fn write_uint<T: ByteOrder>(
			&mut self,
			n: u64,
			nbytes: usize,
		) -> Result<()> {
			let mut buf = [0; 8];
			T::write_uint(&mut buf, n, nbytes);
			self.write_all(&buf[0..nbytes])
		}

		/// Writes a signed n-bytes integer to the underlying writer.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Write::write_all`].
		///
		/// [`Write::write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all
		///
		/// # Panics
		///
		/// If the given integer is not representable in the given number of bytes,
		/// this method panics. If `nbytes > 8`, this method panics.
		///
		/// # Examples
		///
		/// Write signed 56 bit big-endian integers to a `Write`:
		///
		/// ```rust
		/// use byteorder::{BigEndian, WriteBytesExt};
		///
		/// let mut wtr = Vec::new();
		/// wtr.write_int::<BigEndian>(-3548172039376767, 7).unwrap();
		/// wtr.write_int::<BigEndian>(43, 7).unwrap();
		/// assert_eq!(wtr, b"\xf3\x64\xf4\xd1\xfd\xb0\x81\x00\x00\x00\x00\x00\x00\x2b");
		/// ```
		#[inline]
		fn write_int<T: ByteOrder>(
			&mut self,
			n: i64,
			nbytes: usize,
		) -> Result<()> {
			let mut buf = [0; 8];
			T::write_int(&mut buf, n, nbytes);
			self.write_all(&buf[0..nbytes])
		}

		/// Writes an unsigned n-bytes integer to the underlying writer.
		///
		/// If the given integer is not representable in the given number of bytes,
		/// this method panics. If `nbytes > 16`, this method panics.
		#[inline]
		fn write_uint128<T: ByteOrder>(
			&mut self,
			n: u128,
			nbytes: usize,
		) -> Result<()> {
			let mut buf = [0; 16];
			T::write_uint128(&mut buf, n, nbytes);
			self.write_all(&buf[0..nbytes])
		}

		/// Writes a signed n-bytes integer to the underlying writer.
		///
		/// If the given integer is not representable in the given number of bytes,
		/// this method panics. If `nbytes > 16`, this method panics.
		#[inline]
		fn write_int128<T: ByteOrder>(
			&mut self,
			n: i128,
			nbytes: usize,
		) -> Result<()> {
			let mut buf = [0; 16];
			T::write_int128(&mut buf, n, nbytes);
			self.write_all(&buf[0..nbytes])
		}

		/// Writes a IEEE754 single-precision (4 bytes) floating point number to
		/// the underlying writer.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Write::write_all`].
		///
		/// [`Write::write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all
		///
		/// # Examples
		///
		/// Write a big-endian single-precision floating point number to a `Write`:
		///
		/// ```rust
		/// use std::f32;
		///
		/// use byteorder::{BigEndian, WriteBytesExt};
		///
		/// let mut wtr = Vec::new();
		/// wtr.write_f32::<BigEndian>(f32::consts::PI).unwrap();
		/// assert_eq!(wtr, b"\x40\x49\x0f\xdb");
		/// ```
		#[inline]
		fn write_f32<T: ByteOrder>(
			&mut self,
			n: f32,
		) -> Result<()> {
			let mut buf = [0; 4];
			T::write_f32(&mut buf, n);
			self.write_all(&buf)
		}

		/// Writes a IEEE754 double-precision (8 bytes) floating point number to
		/// the underlying writer.
		///
		/// # Errors
		///
		/// This method returns the same errors as [`Write::write_all`].
		///
		/// [`Write::write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all
		///
		/// # Examples
		///
		/// Write a big-endian double-precision floating point number to a `Write`:
		///
		/// ```rust
		/// use std::f64;
		///
		/// use byteorder::{BigEndian, WriteBytesExt};
		///
		/// let mut wtr = Vec::new();
		/// wtr.write_f64::<BigEndian>(f64::consts::PI).unwrap();
		/// assert_eq!(wtr, b"\x40\x09\x21\xfb\x54\x44\x2d\x18");
		/// ```
		#[inline]
		fn write_f64<T: ByteOrder>(
			&mut self,
			n: f64,
		) -> Result<()> {
			let mut buf = [0; 8];
			T::write_f64(&mut buf, n);
			self.write_all(&buf)
		}
	}

	/// All types that implement `Write` get methods defined in `WriteBytesExt`
	/// for free.
	impl<W: io::Write + ?Sized> WriteBytesExt for W {}

	/// Convert a slice of T (where T is plain old data) to its mutable binary
	/// representation.
	///
	/// This function is wildly unsafe because it permits arbitrary modification of
	/// the binary representation of any `Copy` type. Use with care. It's intended
	/// to be called only where `T` is a numeric type.
	unsafe fn slice_to_u8_mut<T: Copy>(slice: &mut [T]) -> &mut [u8] {
		use std::mem::size_of;

		let len = size_of::<T>() * slice.len();
		unsafe { slice::from_raw_parts_mut(slice.as_mut_ptr() as *mut u8, len) }
	}
}

#[inline]
fn extend_sign(
	val: u64,
	nbytes: usize,
) -> i64 {
	let shift = (8 - nbytes) * 8;
	(val << shift) as i64 >> shift
}

#[inline]
fn extend_sign128(
	val: u128,
	nbytes: usize,
) -> i128 {
	let shift = (16 - nbytes) * 8;
	(val << shift) as i128 >> shift
}

#[inline]
fn unextend_sign(
	val: i64,
	nbytes: usize,
) -> u64 {
	let shift = (8 - nbytes) * 8;
	(val << shift) as u64 >> shift
}

#[inline]
fn unextend_sign128(
	val: i128,
	nbytes: usize,
) -> u128 {
	let shift = (16 - nbytes) * 8;
	(val << shift) as u128 >> shift
}

#[inline]
fn pack_size(n: u64) -> usize {
	if n < 1 << 8 {
		1
	} else if n < 1 << 16 {
		2
	} else if n < 1 << 24 {
		3
	} else if n < 1 << 32 {
		4
	} else if n < 1 << 40 {
		5
	} else if n < 1 << 48 {
		6
	} else if n < 1 << 56 {
		7
	} else {
		8
	}
}

#[inline]
fn pack_size128(n: u128) -> usize {
	if n < 1 << 8 {
		1
	} else if n < 1 << 16 {
		2
	} else if n < 1 << 24 {
		3
	} else if n < 1 << 32 {
		4
	} else if n < 1 << 40 {
		5
	} else if n < 1 << 48 {
		6
	} else if n < 1 << 56 {
		7
	} else if n < 1 << 64 {
		8
	} else if n < 1 << 72 {
		9
	} else if n < 1 << 80 {
		10
	} else if n < 1 << 88 {
		11
	} else if n < 1 << 96 {
		12
	} else if n < 1 << 104 {
		13
	} else if n < 1 << 112 {
		14
	} else if n < 1 << 120 {
		15
	} else {
		16
	}
}

mod private {
	/// Sealed stops crates other than byteorder from implementing any traits
	/// that use it.
	pub trait Sealed {}
	impl Sealed for super::LittleEndian {}
	impl Sealed for super::BigEndian {}
}

/// `ByteOrder` describes types that can serialize integers as bytes.
///
/// Note that `Self` does not appear anywhere in this trait's definition!
/// Therefore, in order to use it, you'll need to use syntax like
/// `T::read_u16(&[0, 1])` where `T` implements `ByteOrder`.
///
/// This crate provides two types that implement `ByteOrder`: [`BigEndian`]
/// and [`LittleEndian`].
/// This trait is sealed and cannot be implemented for callers to avoid
/// breaking backwards compatibility when adding new derived traits.
///
/// # Examples
///
/// Write and read `u32` numbers in little endian order:
///
/// ```rust
/// use byteorder::{ByteOrder, LittleEndian};
///
/// let mut buf = [0; 4];
/// LittleEndian::write_u32(&mut buf, 1_000_000);
/// assert_eq!(1_000_000, LittleEndian::read_u32(&buf));
/// ```
///
/// Write and read `i16` numbers in big endian order:
///
/// ```rust
/// use byteorder::{ByteOrder, BigEndian};
///
/// let mut buf = [0; 2];
/// BigEndian::write_i16(&mut buf, -5_000);
/// assert_eq!(-5_000, BigEndian::read_i16(&buf));
/// ```
///
/// [`BigEndian`]: enum.BigEndian.html
/// [`LittleEndian`]: enum.LittleEndian.html
pub trait ByteOrder:
	Clone + Copy + Debug + Default + Eq + Hash + Ord + PartialEq + PartialOrd + private::Sealed
{
	/// Reads an unsigned 16 bit integer from `buf`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 2`.
	fn read_u16(buf: &[u8]) -> u16;

	/// Reads an unsigned 24 bit integer from `buf`, stored in u32.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 3`.
	///
	/// # Examples
	///
	/// Write and read 24 bit `u32` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 3];
	/// LittleEndian::write_u24(&mut buf, 1_000_000);
	/// assert_eq!(1_000_000, LittleEndian::read_u24(&buf));
	/// ```
	fn read_u24(buf: &[u8]) -> u32 {
		Self::read_uint(buf, 3) as u32
	}

	/// Reads an unsigned 32 bit integer from `buf`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 4`.
	///
	/// # Examples
	///
	/// Write and read `u32` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 4];
	/// LittleEndian::write_u32(&mut buf, 1_000_000);
	/// assert_eq!(1_000_000, LittleEndian::read_u32(&buf));
	/// ```
	fn read_u32(buf: &[u8]) -> u32;

	/// Reads an unsigned 48 bit integer from `buf`, stored in u64.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 6`.
	///
	/// # Examples
	///
	/// Write and read 48 bit `u64` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 6];
	/// LittleEndian::write_u48(&mut buf, 1_000_000_000_000);
	/// assert_eq!(1_000_000_000_000, LittleEndian::read_u48(&buf));
	/// ```
	fn read_u48(buf: &[u8]) -> u64 {
		Self::read_uint(buf, 6) as u64
	}

	/// Reads an unsigned 64 bit integer from `buf`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 8`.
	///
	/// # Examples
	///
	/// Write and read `u64` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 8];
	/// LittleEndian::write_u64(&mut buf, 1_000_000);
	/// assert_eq!(1_000_000, LittleEndian::read_u64(&buf));
	/// ```
	fn read_u64(buf: &[u8]) -> u64;

	/// Reads an unsigned 128 bit integer from `buf`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 16`.
	///
	/// # Examples
	///
	/// Write and read `u128` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 16];
	/// LittleEndian::write_u128(&mut buf, 1_000_000);
	/// assert_eq!(1_000_000, LittleEndian::read_u128(&buf));
	/// ```
	fn read_u128(buf: &[u8]) -> u128;

	/// Reads an unsigned n-bytes integer from `buf`.
	///
	/// # Panics
	///
	/// Panics when `nbytes < 1` or `nbytes > 8` or
	/// `buf.len() < nbytes`
	///
	/// # Examples
	///
	/// Write and read an n-byte number in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 3];
	/// LittleEndian::write_uint(&mut buf, 1_000_000, 3);
	/// assert_eq!(1_000_000, LittleEndian::read_uint(&buf, 3));
	/// ```
	fn read_uint(
		buf: &[u8],
		nbytes: usize,
	) -> u64;

	/// Reads an unsigned n-bytes integer from `buf`.
	///
	/// # Panics
	///
	/// Panics when `nbytes < 1` or `nbytes > 16` or
	/// `buf.len() < nbytes`
	///
	/// # Examples
	///
	/// Write and read an n-byte number in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 3];
	/// LittleEndian::write_uint128(&mut buf, 1_000_000, 3);
	/// assert_eq!(1_000_000, LittleEndian::read_uint128(&buf, 3));
	/// ```
	fn read_uint128(
		buf: &[u8],
		nbytes: usize,
	) -> u128;

	/// Writes an unsigned 16 bit integer `n` to `buf`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 2`.
	///
	/// # Examples
	///
	/// Write and read `u16` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 2];
	/// LittleEndian::write_u16(&mut buf, 1_000);
	/// assert_eq!(1_000, LittleEndian::read_u16(&buf));
	/// ```
	fn write_u16(
		buf: &mut [u8],
		n: u16,
	);

	/// Writes an unsigned 24 bit integer `n` to `buf`, stored in u32.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 3`.
	///
	/// # Examples
	///
	/// Write and read 24 bit `u32` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 3];
	/// LittleEndian::write_u24(&mut buf, 1_000_000);
	/// assert_eq!(1_000_000, LittleEndian::read_u24(&buf));
	/// ```
	fn write_u24(
		buf: &mut [u8],
		n: u32,
	) {
		Self::write_uint(buf, n as u64, 3)
	}

	/// Writes an unsigned 32 bit integer `n` to `buf`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 4`.
	///
	/// # Examples
	///
	/// Write and read `u32` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 4];
	/// LittleEndian::write_u32(&mut buf, 1_000_000);
	/// assert_eq!(1_000_000, LittleEndian::read_u32(&buf));
	/// ```
	fn write_u32(
		buf: &mut [u8],
		n: u32,
	);

	/// Writes an unsigned 48 bit integer `n` to `buf`, stored in u64.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 6`.
	///
	/// # Examples
	///
	/// Write and read 48 bit `u64` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 6];
	/// LittleEndian::write_u48(&mut buf, 1_000_000_000_000);
	/// assert_eq!(1_000_000_000_000, LittleEndian::read_u48(&buf));
	/// ```
	fn write_u48(
		buf: &mut [u8],
		n: u64,
	) {
		Self::write_uint(buf, n as u64, 6)
	}

	/// Writes an unsigned 64 bit integer `n` to `buf`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 8`.
	///
	/// # Examples
	///
	/// Write and read `u64` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 8];
	/// LittleEndian::write_u64(&mut buf, 1_000_000);
	/// assert_eq!(1_000_000, LittleEndian::read_u64(&buf));
	/// ```
	fn write_u64(
		buf: &mut [u8],
		n: u64,
	);

	/// Writes an unsigned 128 bit integer `n` to `buf`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 16`.
	///
	/// # Examples
	///
	/// Write and read `u128` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 16];
	/// LittleEndian::write_u128(&mut buf, 1_000_000);
	/// assert_eq!(1_000_000, LittleEndian::read_u128(&buf));
	/// ```
	fn write_u128(
		buf: &mut [u8],
		n: u128,
	);

	/// Writes an unsigned integer `n` to `buf` using only `nbytes`.
	///
	/// # Panics
	///
	/// If `n` is not representable in `nbytes`, or if `nbytes` is `> 8`, then
	/// this method panics.
	///
	/// # Examples
	///
	/// Write and read an n-byte number in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 3];
	/// LittleEndian::write_uint(&mut buf, 1_000_000, 3);
	/// assert_eq!(1_000_000, LittleEndian::read_uint(&buf, 3));
	/// ```
	fn write_uint(
		buf: &mut [u8],
		n: u64,
		nbytes: usize,
	);

	/// Writes an unsigned integer `n` to `buf` using only `nbytes`.
	///
	/// # Panics
	///
	/// If `n` is not representable in `nbytes`, or if `nbytes` is `> 16`, then
	/// this method panics.
	///
	/// # Examples
	///
	/// Write and read an n-byte number in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 3];
	/// LittleEndian::write_uint128(&mut buf, 1_000_000, 3);
	/// assert_eq!(1_000_000, LittleEndian::read_uint128(&buf, 3));
	/// ```
	fn write_uint128(
		buf: &mut [u8],
		n: u128,
		nbytes: usize,
	);

	/// Reads a signed 16 bit integer from `buf`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 2`.
	///
	/// # Examples
	///
	/// Write and read `i16` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 2];
	/// LittleEndian::write_i16(&mut buf, -1_000);
	/// assert_eq!(-1_000, LittleEndian::read_i16(&buf));
	/// ```
	#[inline]
	fn read_i16(buf: &[u8]) -> i16 {
		Self::read_u16(buf) as i16
	}

	/// Reads a signed 24 bit integer from `buf`, stored in i32.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 3`.
	///
	/// # Examples
	///
	/// Write and read 24 bit `i32` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 3];
	/// LittleEndian::write_i24(&mut buf, -1_000_000);
	/// assert_eq!(-1_000_000, LittleEndian::read_i24(&buf));
	/// ```
	#[inline]
	fn read_i24(buf: &[u8]) -> i32 {
		Self::read_int(buf, 3) as i32
	}

	/// Reads a signed 32 bit integer from `buf`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 4`.
	///
	/// # Examples
	///
	/// Write and read `i32` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 4];
	/// LittleEndian::write_i32(&mut buf, -1_000_000);
	/// assert_eq!(-1_000_000, LittleEndian::read_i32(&buf));
	/// ```
	#[inline]
	fn read_i32(buf: &[u8]) -> i32 {
		Self::read_u32(buf) as i32
	}

	/// Reads a signed 48 bit integer from `buf`, stored in i64.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 6`.
	///
	/// # Examples
	///
	/// Write and read 48 bit `i64` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 6];
	/// LittleEndian::write_i48(&mut buf, -1_000_000_000_000);
	/// assert_eq!(-1_000_000_000_000, LittleEndian::read_i48(&buf));
	/// ```
	#[inline]
	fn read_i48(buf: &[u8]) -> i64 {
		Self::read_int(buf, 6) as i64
	}

	/// Reads a signed 64 bit integer from `buf`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 8`.
	///
	/// # Examples
	///
	/// Write and read `i64` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 8];
	/// LittleEndian::write_i64(&mut buf, -1_000_000_000);
	/// assert_eq!(-1_000_000_000, LittleEndian::read_i64(&buf));
	/// ```
	#[inline]
	fn read_i64(buf: &[u8]) -> i64 {
		Self::read_u64(buf) as i64
	}

	/// Reads a signed 128 bit integer from `buf`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 16`.
	///
	/// # Examples
	///
	/// Write and read `i128` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 16];
	/// LittleEndian::write_i128(&mut buf, -1_000_000_000);
	/// assert_eq!(-1_000_000_000, LittleEndian::read_i128(&buf));
	/// ```
	#[inline]
	fn read_i128(buf: &[u8]) -> i128 {
		Self::read_u128(buf) as i128
	}

	/// Reads a signed n-bytes integer from `buf`.
	///
	/// # Panics
	///
	/// Panics when `nbytes < 1` or `nbytes > 8` or
	/// `buf.len() < nbytes`
	///
	/// # Examples
	///
	/// Write and read n-length signed numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 3];
	/// LittleEndian::write_int(&mut buf, -1_000, 3);
	/// assert_eq!(-1_000, LittleEndian::read_int(&buf, 3));
	/// ```
	#[inline]
	fn read_int(
		buf: &[u8],
		nbytes: usize,
	) -> i64 {
		extend_sign(Self::read_uint(buf, nbytes), nbytes)
	}

	/// Reads a signed n-bytes integer from `buf`.
	///
	/// # Panics
	///
	/// Panics when `nbytes < 1` or `nbytes > 16` or
	/// `buf.len() < nbytes`
	///
	/// # Examples
	///
	/// Write and read n-length signed numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 3];
	/// LittleEndian::write_int128(&mut buf, -1_000, 3);
	/// assert_eq!(-1_000, LittleEndian::read_int128(&buf, 3));
	/// ```
	#[inline]
	fn read_int128(
		buf: &[u8],
		nbytes: usize,
	) -> i128 {
		extend_sign128(Self::read_uint128(buf, nbytes), nbytes)
	}

	/// Reads a IEEE754 single-precision (4 bytes) floating point number.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 4`.
	///
	/// # Examples
	///
	/// Write and read `f32` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let e = 2.71828;
	/// let mut buf = [0; 4];
	/// LittleEndian::write_f32(&mut buf, e);
	/// assert_eq!(e, LittleEndian::read_f32(&buf));
	/// ```
	#[inline]
	fn read_f32(buf: &[u8]) -> f32 {
		f32::from_bits(Self::read_u32(buf))
	}

	/// Reads a IEEE754 double-precision (8 bytes) floating point number.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 8`.
	///
	/// # Examples
	///
	/// Write and read `f64` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let phi = 1.6180339887;
	/// let mut buf = [0; 8];
	/// LittleEndian::write_f64(&mut buf, phi);
	/// assert_eq!(phi, LittleEndian::read_f64(&buf));
	/// ```
	#[inline]
	fn read_f64(buf: &[u8]) -> f64 {
		f64::from_bits(Self::read_u64(buf))
	}

	/// Writes a signed 16 bit integer `n` to `buf`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 2`.
	///
	/// # Examples
	///
	/// Write and read `i16` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 2];
	/// LittleEndian::write_i16(&mut buf, -1_000);
	/// assert_eq!(-1_000, LittleEndian::read_i16(&buf));
	/// ```
	#[inline]
	fn write_i16(
		buf: &mut [u8],
		n: i16,
	) {
		Self::write_u16(buf, n as u16)
	}

	/// Writes a signed 24 bit integer `n` to `buf`, stored in i32.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 3`.
	///
	/// # Examples
	///
	/// Write and read 24 bit `i32` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 3];
	/// LittleEndian::write_i24(&mut buf, -1_000_000);
	/// assert_eq!(-1_000_000, LittleEndian::read_i24(&buf));
	/// ```
	#[inline]
	fn write_i24(
		buf: &mut [u8],
		n: i32,
	) {
		Self::write_int(buf, n as i64, 3)
	}

	/// Writes a signed 32 bit integer `n` to `buf`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 4`.
	///
	/// # Examples
	///
	/// Write and read `i32` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 4];
	/// LittleEndian::write_i32(&mut buf, -1_000_000);
	/// assert_eq!(-1_000_000, LittleEndian::read_i32(&buf));
	/// ```
	#[inline]
	fn write_i32(
		buf: &mut [u8],
		n: i32,
	) {
		Self::write_u32(buf, n as u32)
	}

	/// Writes a signed 48 bit integer `n` to `buf`, stored in i64.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 6`.
	///
	/// # Examples
	///
	/// Write and read 48 bit `i64` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 6];
	/// LittleEndian::write_i48(&mut buf, -1_000_000_000_000);
	/// assert_eq!(-1_000_000_000_000, LittleEndian::read_i48(&buf));
	/// ```
	#[inline]
	fn write_i48(
		buf: &mut [u8],
		n: i64,
	) {
		Self::write_int(buf, n as i64, 6)
	}

	/// Writes a signed 64 bit integer `n` to `buf`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 8`.
	///
	/// # Examples
	///
	/// Write and read `i64` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 8];
	/// LittleEndian::write_i64(&mut buf, -1_000_000_000);
	/// assert_eq!(-1_000_000_000, LittleEndian::read_i64(&buf));
	/// ```
	#[inline]
	fn write_i64(
		buf: &mut [u8],
		n: i64,
	) {
		Self::write_u64(buf, n as u64)
	}

	/// Writes a signed 128 bit integer `n` to `buf`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 16`.
	///
	/// # Examples
	///
	/// Write and read n-byte `i128` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 16];
	/// LittleEndian::write_i128(&mut buf, -1_000_000_000);
	/// assert_eq!(-1_000_000_000, LittleEndian::read_i128(&buf));
	/// ```
	#[inline]
	fn write_i128(
		buf: &mut [u8],
		n: i128,
	) {
		Self::write_u128(buf, n as u128)
	}

	/// Writes a signed integer `n` to `buf` using only `nbytes`.
	///
	/// # Panics
	///
	/// If `n` is not representable in `nbytes`, or if `nbytes` is `> 8`, then
	/// this method panics.
	///
	/// # Examples
	///
	/// Write and read an n-byte number in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 3];
	/// LittleEndian::write_int(&mut buf, -1_000, 3);
	/// assert_eq!(-1_000, LittleEndian::read_int(&buf, 3));
	/// ```
	#[inline]
	fn write_int(
		buf: &mut [u8],
		n: i64,
		nbytes: usize,
	) {
		Self::write_uint(buf, unextend_sign(n, nbytes), nbytes)
	}

	/// Writes a signed integer `n` to `buf` using only `nbytes`.
	///
	/// # Panics
	///
	/// If `n` is not representable in `nbytes`, or if `nbytes` is `> 16`, then
	/// this method panics.
	///
	/// # Examples
	///
	/// Write and read n-length signed numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut buf = [0; 3];
	/// LittleEndian::write_int128(&mut buf, -1_000, 3);
	/// assert_eq!(-1_000, LittleEndian::read_int128(&buf, 3));
	/// ```
	#[inline]
	fn write_int128(
		buf: &mut [u8],
		n: i128,
		nbytes: usize,
	) {
		Self::write_uint128(buf, unextend_sign128(n, nbytes), nbytes)
	}

	/// Writes a IEEE754 single-precision (4 bytes) floating point number.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 4`.
	///
	/// # Examples
	///
	/// Write and read `f32` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let e = 2.71828;
	/// let mut buf = [0; 4];
	/// LittleEndian::write_f32(&mut buf, e);
	/// assert_eq!(e, LittleEndian::read_f32(&buf));
	/// ```
	#[inline]
	fn write_f32(
		buf: &mut [u8],
		n: f32,
	) {
		Self::write_u32(buf, n.to_bits())
	}

	/// Writes a IEEE754 double-precision (8 bytes) floating point number.
	///
	/// # Panics
	///
	/// Panics when `buf.len() < 8`.
	///
	/// # Examples
	///
	/// Write and read `f64` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let phi = 1.6180339887;
	/// let mut buf = [0; 8];
	/// LittleEndian::write_f64(&mut buf, phi);
	/// assert_eq!(phi, LittleEndian::read_f64(&buf));
	/// ```
	#[inline]
	fn write_f64(
		buf: &mut [u8],
		n: f64,
	) {
		Self::write_u64(buf, n.to_bits())
	}

	/// Reads unsigned 16 bit integers from `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `src.len() != 2*dst.len()`.
	///
	/// # Examples
	///
	/// Write and read `u16` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 8];
	/// let numbers_given = [1, 2, 0xf00f, 0xffee];
	/// LittleEndian::write_u16_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0; 4];
	/// LittleEndian::read_u16_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	fn read_u16_into(
		src: &[u8],
		dst: &mut [u16],
	);

	/// Reads unsigned 32 bit integers from `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `src.len() != 4*dst.len()`.
	///
	/// # Examples
	///
	/// Write and read `u32` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 16];
	/// let numbers_given = [1, 2, 0xf00f, 0xffee];
	/// LittleEndian::write_u32_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0; 4];
	/// LittleEndian::read_u32_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	fn read_u32_into(
		src: &[u8],
		dst: &mut [u32],
	);

	/// Reads unsigned 64 bit integers from `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `src.len() != 8*dst.len()`.
	///
	/// # Examples
	///
	/// Write and read `u64` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 32];
	/// let numbers_given = [1, 2, 0xf00f, 0xffee];
	/// LittleEndian::write_u64_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0; 4];
	/// LittleEndian::read_u64_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	fn read_u64_into(
		src: &[u8],
		dst: &mut [u64],
	);

	/// Reads unsigned 128 bit integers from `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `src.len() != 16*dst.len()`.
	///
	/// # Examples
	///
	/// Write and read `u128` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 64];
	/// let numbers_given = [1, 2, 0xf00f, 0xffee];
	/// LittleEndian::write_u128_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0; 4];
	/// LittleEndian::read_u128_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	fn read_u128_into(
		src: &[u8],
		dst: &mut [u128],
	);

	/// Reads signed 16 bit integers from `src` to `dst`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() != 2*dst.len()`.
	///
	/// # Examples
	///
	/// Write and read `i16` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 8];
	/// let numbers_given = [1, 2, 0x0f, 0xee];
	/// LittleEndian::write_i16_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0; 4];
	/// LittleEndian::read_i16_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	#[inline]
	fn read_i16_into(
		src: &[u8],
		dst: &mut [i16],
	) {
		let dst = unsafe { slice::from_raw_parts_mut(dst.as_mut_ptr() as *mut u16, dst.len()) };
		Self::read_u16_into(src, dst)
	}

	/// Reads signed 32 bit integers from `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `src.len() != 4*dst.len()`.
	///
	/// # Examples
	///
	/// Write and read `i32` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 16];
	/// let numbers_given = [1, 2, 0xf00f, 0xffee];
	/// LittleEndian::write_i32_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0; 4];
	/// LittleEndian::read_i32_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	#[inline]
	fn read_i32_into(
		src: &[u8],
		dst: &mut [i32],
	) {
		let dst = unsafe { slice::from_raw_parts_mut(dst.as_mut_ptr() as *mut u32, dst.len()) };
		Self::read_u32_into(src, dst);
	}

	/// Reads signed 64 bit integers from `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `src.len() != 8*dst.len()`.
	///
	/// # Examples
	///
	/// Write and read `i64` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 32];
	/// let numbers_given = [1, 2, 0xf00f, 0xffee];
	/// LittleEndian::write_i64_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0; 4];
	/// LittleEndian::read_i64_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	#[inline]
	fn read_i64_into(
		src: &[u8],
		dst: &mut [i64],
	) {
		let dst = unsafe { slice::from_raw_parts_mut(dst.as_mut_ptr() as *mut u64, dst.len()) };
		Self::read_u64_into(src, dst);
	}

	/// Reads signed 128 bit integers from `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `src.len() != 16*dst.len()`.
	///
	/// # Examples
	///
	/// Write and read `i128` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 64];
	/// let numbers_given = [1, 2, 0xf00f, 0xffee];
	/// LittleEndian::write_i128_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0; 4];
	/// LittleEndian::read_i128_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	#[inline]
	fn read_i128_into(
		src: &[u8],
		dst: &mut [i128],
	) {
		let dst = unsafe { slice::from_raw_parts_mut(dst.as_mut_ptr() as *mut u128, dst.len()) };
		Self::read_u128_into(src, dst);
	}

	/// Reads IEEE754 single-precision (4 bytes) floating point numbers from
	/// `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `src.len() != 4*dst.len()`.
	///
	/// # Examples
	///
	/// Write and read `f32` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 16];
	/// let numbers_given = [1.0, 2.0, 31.312e31, -11.32e19];
	/// LittleEndian::write_f32_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0.0; 4];
	/// LittleEndian::read_f32_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	#[inline]
	fn read_f32_into(
		src: &[u8],
		dst: &mut [f32],
	) {
		let dst = unsafe {
			const _: () = assert!(align_of::<u32>() <= align_of::<f32>());
			slice::from_raw_parts_mut(dst.as_mut_ptr() as *mut u32, dst.len())
		};
		Self::read_u32_into(src, dst);
	}

	/// **DEPRECATED**.
	///
	/// This method is deprecated. Use `read_f32_into` instead.
	/// Reads IEEE754 single-precision (4 bytes) floating point numbers from
	/// `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `src.len() != 4*dst.len()`.
	///
	/// # Examples
	///
	/// Write and read `f32` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 16];
	/// let numbers_given = [1.0, 2.0, 31.312e31, -11.32e19];
	/// LittleEndian::write_f32_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0.0; 4];
	/// LittleEndian::read_f32_into_unchecked(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	#[inline]
	#[deprecated(since = "1.3.0", note = "please use `read_f32_into` instead")]
	fn read_f32_into_unchecked(
		src: &[u8],
		dst: &mut [f32],
	) {
		Self::read_f32_into(src, dst);
	}

	/// Reads IEEE754 single-precision (4 bytes) floating point numbers from
	/// `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `src.len() != 8*dst.len()`.
	///
	/// # Examples
	///
	/// Write and read `f64` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 32];
	/// let numbers_given = [1.0, 2.0, 31.312e211, -11.32e91];
	/// LittleEndian::write_f64_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0.0; 4];
	/// LittleEndian::read_f64_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	#[inline]
	fn read_f64_into(
		src: &[u8],
		dst: &mut [f64],
	) {
		let dst = unsafe {
			const _: () = assert!(align_of::<u64>() <= align_of::<f64>());
			slice::from_raw_parts_mut(dst.as_mut_ptr() as *mut u64, dst.len())
		};
		Self::read_u64_into(src, dst);
	}

	/// **DEPRECATED**.
	///
	/// This method is deprecated. Use `read_f64_into` instead.
	///
	/// Reads IEEE754 single-precision (4 bytes) floating point numbers from
	/// `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `src.len() != 8*dst.len()`.
	///
	/// # Examples
	///
	/// Write and read `f64` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 32];
	/// let numbers_given = [1.0, 2.0, 31.312e211, -11.32e91];
	/// LittleEndian::write_f64_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0.0; 4];
	/// LittleEndian::read_f64_into_unchecked(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	#[inline]
	#[deprecated(since = "1.3.0", note = "please use `read_f64_into` instead")]
	fn read_f64_into_unchecked(
		src: &[u8],
		dst: &mut [f64],
	) {
		Self::read_f64_into(src, dst);
	}

	/// Writes unsigned 16 bit integers from `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `dst.len() != 2*src.len()`.
	///
	/// # Examples
	///
	/// Write and read `u16` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 8];
	/// let numbers_given = [1, 2, 0xf00f, 0xffee];
	/// LittleEndian::write_u16_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0; 4];
	/// LittleEndian::read_u16_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	fn write_u16_into(
		src: &[u16],
		dst: &mut [u8],
	);

	/// Writes unsigned 32 bit integers from `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `dst.len() != 4*src.len()`.
	///
	/// # Examples
	///
	/// Write and read `u32` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 16];
	/// let numbers_given = [1, 2, 0xf00f, 0xffee];
	/// LittleEndian::write_u32_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0; 4];
	/// LittleEndian::read_u32_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	fn write_u32_into(
		src: &[u32],
		dst: &mut [u8],
	);

	/// Writes unsigned 64 bit integers from `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `dst.len() != 8*src.len()`.
	///
	/// # Examples
	///
	/// Write and read `u64` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 32];
	/// let numbers_given = [1, 2, 0xf00f, 0xffee];
	/// LittleEndian::write_u64_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0; 4];
	/// LittleEndian::read_u64_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	fn write_u64_into(
		src: &[u64],
		dst: &mut [u8],
	);

	/// Writes unsigned 128 bit integers from `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `dst.len() != 16*src.len()`.
	///
	/// # Examples
	///
	/// Write and read `u128` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 64];
	/// let numbers_given = [1, 2, 0xf00f, 0xffee];
	/// LittleEndian::write_u128_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0; 4];
	/// LittleEndian::read_u128_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	fn write_u128_into(
		src: &[u128],
		dst: &mut [u8],
	);

	/// Writes signed 8 bit integers from `src` into `dst`.
	///
	/// Note that since each `i8` is a single byte, no byte order conversions
	/// are used. This method is included because it provides a safe, simple
	/// way for the caller to write from a `&[i8]` buffer. (Without this
	/// method, the caller would have to either use `unsafe` code or convert
	/// each byte to `u8` individually.)
	///
	/// # Panics
	///
	/// Panics when `buf.len() != src.len()`.
	///
	/// # Examples
	///
	/// Write and read `i8` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};
	///
	/// let mut bytes = [0; 4];
	/// let numbers_given = [1, 2, 0xf, 0xe];
	/// LittleEndian::write_i8_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0; 4];
	/// bytes.as_ref().read_i8_into(&mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	fn write_i8_into(
		src: &[i8],
		dst: &mut [u8],
	) {
		let src = unsafe { slice::from_raw_parts(src.as_ptr() as *const u8, src.len()) };
		dst.copy_from_slice(src);
	}

	/// Writes signed 16 bit integers from `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `buf.len() != 2*src.len()`.
	///
	/// # Examples
	///
	/// Write and read `i16` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 8];
	/// let numbers_given = [1, 2, 0x0f, 0xee];
	/// LittleEndian::write_i16_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0; 4];
	/// LittleEndian::read_i16_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	fn write_i16_into(
		src: &[i16],
		dst: &mut [u8],
	) {
		let src = unsafe { slice::from_raw_parts(src.as_ptr() as *const u16, src.len()) };
		Self::write_u16_into(src, dst);
	}

	/// Writes signed 32 bit integers from `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `dst.len() != 4*src.len()`.
	///
	/// # Examples
	///
	/// Write and read `i32` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 16];
	/// let numbers_given = [1, 2, 0xf00f, 0xffee];
	/// LittleEndian::write_i32_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0; 4];
	/// LittleEndian::read_i32_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	fn write_i32_into(
		src: &[i32],
		dst: &mut [u8],
	) {
		let src = unsafe { slice::from_raw_parts(src.as_ptr() as *const u32, src.len()) };
		Self::write_u32_into(src, dst);
	}

	/// Writes signed 64 bit integers from `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `dst.len() != 8*src.len()`.
	///
	/// # Examples
	///
	/// Write and read `i64` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 32];
	/// let numbers_given = [1, 2, 0xf00f, 0xffee];
	/// LittleEndian::write_i64_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0; 4];
	/// LittleEndian::read_i64_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	fn write_i64_into(
		src: &[i64],
		dst: &mut [u8],
	) {
		let src = unsafe { slice::from_raw_parts(src.as_ptr() as *const u64, src.len()) };
		Self::write_u64_into(src, dst);
	}

	/// Writes signed 128 bit integers from `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `dst.len() != 16*src.len()`.
	///
	/// # Examples
	///
	/// Write and read `i128` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 64];
	/// let numbers_given = [1, 2, 0xf00f, 0xffee];
	/// LittleEndian::write_i128_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0; 4];
	/// LittleEndian::read_i128_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	fn write_i128_into(
		src: &[i128],
		dst: &mut [u8],
	) {
		let src = unsafe { slice::from_raw_parts(src.as_ptr() as *const u128, src.len()) };
		Self::write_u128_into(src, dst);
	}

	/// Writes IEEE754 single-precision (4 bytes) floating point numbers from
	/// `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `src.len() != 4*dst.len()`.
	///
	/// # Examples
	///
	/// Write and read `f32` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 16];
	/// let numbers_given = [1.0, 2.0, 31.312e31, -11.32e19];
	/// LittleEndian::write_f32_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0.0; 4];
	/// LittleEndian::read_f32_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	fn write_f32_into(
		src: &[f32],
		dst: &mut [u8],
	) {
		let src = unsafe { slice::from_raw_parts(src.as_ptr() as *const u32, src.len()) };
		Self::write_u32_into(src, dst);
	}

	/// Writes IEEE754 double-precision (8 bytes) floating point numbers from
	/// `src` into `dst`.
	///
	/// # Panics
	///
	/// Panics when `src.len() != 8*dst.len()`.
	///
	/// # Examples
	///
	/// Write and read `f64` numbers in little endian order:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, LittleEndian};
	///
	/// let mut bytes = [0; 32];
	/// let numbers_given = [1.0, 2.0, 31.312e211, -11.32e91];
	/// LittleEndian::write_f64_into(&numbers_given, &mut bytes);
	///
	/// let mut numbers_got = [0.0; 4];
	/// LittleEndian::read_f64_into(&bytes, &mut numbers_got);
	/// assert_eq!(numbers_given, numbers_got);
	/// ```
	fn write_f64_into(
		src: &[f64],
		dst: &mut [u8],
	) {
		let src = unsafe { slice::from_raw_parts(src.as_ptr() as *const u64, src.len()) };
		Self::write_u64_into(src, dst);
	}

	/// Converts the given slice of unsigned 16 bit integers to a particular
	/// endianness.
	///
	/// If the endianness matches the endianness of the host platform, then
	/// this is a no-op.
	///
	/// # Examples
	///
	/// Convert the host platform's endianness to big-endian:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, BigEndian};
	///
	/// let mut numbers = [5, 65000];
	/// BigEndian::from_slice_u16(&mut numbers);
	/// assert_eq!(numbers, [5u16.to_be(), 65000u16.to_be()]);
	/// ```
	fn from_slice_u16(numbers: &mut [u16]);

	/// Converts the given slice of unsigned 32 bit integers to a particular
	/// endianness.
	///
	/// If the endianness matches the endianness of the host platform, then
	/// this is a no-op.
	///
	/// # Examples
	///
	/// Convert the host platform's endianness to big-endian:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, BigEndian};
	///
	/// let mut numbers = [5, 65000];
	/// BigEndian::from_slice_u32(&mut numbers);
	/// assert_eq!(numbers, [5u32.to_be(), 65000u32.to_be()]);
	/// ```
	fn from_slice_u32(numbers: &mut [u32]);

	/// Converts the given slice of unsigned 64 bit integers to a particular
	/// endianness.
	///
	/// If the endianness matches the endianness of the host platform, then
	/// this is a no-op.
	///
	/// # Examples
	///
	/// Convert the host platform's endianness to big-endian:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, BigEndian};
	///
	/// let mut numbers = [5, 65000];
	/// BigEndian::from_slice_u64(&mut numbers);
	/// assert_eq!(numbers, [5u64.to_be(), 65000u64.to_be()]);
	/// ```
	fn from_slice_u64(numbers: &mut [u64]);

	/// Converts the given slice of unsigned 128 bit integers to a particular
	/// endianness.
	///
	/// If the endianness matches the endianness of the host platform, then
	/// this is a no-op.
	///
	/// # Examples
	///
	/// Convert the host platform's endianness to big-endian:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, BigEndian};
	///
	/// let mut numbers = [5, 65000];
	/// BigEndian::from_slice_u128(&mut numbers);
	/// assert_eq!(numbers, [5u128.to_be(), 65000u128.to_be()]);
	/// ```
	fn from_slice_u128(numbers: &mut [u128]);

	/// Converts the given slice of signed 16 bit integers to a particular
	/// endianness.
	///
	/// If the endianness matches the endianness of the host platform, then
	/// this is a no-op.
	///
	/// # Examples
	///
	/// Convert the host platform's endianness to big-endian:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, BigEndian};
	///
	/// let mut numbers = [5, 6500];
	/// BigEndian::from_slice_i16(&mut numbers);
	/// assert_eq!(numbers, [5i16.to_be(), 6500i16.to_be()]);
	/// ```
	#[inline]
	fn from_slice_i16(src: &mut [i16]) {
		let src = unsafe { slice::from_raw_parts_mut(src.as_mut_ptr() as *mut u16, src.len()) };
		Self::from_slice_u16(src);
	}

	/// Converts the given slice of signed 32 bit integers to a particular
	/// endianness.
	///
	/// If the endianness matches the endianness of the host platform, then
	/// this is a no-op.
	///
	/// # Examples
	///
	/// Convert the host platform's endianness to big-endian:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, BigEndian};
	///
	/// let mut numbers = [5, 65000];
	/// BigEndian::from_slice_i32(&mut numbers);
	/// assert_eq!(numbers, [5i32.to_be(), 65000i32.to_be()]);
	/// ```
	#[inline]
	fn from_slice_i32(src: &mut [i32]) {
		let src = unsafe { slice::from_raw_parts_mut(src.as_mut_ptr() as *mut u32, src.len()) };
		Self::from_slice_u32(src);
	}

	/// Converts the given slice of signed 64 bit integers to a particular
	/// endianness.
	///
	/// If the endianness matches the endianness of the host platform, then
	/// this is a no-op.
	///
	/// # Examples
	///
	/// Convert the host platform's endianness to big-endian:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, BigEndian};
	///
	/// let mut numbers = [5, 65000];
	/// BigEndian::from_slice_i64(&mut numbers);
	/// assert_eq!(numbers, [5i64.to_be(), 65000i64.to_be()]);
	/// ```
	#[inline]
	fn from_slice_i64(src: &mut [i64]) {
		let src = unsafe { slice::from_raw_parts_mut(src.as_mut_ptr() as *mut u64, src.len()) };
		Self::from_slice_u64(src);
	}

	/// Converts the given slice of signed 128 bit integers to a particular
	/// endianness.
	///
	/// If the endianness matches the endianness of the host platform, then
	/// this is a no-op.
	///
	/// # Examples
	///
	/// Convert the host platform's endianness to big-endian:
	///
	/// ```rust
	/// use byteorder::{ByteOrder, BigEndian};
	///
	/// let mut numbers = [5, 65000];
	/// BigEndian::from_slice_i128(&mut numbers);
	/// assert_eq!(numbers, [5i128.to_be(), 65000i128.to_be()]);
	/// ```
	#[inline]
	fn from_slice_i128(src: &mut [i128]) {
		let src = unsafe { slice::from_raw_parts_mut(src.as_mut_ptr() as *mut u128, src.len()) };
		Self::from_slice_u128(src);
	}

	/// Converts the given slice of IEEE754 single-precision (4 bytes) floating
	/// point numbers to a particular endianness.
	///
	/// If the endianness matches the endianness of the host platform, then
	/// this is a no-op.
	fn from_slice_f32(numbers: &mut [f32]);

	/// Converts the given slice of IEEE754 double-precision (8 bytes) floating
	/// point numbers to a particular endianness.
	///
	/// If the endianness matches the endianness of the host platform, then
	/// this is a no-op.
	fn from_slice_f64(numbers: &mut [f64]);
}

/// Defines big-endian serialization.
///
/// Note that this type has no value constructor. It is used purely at the
/// type level.
///
/// # Examples
///
/// Write and read `u32` numbers in big endian order:
///
/// ```rust
/// use byteorder::{ByteOrder, BigEndian};
///
/// let mut buf = [0; 4];
/// BigEndian::write_u32(&mut buf, 1_000_000);
/// assert_eq!(1_000_000, BigEndian::read_u32(&buf));
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BigEndian {}

impl Default for BigEndian {
	fn default() -> BigEndian {
		panic!("BigEndian default")
	}
}

/// A type alias for [`BigEndian`].
///
/// [`BigEndian`]: enum.BigEndian.html
pub type BE = BigEndian;

/// Defines little-endian serialization.
///
/// Note that this type has no value constructor. It is used purely at the
/// type level.
///
/// # Examples
///
/// Write and read `u32` numbers in little endian order:
///
/// ```rust
/// use byteorder::{ByteOrder, LittleEndian};
///
/// let mut buf = [0; 4];
/// LittleEndian::write_u32(&mut buf, 1_000_000);
/// assert_eq!(1_000_000, LittleEndian::read_u32(&buf));
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LittleEndian {}

impl Default for LittleEndian {
	fn default() -> LittleEndian {
		panic!("LittleEndian default")
	}
}

/// A type alias for [`LittleEndian`].
///
/// [`LittleEndian`]: enum.LittleEndian.html
pub type LE = LittleEndian;

/// Defines network byte order serialization.
///
/// Network byte order is defined by [RFC 1700][1] to be big-endian, and is
/// referred to in several protocol specifications.  This type is an alias of
/// [`BigEndian`].
///
/// [1]: https://tools.ietf.org/html/rfc1700
///
/// Note that this type has no value constructor. It is used purely at the
/// type level.
///
/// # Examples
///
/// Write and read `i16` numbers in big endian order:
///
/// ```rust
/// use byteorder::{ByteOrder, NetworkEndian, BigEndian};
///
/// let mut buf = [0; 2];
/// BigEndian::write_i16(&mut buf, -5_000);
/// assert_eq!(-5_000, NetworkEndian::read_i16(&buf));
/// ```
///
/// [`BigEndian`]: enum.BigEndian.html
pub type NetworkEndian = BigEndian;

/// Defines system native-endian serialization.
///
/// Note that this type has no value constructor. It is used purely at the
/// type level.
///
/// On this platform, this is an alias for [`LittleEndian`].
///
/// [`LittleEndian`]: enum.LittleEndian.html
#[cfg(target_endian = "little")]
pub type NativeEndian = LittleEndian;

/// Defines system native-endian serialization.
///
/// Note that this type has no value constructor. It is used purely at the
/// type level.
///
/// On this platform, this is an alias for [`BigEndian`].
///
/// [`BigEndian`]: enum.BigEndian.html
#[cfg(target_endian = "big")]
pub type NativeEndian = BigEndian;

/// Copies a &[u8] $src into a &mut [$ty] $dst for the endianness given by
/// $from_bytes (must be either from_be_bytes or from_le_bytes).
///
/// Panics if $src.len() != $dst.len() * size_of::<$ty>().
macro_rules! read_slice {
	($src:expr, $dst:expr, $ty:ty, $from_bytes:ident) => {{
		const SIZE: usize = core::mem::size_of::<$ty>();
		// Check types:
		let src: &[u8] = $src;
		let dst: &mut [$ty] = $dst;
		assert_eq!(src.len(), dst.len() * SIZE);
		for (src, dst) in src.chunks_exact(SIZE).zip(dst.iter_mut()) {
			*dst = <$ty>::$from_bytes(src.try_into().unwrap());
		}
	}};
}

/// Copies a &[$ty] $src into a &mut [u8] $dst for the endianness given by
/// $from_bytes (must be either from_be_bytes or from_le_bytes).
///
/// Panics if $src.len() * size_of::<$ty>() != $dst.len().
macro_rules! write_slice {
	($src:expr, $dst:expr, $ty:ty, $to_bytes:ident) => {{
		const SIZE: usize = core::mem::size_of::<$ty>();
		// Check types:
		let src: &[$ty] = $src;
		let dst: &mut [u8] = $dst;
		assert_eq!(src.len() * SIZE, dst.len());
		for (src, dst) in src.iter().zip(dst.chunks_exact_mut(SIZE)) {
			dst.copy_from_slice(&src.$to_bytes());
		}
	}};
}

impl ByteOrder for BigEndian {
	#[inline]
	fn read_u16(buf: &[u8]) -> u16 {
		u16::from_be_bytes(buf[..2].try_into().unwrap())
	}

	#[inline]
	fn read_u32(buf: &[u8]) -> u32 {
		u32::from_be_bytes(buf[..4].try_into().unwrap())
	}

	#[inline]
	fn read_u64(buf: &[u8]) -> u64 {
		u64::from_be_bytes(buf[..8].try_into().unwrap())
	}

	#[inline]
	fn read_u128(buf: &[u8]) -> u128 {
		u128::from_be_bytes(buf[..16].try_into().unwrap())
	}

	#[inline]
	fn read_uint(
		buf: &[u8],
		nbytes: usize,
	) -> u64 {
		let mut out = [0; 8];
		assert!(1 <= nbytes && nbytes <= out.len() && nbytes <= buf.len());
		let start = out.len() - nbytes;
		out[start..].copy_from_slice(&buf[..nbytes]);
		u64::from_be_bytes(out)
	}

	#[inline]
	fn read_uint128(
		buf: &[u8],
		nbytes: usize,
	) -> u128 {
		let mut out = [0; 16];
		assert!(1 <= nbytes && nbytes <= out.len() && nbytes <= buf.len());
		let start = out.len() - nbytes;
		out[start..].copy_from_slice(&buf[..nbytes]);
		u128::from_be_bytes(out)
	}

	#[inline]
	fn write_u16(
		buf: &mut [u8],
		n: u16,
	) {
		buf[..2].copy_from_slice(&n.to_be_bytes());
	}

	#[inline]
	fn write_u32(
		buf: &mut [u8],
		n: u32,
	) {
		buf[..4].copy_from_slice(&n.to_be_bytes());
	}

	#[inline]
	fn write_u64(
		buf: &mut [u8],
		n: u64,
	) {
		buf[..8].copy_from_slice(&n.to_be_bytes());
	}

	#[inline]
	fn write_u128(
		buf: &mut [u8],
		n: u128,
	) {
		buf[..16].copy_from_slice(&n.to_be_bytes());
	}

	#[inline]
	fn write_uint(
		buf: &mut [u8],
		n: u64,
		nbytes: usize,
	) {
		assert!(pack_size(n) <= nbytes && nbytes <= 8);
		assert!(nbytes <= buf.len());
		unsafe {
			let bytes = *(&n.to_be() as *const u64 as *const [u8; 8]);
			copy_nonoverlapping(
				bytes.as_ptr().offset((8 - nbytes) as isize),
				buf.as_mut_ptr(),
				nbytes,
			);
		}
	}

	#[inline]
	fn write_uint128(
		buf: &mut [u8],
		n: u128,
		nbytes: usize,
	) {
		assert!(pack_size128(n) <= nbytes && nbytes <= 16);
		assert!(nbytes <= buf.len());
		unsafe {
			let bytes = *(&n.to_be() as *const u128 as *const [u8; 16]);
			copy_nonoverlapping(
				bytes.as_ptr().offset((16 - nbytes) as isize),
				buf.as_mut_ptr(),
				nbytes,
			);
		}
	}

	#[inline]
	fn read_u16_into(
		src: &[u8],
		dst: &mut [u16],
	) {
		read_slice!(src, dst, u16, from_be_bytes);
	}

	#[inline]
	fn read_u32_into(
		src: &[u8],
		dst: &mut [u32],
	) {
		read_slice!(src, dst, u32, from_be_bytes);
	}

	#[inline]
	fn read_u64_into(
		src: &[u8],
		dst: &mut [u64],
	) {
		read_slice!(src, dst, u64, from_be_bytes);
	}

	#[inline]
	fn read_u128_into(
		src: &[u8],
		dst: &mut [u128],
	) {
		read_slice!(src, dst, u128, from_be_bytes);
	}

	#[inline]
	fn write_u16_into(
		src: &[u16],
		dst: &mut [u8],
	) {
		write_slice!(src, dst, u16, to_be_bytes);
	}

	#[inline]
	fn write_u32_into(
		src: &[u32],
		dst: &mut [u8],
	) {
		write_slice!(src, dst, u32, to_be_bytes);
	}

	#[inline]
	fn write_u64_into(
		src: &[u64],
		dst: &mut [u8],
	) {
		write_slice!(src, dst, u64, to_be_bytes);
	}

	#[inline]
	fn write_u128_into(
		src: &[u128],
		dst: &mut [u8],
	) {
		write_slice!(src, dst, u128, to_be_bytes);
	}

	#[inline]
	fn from_slice_u16(numbers: &mut [u16]) {
		if cfg!(target_endian = "little") {
			for n in numbers {
				*n = n.to_be();
			}
		}
	}

	#[inline]
	fn from_slice_u32(numbers: &mut [u32]) {
		if cfg!(target_endian = "little") {
			for n in numbers {
				*n = n.to_be();
			}
		}
	}

	#[inline]
	fn from_slice_u64(numbers: &mut [u64]) {
		if cfg!(target_endian = "little") {
			for n in numbers {
				*n = n.to_be();
			}
		}
	}

	#[inline]
	fn from_slice_u128(numbers: &mut [u128]) {
		if cfg!(target_endian = "little") {
			for n in numbers {
				*n = n.to_be();
			}
		}
	}

	#[inline]
	fn from_slice_f32(numbers: &mut [f32]) {
		if cfg!(target_endian = "little") {
			for n in numbers {
				unsafe {
					let int = *(n as *const f32 as *const u32);
					*n = *(&int.to_be() as *const u32 as *const f32);
				}
			}
		}
	}

	#[inline]
	fn from_slice_f64(numbers: &mut [f64]) {
		if cfg!(target_endian = "little") {
			for n in numbers {
				unsafe {
					let int = *(n as *const f64 as *const u64);
					*n = *(&int.to_be() as *const u64 as *const f64);
				}
			}
		}
	}
}

impl ByteOrder for LittleEndian {
	#[inline]
	fn read_u16(buf: &[u8]) -> u16 {
		u16::from_le_bytes(buf[..2].try_into().unwrap())
	}

	#[inline]
	fn read_u32(buf: &[u8]) -> u32 {
		u32::from_le_bytes(buf[..4].try_into().unwrap())
	}

	#[inline]
	fn read_u64(buf: &[u8]) -> u64 {
		u64::from_le_bytes(buf[..8].try_into().unwrap())
	}

	#[inline]
	fn read_u128(buf: &[u8]) -> u128 {
		u128::from_le_bytes(buf[..16].try_into().unwrap())
	}

	#[inline]
	fn read_uint(
		buf: &[u8],
		nbytes: usize,
	) -> u64 {
		let mut out = [0; 8];
		assert!(1 <= nbytes && nbytes <= out.len() && nbytes <= buf.len());
		out[..nbytes].copy_from_slice(&buf[..nbytes]);
		u64::from_le_bytes(out)
	}

	#[inline]
	fn read_uint128(
		buf: &[u8],
		nbytes: usize,
	) -> u128 {
		let mut out = [0; 16];
		assert!(1 <= nbytes && nbytes <= out.len() && nbytes <= buf.len());
		out[..nbytes].copy_from_slice(&buf[..nbytes]);
		u128::from_le_bytes(out)
	}

	#[inline]
	fn write_u16(
		buf: &mut [u8],
		n: u16,
	) {
		buf[..2].copy_from_slice(&n.to_le_bytes());
	}

	#[inline]
	fn write_u32(
		buf: &mut [u8],
		n: u32,
	) {
		buf[..4].copy_from_slice(&n.to_le_bytes());
	}

	#[inline]
	fn write_u64(
		buf: &mut [u8],
		n: u64,
	) {
		buf[..8].copy_from_slice(&n.to_le_bytes());
	}

	#[inline]
	fn write_u128(
		buf: &mut [u8],
		n: u128,
	) {
		buf[..16].copy_from_slice(&n.to_le_bytes());
	}

	#[inline]
	fn write_uint(
		buf: &mut [u8],
		n: u64,
		nbytes: usize,
	) {
		assert!(pack_size(n as u64) <= nbytes && nbytes <= 8);
		assert!(nbytes <= buf.len());
		unsafe {
			let bytes = *(&n.to_le() as *const u64 as *const [u8; 8]);
			copy_nonoverlapping(bytes.as_ptr(), buf.as_mut_ptr(), nbytes);
		}
	}

	#[inline]
	fn write_uint128(
		buf: &mut [u8],
		n: u128,
		nbytes: usize,
	) {
		assert!(pack_size128(n as u128) <= nbytes && nbytes <= 16);
		assert!(nbytes <= buf.len());
		unsafe {
			let bytes = *(&n.to_le() as *const u128 as *const [u8; 16]);
			copy_nonoverlapping(bytes.as_ptr(), buf.as_mut_ptr(), nbytes);
		}
	}

	#[inline]
	fn read_u16_into(
		src: &[u8],
		dst: &mut [u16],
	) {
		read_slice!(src, dst, u16, from_le_bytes);
	}

	#[inline]
	fn read_u32_into(
		src: &[u8],
		dst: &mut [u32],
	) {
		read_slice!(src, dst, u32, from_le_bytes);
	}

	#[inline]
	fn read_u64_into(
		src: &[u8],
		dst: &mut [u64],
	) {
		read_slice!(src, dst, u64, from_le_bytes);
	}

	#[inline]
	fn read_u128_into(
		src: &[u8],
		dst: &mut [u128],
	) {
		read_slice!(src, dst, u128, from_le_bytes);
	}

	#[inline]
	fn write_u16_into(
		src: &[u16],
		dst: &mut [u8],
	) {
		write_slice!(src, dst, u16, to_le_bytes);
	}

	#[inline]
	fn write_u32_into(
		src: &[u32],
		dst: &mut [u8],
	) {
		write_slice!(src, dst, u32, to_le_bytes);
	}

	#[inline]
	fn write_u64_into(
		src: &[u64],
		dst: &mut [u8],
	) {
		write_slice!(src, dst, u64, to_le_bytes);
	}

	#[inline]
	fn write_u128_into(
		src: &[u128],
		dst: &mut [u8],
	) {
		write_slice!(src, dst, u128, to_le_bytes);
	}

	#[inline]
	fn from_slice_u16(numbers: &mut [u16]) {
		if cfg!(target_endian = "big") {
			for n in numbers {
				*n = n.to_le();
			}
		}
	}

	#[inline]
	fn from_slice_u32(numbers: &mut [u32]) {
		if cfg!(target_endian = "big") {
			for n in numbers {
				*n = n.to_le();
			}
		}
	}

	#[inline]
	fn from_slice_u64(numbers: &mut [u64]) {
		if cfg!(target_endian = "big") {
			for n in numbers {
				*n = n.to_le();
			}
		}
	}

	#[inline]
	fn from_slice_u128(numbers: &mut [u128]) {
		if cfg!(target_endian = "big") {
			for n in numbers {
				*n = n.to_le();
			}
		}
	}

	#[inline]
	fn from_slice_f32(numbers: &mut [f32]) {
		if cfg!(target_endian = "big") {
			for n in numbers {
				unsafe {
					let int = *(n as *const f32 as *const u32);
					*n = *(&int.to_le() as *const u32 as *const f32);
				}
			}
		}
	}

	#[inline]
	fn from_slice_f64(numbers: &mut [f64]) {
		if cfg!(target_endian = "big") {
			for n in numbers {
				unsafe {
					let int = *(n as *const f64 as *const u64);
					*n = *(&int.to_le() as *const u64 as *const f64);
				}
			}
		}
	}
}
