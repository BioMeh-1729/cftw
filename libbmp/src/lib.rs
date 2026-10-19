#[allow(unused)]
mod byteorder;
#[allow(unused)]
mod bmp {

	//! A small library for reading and writing BMP images.
	//!
	//! The library supports uncompressed BMP Version 3 images.
	//! The different decoding and encoding schemes is shown in the table below.
	//!
	//! |Scheme | Decoding | Encoding | Compression |
	//! |-------|----------|----------|-------------|
	//! | 24 bpp| ✓        | ✓        | No          |
	//! | 8 bpp | ✓        | ✗        | No          |
	//! | 4 bpp | ✓        | ✗        | No          |
	//! | 1 bpp | ✓        | ✗        | No          |
	//!
	//! # Example
	//!
	//! ```
	//! #[macro_use]
	//! extern crate bmp;
	//! use bmp::{Image, Pixel};
	//!
	//! fn main() {
	//!     let mut img = Image::new(256, 256);
	//!
	//!     for (x, y) in img.coordinates() {
	//!         img.set_pixel(x, y, px!(x, y, 200));
	//!     }
	//!     let _ = img.save("img.bmp");
	//! }
	//! ```
	//!

	use std::convert::AsRef;
	use std::fmt;
	use std::fs;
	use std::io;
	use std::io::{Cursor, Read, Write};
	use std::iter::Iterator;
	use std::path::Path;

	// Expose decoder's public types, structs, and enums
	pub use decoder::{BmpError, BmpErrorKind, BmpResult};

	/// Macro to generate a `Pixel` from `r`, `g` and `b` values.
	#[macro_export]
	macro_rules! px {
		($r:expr, $g:expr, $b:expr) => {
			Pixel {
				r: $r as u8,
				g: $g as u8,
				b: $b as u8,
			}
		};
	}

	macro_rules! file_size {
		($bpp:expr, $width:expr, $height:expr) => {{
			let header_size = 2 + 12 + 40;
			// find row size in bytes, round up to 4 bytes (padding)
			let row_size = (($bpp as f32 * $width as f32 + 31.0) / 32.0).floor() as u32 * 4;
			(header_size as u32, $height as u32 * row_size)
		}};
	}

	mod decoder {
		use super::super::byteorder;

		use byteorder::{LittleEndian, ReadBytesExt};

		use std::convert::{AsRef, From};
		use std::error::Error;
		use std::fmt;
		use std::io::{self, Cursor, Read, Seek, SeekFrom};

		// The BmpHeader always has a size of 14 bytes
		const BMP_HEADER_SIZE: u64 = 14;

		// Import structs/functions defined in lib.rs
		use self::BmpErrorKind::*;
		use super::*;

		/// A result type, either containing an `Image` or a `BmpError`.
		pub type BmpResult<T> = Result<T, BmpError>;

		/// The error type returned if the decoding of an image from disk fails.
		#[derive(Debug)]
		pub struct BmpError {
			pub kind: BmpErrorKind,
			pub details: String,
		}

		impl BmpError {
			fn new<T: AsRef<str>>(
				kind: BmpErrorKind,
				details: T,
			) -> BmpError {
				BmpError {
					kind,
					details: String::from(details.as_ref()),
				}
			}
		}

		impl fmt::Display for BmpError {
			fn fmt(
				&self,
				fmt: &mut fmt::Formatter,
			) -> fmt::Result {
				match self.kind {
					BmpIoError(ref error) => error.fmt(fmt),
					ref e => {
						let kind_desc: &str = e.as_ref();
						write!(fmt, "{}: {}", kind_desc, self.details)
					}
				}
			}
		}

		impl From<io::Error> for BmpError {
			fn from(err: io::Error) -> BmpError {
				BmpError::new(BmpIoError(err), "Io Error")
			}
		}

		impl Error for BmpError {}

		/// The different kinds of possible BMP errors.
		#[derive(Debug)]
		pub enum BmpErrorKind {
			WrongMagicNumbers,
			UnsupportedBitsPerPixel,
			UnsupportedCompressionType,
			UnsupportedBmpVersion,
			UnsupportedHeader,
			BmpIoError(io::Error),
		}

		impl AsRef<str> for BmpErrorKind {
			fn as_ref(&self) -> &str {
				match *self {
					WrongMagicNumbers => "Wrong magic numbers",
					UnsupportedBitsPerPixel => "Unsupported bits per pixel",
					UnsupportedCompressionType => "Unsupported compression type",
					UnsupportedBmpVersion => "Unsupported BMP version",
					_ => "BMP Error",
				}
			}
		}

		pub fn decode_image(bmp_data: &mut Cursor<Vec<u8>>) -> BmpResult<Image> {
			read_bmp_id(bmp_data)?;
			let header = read_bmp_header(bmp_data)?;
			let dib_header = read_bmp_dib_header(bmp_data)?;

			let color_palette = read_color_palette(bmp_data, &dib_header)?;

			let width = dib_header.width.abs() as u32;
			let height = dib_header.height.abs() as u32;
			let padding = width % 4;

			let data = match color_palette {
				Some(ref palette) => read_indexes(
					bmp_data.get_mut(),
					&palette,
					width as usize,
					height as usize,
					dib_header.bits_per_pixel,
					header.pixel_offset as usize,
				)?,
				None => read_pixels(bmp_data, width, height, header.pixel_offset, padding as i64)?,
			};

			let image = Image {
				header,
				dib_header: BmpDibHeader::new(width as i32, height as i32),
				color_palette,
				width,
				height,
				padding,
				data,
			};

			Ok(image)
		}

		fn read_bmp_id(bmp_data: &mut Cursor<Vec<u8>>) -> BmpResult<()> {
			let mut bm = [0, 0];
			bmp_data.read(&mut bm)?;

			if bm == b"BM"[..] {
				Ok(())
			} else {
				Err(BmpError::new(
					WrongMagicNumbers,
					format!("Expected [66, 77], but was {:?}", bm),
				))
			}
		}

		fn read_bmp_header(bmp_data: &mut Cursor<Vec<u8>>) -> BmpResult<BmpHeader> {
			let header = BmpHeader {
				file_size: bmp_data.read_u32::<LittleEndian>()?,
				creator1: bmp_data.read_u16::<LittleEndian>()?,
				creator2: bmp_data.read_u16::<LittleEndian>()?,
				pixel_offset: bmp_data.read_u32::<LittleEndian>()?,
			};

			Ok(header)
		}

		fn read_bmp_dib_header(bmp_data: &mut Cursor<Vec<u8>>) -> BmpResult<BmpDibHeader> {
			let dib_header = BmpDibHeader {
				header_size: bmp_data.read_u32::<LittleEndian>()?,
				width: bmp_data.read_i32::<LittleEndian>()?,
				height: bmp_data.read_i32::<LittleEndian>()?,
				num_planes: bmp_data.read_u16::<LittleEndian>()?,
				bits_per_pixel: bmp_data.read_u16::<LittleEndian>()?,
				compress_type: bmp_data.read_u32::<LittleEndian>()?,
				data_size: bmp_data.read_u32::<LittleEndian>()?,
				hres: bmp_data.read_i32::<LittleEndian>()?,
				vres: bmp_data.read_i32::<LittleEndian>()?,
				num_colors: bmp_data.read_u32::<LittleEndian>()?,
				num_imp_colors: bmp_data.read_u32::<LittleEndian>()?,
			};

			match BmpVersion::from_dib_header(&dib_header) {
				// V3 is the only version that is "fully" supported (decompressed images are the exception)
				// We will also attempt to decode v4 and v5, but we ignore all the additional data in the header.
				// This should not impose a big problem because neither decompression, nor 16 and 32-bit images are supported,
				// so the decoding will likely fail due to these constraints either way.
				Some(BmpVersion::Three) | Some(BmpVersion::Four) | Some(BmpVersion::Five) => (),
				// Otherwise, report the errors
				Some(other) => return Err(BmpError::new(UnsupportedBmpVersion, other)),
				None => {
					return Err(BmpError::new(
						UnsupportedHeader,
						format!(
							"Only simple BMP images of version 3, 4, and 5 are currently supported. \
                Connot decode the image for the following header: {:?}",
							dib_header
						),
					));
				}
			}

			match dib_header.bits_per_pixel {
				// Currently supported
				1 | 4 | 8 | 24 => (),
				other => {
					return Err(BmpError::new(
						UnsupportedBitsPerPixel,
						format!(
							"Only 1, 4, 8, and 24 bits per pixel are currently supported, was: {}",
							other
						),
					));
				}
			}

			match CompressionType::from_u32(dib_header.compress_type) {
				CompressionType::Uncompressed => (),
				other => return Err(BmpError::new(UnsupportedCompressionType, other)),
			}

			Ok(dib_header)
		}

		fn read_color_palette(
			bmp_data: &mut Cursor<Vec<u8>>,
			dh: &BmpDibHeader,
		) -> BmpResult<Option<Vec<Pixel>>> {
			let num_entries = match dh.bits_per_pixel {
				// We have a color_palette if the num_colors in the dib header is not zero
				_ if dh.num_colors != 0 => dh.num_colors as usize,
				// Or if there are 8 or less bits per pixel
				bpp @ 1 | bpp @ 4 | bpp @ 8 => 1 << bpp,
				_ => return Ok(None),
			};

			let num_bytes = match BmpVersion::from_dib_header(&dh) {
				// Three bytes for v2. Though, this is currently not supported
				Some(BmpVersion::Two) => return Err(BmpError::new(UnsupportedBmpVersion, BmpVersion::Two)),
				// Each entry in the color_palette is four bytes for v3, v4, and v5
				_ => 4,
			};

			bmp_data.seek(SeekFrom::Start(BMP_HEADER_SIZE + dh.header_size as u64))?;

			let mut px = &mut [0; 4][0..num_bytes as usize];
			let mut color_palette = Vec::with_capacity(num_entries);
			for _ in 0..num_entries {
				bmp_data.read(&mut px)?;
				color_palette.push(px!(px[2], px[1], px[0]));
			}

			Ok(Some(color_palette))
		}

		fn read_indexes(
			bmp_data: &mut Vec<u8>,
			palette: &Vec<Pixel>,
			width: usize,
			height: usize,
			bpp: u16,
			offset: usize,
		) -> BmpResult<Vec<Pixel>> {
			let mut data = Vec::with_capacity(height * width);
			// Number of bytes to read from each row, varies based on bits_per_pixel
			let bytes_per_row = (width as f64 / (8.0 / bpp as f64)).ceil() as usize;
			for y in 0..height {
				let padding = match bytes_per_row % 4 {
					0 => 0,
					other => 4 - other,
				};
				let start = offset + (bytes_per_row + padding) * y;
				let bytes = &bmp_data[start..start + bytes_per_row];

				for i in bit_index(&bytes, bpp as usize, width as usize) {
					data.push(palette[i]);
				}
			}
			Ok(data)
		}

		fn read_pixels(
			bmp_data: &mut Cursor<Vec<u8>>,
			width: u32,
			height: u32,
			offset: u32,
			padding: i64,
		) -> BmpResult<Vec<Pixel>> {
			let mut data = Vec::with_capacity((height * width) as usize);
			// seek until data
			bmp_data.seek(SeekFrom::Start(offset as u64))?;
			// read pixels until padding
			let mut px = [0; 3];
			for _ in 0..height {
				for _ in 0..width {
					bmp_data.read(&mut px)?;
					data.push(px!(px[2], px[1], px[0]));
				}
				// seek padding
				bmp_data.seek(SeekFrom::Current(padding))?;
			}
			Ok(data)
		}

		const BITS: usize = 8;

		#[derive(Debug)]
		struct BitIndex<'a> {
			size: usize,
			nbits: usize,
			bits_left: usize,
			mask: u8,
			bytes: &'a [u8],
			index: usize,
		}

		fn bit_index<'a>(
			bytes: &'a [u8],
			nbits: usize,
			size: usize,
		) -> BitIndex<'a> {
			let bits_left = BITS - nbits;
			BitIndex {
				size,
				nbits,
				bits_left,
				mask: (!0 as u8 >> bits_left),
				bytes,
				index: 0,
			}
		}

		impl<'a> Iterator for BitIndex<'a> {
			type Item = usize;

			fn next(&mut self) -> Option<usize> {
				let n = self.index / BITS;
				let offset = self.bits_left - self.index % BITS;

				self.index += self.nbits;

				if self.size == 0 {
					None
				} else {
					self.size -= 1;
					self
						.bytes
						.get(n)
						.map(|&block| ((block & self.mask << offset) >> offset) as usize)
				}
			}
		}

		#[test]
		fn test_calculate_bit_index() {
			let bytes = vec![0b1000_0001, 0b1111_0001];

			let mut bi = bit_index(&bytes, 1, 15);
			assert_eq!(bi.next(), Some(1));
			assert_eq!(bi.next(), Some(0));
			assert_eq!(bi.next(), Some(0));
			assert_eq!(bi.next(), Some(0));
			assert_eq!(bi.next(), Some(0));
			assert_eq!(bi.next(), Some(0));
			assert_eq!(bi.next(), Some(0));
			assert_eq!(bi.next(), Some(1));
			assert_eq!(bi.next(), Some(1));
			assert_eq!(bi.next(), Some(1));
			assert_eq!(bi.next(), Some(1));
			assert_eq!(bi.next(), Some(1));
			assert_eq!(bi.next(), Some(0));
			assert_eq!(bi.next(), Some(0));
			assert_eq!(bi.next(), Some(0));
			assert_eq!(bi.next(), None);
			assert_eq!(bi.next(), None);

			let mut bi = bit_index(&bytes, 4, 4);
			assert_eq!(bi.next(), Some(0b1000));
			assert_eq!(bi.next(), Some(0b0001));
			assert_eq!(bi.next(), Some(0b1111));
			assert_eq!(bi.next(), Some(0b0001));
			assert_eq!(bi.next(), None);

			let mut bi = bit_index(&bytes, 8, 2);
			assert_eq!(bi.next(), Some(0b1000_0001));
			assert_eq!(bi.next(), Some(0b1111_0001));
			assert_eq!(bi.next(), None);
		}
	}
	mod encoder {
		use super::super::byteorder::{LittleEndian, WriteBytesExt};

		use std::io::{self, Write};

		use super::Image;

		const B: u8 = 66;
		const M: u8 = 77;

		pub fn encode_image(bmp_image: &Image) -> io::Result<Vec<u8>> {
			let mut bmp_data = Vec::with_capacity(bmp_image.header.file_size as usize);

			write_header(&mut bmp_data, bmp_image)?;
			write_data(&mut bmp_data, bmp_image)?;
			Ok(bmp_data)
		}

		fn write_header(
			bmp_data: &mut Vec<u8>,
			img: &Image,
		) -> io::Result<()> {
			let header = &img.header;
			let dib_header = &img.dib_header;
			let (header_size, data_size) = file_size!(24, img.width, img.height);

			io::Write::write(bmp_data, &[B, M])?;

			bmp_data.write_u32::<LittleEndian>(header_size + data_size)?;
			bmp_data.write_u16::<LittleEndian>(header.creator1)?;
			bmp_data.write_u16::<LittleEndian>(header.creator2)?;
			bmp_data.write_u32::<LittleEndian>(header_size)?; // pixel_offset

			bmp_data.write_u32::<LittleEndian>(dib_header.header_size)?;
			bmp_data.write_i32::<LittleEndian>(dib_header.width)?;
			bmp_data.write_i32::<LittleEndian>(dib_header.height)?;
			bmp_data.write_u16::<LittleEndian>(1)?; // num_planes
			bmp_data.write_u16::<LittleEndian>(24)?; // bits_per_pixel
			bmp_data.write_u32::<LittleEndian>(0)?; // compress_type
			bmp_data.write_u32::<LittleEndian>(data_size)?;
			bmp_data.write_i32::<LittleEndian>(dib_header.hres)?;
			bmp_data.write_i32::<LittleEndian>(dib_header.vres)?;
			bmp_data.write_u32::<LittleEndian>(0)?; // num_colors
			bmp_data.write_u32::<LittleEndian>(0)?; // num_imp_colors
			Ok(())
		}

		fn write_data(
			bmp_data: &mut Vec<u8>,
			img: &Image,
		) -> io::Result<()> {
			let padding = &[0; 4][0..img.padding as usize];
			for y in 0..img.height {
				for x in 0..img.width {
					let index = (y * img.width + x) as usize;
					let px = &img.data[index];
					Write::write(bmp_data, &[px.b, px.g, px.r])?;
				}
				Write::write(bmp_data, padding)?;
			}
			Ok(())
		}
	}

	/// The pixel data used in the `Image`.
	///
	/// It has three values for the `red`, `blue` and `green` color channels, respectively.
	#[repr(C)]
	#[derive(Clone, Copy, Debug, Eq, PartialEq)]
	pub struct Pixel {
		pub r: u8,
		pub g: u8,
		pub b: u8,
	}

	impl Pixel {
		/// Creates a new `Pixel`.
		pub fn new(
			r: u8,
			g: u8,
			b: u8,
		) -> Pixel {
			Pixel { r, g, b }
		}
	}

	/// Displays the rgb values as an rgb color triple
	impl fmt::Display for Pixel {
		fn fmt(
			&self,
			f: &mut fmt::Formatter,
		) -> fmt::Result {
			write!(f, "rgb({}, {}, {})", self.r, self.g, self.b)
		}
	}

	/// Displays the rgb values as an upper-case 24-bit hexadecimal number
	impl fmt::UpperHex for Pixel {
		fn fmt(
			&self,
			f: &mut fmt::Formatter,
		) -> fmt::Result {
			write!(f, "{:02X}{:02X}{:02X}", self.r, self.g, self.b)
		}
	}

	/// Displays the rgb values as a lower-case 24-bit hexadecimal number
	impl fmt::LowerHex for Pixel {
		fn fmt(
			&self,
			f: &mut fmt::Formatter,
		) -> fmt::Result {
			write!(f, "{:02x}{:02x}{:02x}", self.r, self.g, self.b)
		}
	}

	#[derive(Clone, Debug, Eq, PartialEq)]
	enum BmpVersion {
		Two,
		Three,
		ThreeNT,
		Four,
		Five,
	}

	impl BmpVersion {
		fn from_dib_header(dib_header: &BmpDibHeader) -> Option<BmpVersion> {
			match dib_header.header_size {
				12 => Some(BmpVersion::Two),
				40 if dib_header.compress_type == 3 => Some(BmpVersion::ThreeNT),
				40 => Some(BmpVersion::Three),
				108 => Some(BmpVersion::Four),
				124 => Some(BmpVersion::Five),
				_ => None,
			}
		}
	}

	impl AsRef<str> for BmpVersion {
		fn as_ref(&self) -> &str {
			match *self {
				BmpVersion::Two => "BMP Version 2",
				BmpVersion::Three => "BMP Version 3",
				BmpVersion::ThreeNT => "BMP Version 3 NT",
				BmpVersion::Four => "BMP Version 4",
				BmpVersion::Five => "BMP Version 5",
			}
		}
	}

	#[derive(Clone, Debug, Eq, PartialEq)]
	enum CompressionType {
		Uncompressed,
		Rle8bit,
		Rle4bit,
		// Only for BMP version 4
		BitfieldsEncoding,
	}

	impl CompressionType {
		fn from_u32(val: u32) -> CompressionType {
			match val {
				1 => CompressionType::Rle8bit,
				2 => CompressionType::Rle4bit,
				3 => CompressionType::BitfieldsEncoding,
				_ => CompressionType::Uncompressed,
			}
		}
	}

	impl AsRef<str> for CompressionType {
		fn as_ref(&self) -> &str {
			match *self {
				CompressionType::Rle8bit => "RLE 8-bit",
				CompressionType::Rle4bit => "RLE 4-bit",
				CompressionType::BitfieldsEncoding => "Bitfields Encoding",
				CompressionType::Uncompressed => "Uncompressed",
			}
		}
	}

	#[derive(Clone, Debug, Eq, PartialEq)]
	struct BmpHeader {
		file_size: u32,
		creator1: u16,
		creator2: u16,
		pixel_offset: u32,
	}

	impl BmpHeader {
		fn new(
			header_size: u32,
			data_size: u32,
		) -> BmpHeader {
			BmpHeader {
				file_size: header_size + data_size,
				creator1: 0, /* Unused */
				creator2: 0, /* Unused */
				pixel_offset: header_size,
			}
		}
	}

	#[derive(Clone, Debug, Eq, PartialEq)]
	struct BmpDibHeader {
		header_size: u32,
		width: i32,
		height: i32,
		num_planes: u16,
		bits_per_pixel: u16,
		compress_type: u32,
		data_size: u32,
		hres: i32,
		vres: i32,
		num_colors: u32,
		num_imp_colors: u32,
	}

	impl BmpDibHeader {
		fn new(
			width: i32,
			height: i32,
		) -> BmpDibHeader {
			let (_, pixel_array_size) = file_size!(24, width, height);
			BmpDibHeader {
				header_size: 40,
				width,
				height,
				num_planes: 1,
				bits_per_pixel: 24,
				compress_type: 0,
				data_size: pixel_array_size,
				hres: 1000,
				vres: 1000,
				num_colors: 0,
				num_imp_colors: 0,
			}
		}
	}

	/// The image type provided by the library.
	///
	/// It exposes functions to initialize or read BMP images from disk, common modification of pixel
	/// data, and saving to disk.
	///
	/// The image is accessed in row-major order from top to bottom,
	/// where point (0, 0) is defined to be in the upper left corner of the image.
	///
	/// Currently, only uncompressed BMP images are supported.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Image {
		header: BmpHeader,
		dib_header: BmpDibHeader,
		color_palette: Option<Vec<Pixel>>,
		width: u32,
		height: u32,
		padding: u32,
		pub data: Vec<Pixel>,
	}

	impl Image {
		/// Returns a new BMP Image with the `width` and `height` specified. It is initialized to
		/// a black image by default.
		///
		/// # Example
		///
		/// ```
		/// let mut img = bmp::Image::new(100, 80);
		/// ```
		pub fn new(
			width: u32,
			height: u32,
		) -> Image {
			let mut data = Vec::with_capacity((width * height) as usize);
			for _ in 0..width * height {
				data.push(px!(0, 0, 0));
			}

			let (header_size, data_size) = file_size!(24, width, height);
			Image {
				header: BmpHeader::new(header_size, data_size),
				dib_header: BmpDibHeader::new(width as i32, height as i32),
				color_palette: None,
				width,
				height,
				padding: width % 4,
				data,
			}
		}

		/// Returns the `width` of the Image.
		#[inline]
		pub fn get_width(&self) -> u32 {
			self.width
		}

		/// Returns the `height` of the Image.
		#[inline]
		pub fn get_height(&self) -> u32 {
			self.height
		}

		/// Set the pixel value at the position of `width` and `height`.
		///
		/// # Example
		///
		/// ```
		/// let mut img = bmp::Image::new(100, 80);
		/// img.set_pixel(10, 10, bmp::consts::RED);
		/// ```
		#[inline]
		pub fn set_pixel(
			&mut self,
			x: u32,
			y: u32,
			val: Pixel,
		) {
			self.data[((self.height - y - 1) * self.width + x) as usize] = val;
		}

		/// Returns the pixel value at the position of `width` and `height`.
		///
		/// # Example
		///
		/// ```
		/// let img = bmp::Image::new(100, 80);
		/// assert_eq!(bmp::consts::BLACK, img.get_pixel(10, 10));
		/// ```
		#[inline]
		pub fn get_pixel(
			&self,
			x: u32,
			y: u32,
		) -> Pixel {
			self.data[((self.height - y - 1) * self.width + x) as usize]
		}

		/// Returns a new `ImageIndex` that iterates over the image dimensions in top-bottom order.
		///
		/// # Example
		///
		/// ```
		/// let mut img = bmp::Image::new(100, 100);
		/// for (x, y) in img.coordinates() {
		///     img.set_pixel(x, y, bmp::consts::BLUE);
		/// }
		/// ```
		#[inline]
		pub fn coordinates(&self) -> ImageIndex {
			ImageIndex::new(self.width as u32, self.height as u32)
		}

		/// Saves the `Image` instance to the path specified by `path`.
		/// The function will overwrite the contents if a file already exists at the given path.
		///
		/// The function returns the `io::Result` from the underlying writer.
		///
		/// # Example
		///
		/// ```
		/// use bmp::Image;
		///
		/// let mut img = Image::new(100, 100);
		/// let _ = img.save("black.bmp").unwrap_or_else(|e| {
		///     panic!("Failed to save: {}", e)
		/// });
		/// ```
		pub fn save<P: AsRef<Path>>(
			&self,
			path: P,
		) -> io::Result<()> {
			let mut bmp_file = fs::File::create(path)?;
			self.to_writer(&mut bmp_file)
		}

		/// Writes the `Image` instance to the writer referenced by `destination`.
		pub fn to_writer<W: Write>(
			&self,
			destination: &mut W,
		) -> io::Result<()> {
			let bmp_data = encoder::encode_image(self)?;
			destination.write(&bmp_data)?;
			Ok(())
		}
	}

	impl fmt::Debug for Image {
		fn fmt(
			&self,
			f: &mut fmt::Formatter,
		) -> Result<(), fmt::Error> {
			f.debug_struct("Image")
				.field("header", &self.header)
				.field("dib_header", &self.dib_header)
				.field("color_palette", &self.color_palette)
				.field("width", &self.width)
				.field("height", &self.height)
				.field("padding", &self.padding)
				.finish()
		}
	}

	/// An `Iterator` returning the `x` and `y` coordinates of an image.
	///
	/// It supports iteration over an image in row-major order,
	/// starting from in the upper left corner of the image.
	#[derive(Clone, Copy)]
	pub struct ImageIndex {
		width: u32,
		height: u32,
		x: u32,
		y: u32,
	}

	impl ImageIndex {
		fn new(
			width: u32,
			height: u32,
		) -> ImageIndex {
			ImageIndex {
				width,
				height,
				x: 0,
				y: 0,
			}
		}
	}

	impl Iterator for ImageIndex {
		type Item = (u32, u32);

		fn next(&mut self) -> Option<(u32, u32)> {
			if self.x < self.width && self.y < self.height {
				let this = Some((self.x, self.y));
				self.x += 1;
				if self.x == self.width {
					self.x = 0;
					self.y += 1;
				}
				this
			} else {
				None
			}
		}
	}

	/// Utility function to load an `Image` from the file specified by `path`.
	/// It uses the `from_reader` function internally to decode the `Image`.
	/// Returns a `BmpResult`, either containing an `Image` or a `BmpError`.
	///
	/// # Example
	///
	/// ```
	/// let img = bmp::open("test/rgbw.bmp").unwrap_or_else(|e| {
	///    panic!("Failed to open: {}", e);
	/// });
	/// ```
	pub fn open<P: AsRef<Path>>(path: P) -> BmpResult<Image> {
		let mut f = fs::File::open(path)?;
		from_reader(&mut f)
	}

	/// Attempts to construct a new `Image` from the given reader.
	/// Returns a `BmpResult`, either containing an `Image` or a `BmpError`.
	pub fn from_reader<R: Read>(source: &mut R) -> BmpResult<Image> {
		let mut bytes = Vec::new();
		source.read_to_end(&mut bytes)?;

		let mut bmp_data = Cursor::new(bytes);
		decoder::decode_image(&mut bmp_data)
	}
}

use std::{
	ffi::{CStr, OsStr, c_char},
	os::unix::ffi::OsStrExt,
};

use crate::bmp::Pixel;

#[repr(C)]
pub struct BmpImage {
	ptrRawData: *mut crate::bmp::Pixel,
	ulRawDataSize: u32,
	uiWidth: u32,
	uiHeight: u32,
	header: [u8; 12],
	dib_header: [u8; 40],
}
fn transform(_inp: crate::bmp::Image) -> BmpImage {
	todo!("BmpImage is placeholder type!")
}
fn transform_back(_inp: *mut BmpImage) -> crate::bmp::Image {
	todo!()
}
#[repr(C)]
pub struct ResultBmpImage {
	ptr: *mut BmpImage,
	tag: u8,
}
#[unsafe(no_mangle)]
pub extern "C" fn open_bmp(s: *const c_char) -> ResultBmpImage {
	use crate::bmp::BmpErrorKind::*;
	match bmp::open(std::path::Path::new(OsStr::from_bytes(
		unsafe { CStr::from_ptr(s) }.to_bytes(),
	))) {
		Ok(a) => {
			let mut b = transform(a);
			ResultBmpImage {
				ptr: &raw mut b,
				tag: 0,
			}
		}
		Err(bmp::BmpError { kind, details: _ }) => ResultBmpImage {
			ptr: std::ptr::null_mut(),
			tag: match kind {
				WrongMagicNumbers => 1,
				UnsupportedBitsPerPixel => 2,
				UnsupportedCompressionType => 2,
				UnsupportedBmpVersion => 2,
				UnsupportedHeader => 2,
				BmpIoError(_) => 3,
			},
		},
	}
}
pub extern "C" fn write_bmp(
	s: *const c_char,
	img: *mut BmpImage,
) {
	transform_back(img)
		.save(std::path::Path::new(OsStr::from_bytes(
			unsafe { CStr::from_ptr(s) }.to_bytes(),
		)))
		.unwrap()
}
pub extern "C" fn index(
	img: *mut BmpImage,
	x: u32,
	y: u32,
) -> u32 {
	let a = transform_back(img);
	x + y * a.get_height()
}
pub extern "C" fn read_pixel(
	img: *mut BmpImage,
	x: u32,
	y: u32,
) -> ResultRGBA {
	transform_back(img).get_pixel(x, y);
	todo!()
}
pub extern "C" fn set_pixel(
	img: *mut BmpImage,
	x: u32,
	y: u32,
	color: Pixel,
) {
	transform_back(img).set_pixel(x, y, color);
}

#[cfg(test)]
mod tests {
	#[test]
	fn it_works() {
		let result = 2 + 2;
		assert_eq!(result, 4);
	}
}
