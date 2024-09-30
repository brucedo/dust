use core::fmt;
use log::{debug, error};
use std::fmt::Display;

pub struct Bitmap {}

pub enum BitmapError {
    UnknownHeader,
    UnknownDIBHeader,
    UnsupportedDIBHeader,
}

type Header = [u8; 14];
type BitmapCoreHeader = [u8; 12];
type OS22XBitmapHeaderV1ByteArray = [u8; 64];
type OS22XBitmapHeaderV2 = [u8; 16];
type BitmapInfoHeader = [u8; 40];
type BitmapV2InfoHeader = [u8; 52];
type BitmapV3InfoHeader = [u8; 56];
type BitmapV4Header = [u8; 108];
type BitmapV5Header = [u8; 124];
pub type RGB = [u8; 3];
pub type RGBA = [u8; 4];

enum BitmapType {
    Windows,
    OS2BitmapArray,
    OS2ColorIcon,
    OS2ColorPointer,
    OS2Icon,
    OS2Pointer,
}

impl Display for BitmapType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BitmapType::Windows => write!(f, "Windows"),
            BitmapType::OS2BitmapArray => write!(f, "OS/2 Bitmap Array"),
            BitmapType::OS2ColorIcon => write!(f, "OS/2 Bitmap Color Icon"),
            BitmapType::OS2ColorPointer => write!(f, "OS/2 Bitmap Color Pointer"),
            BitmapType::OS2Icon => write!(f, "OS/2 Bitmap Icon"),
            BitmapType::OS2Pointer => write!(f, "OS/2 Bitmap Pointer"),
        }
    }
}

#[derive(Debug)]
enum DeviceIndependentBitmapType {
    Bitmap(BitmapCoreHeader),
    OS22BitmapV1(OS22XBitmapHeaderV1),
    OS22BitmapV2(OS22XBitmapHeaderV2),
    Windows3_1(Windows3_1BitmapHeader),
    AdobeRGB(BitmapV2InfoHeader),
    AdobeRGBA(BitmapV3InfoHeader),
    Windows95(BitmapV4Header),
    Windows98(BitmapV5Header),
}

#[derive(Debug)]
enum BmpCompression {
    None,
    RunLengthEncoding8BPP,
    RunLengthEncoding4BPP,
    Huffman1D,
    RunLengthEncoding24,
    RGBABitFieldMasks,
    RunLengthEncoding8,
    RunLengthEncoding4,
}

#[derive(Debug)]
enum BitmapOrigin {
    BottomLeft,
    TopLeft,
}

#[derive(Debug)]
enum HalftoneAlgorithm {
    None,
    ErrorDiffusion(u32),
    PANDA((u32, u32)),
    SuperCircle((u32, u32)),
}

#[derive(Debug)]
struct Windows3_1BitmapHeader {
    pub width: i32,
    pub height: i32,
    pub color_planes: u8,
    pub color_depth: u16,
    pub compression: BmpCompression,
    pub raw_image_size: u32,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    pub palette_color_count: u32,
    pub important_colors_count: u32,
}

#[derive(Debug)]
struct OS22XBitmapHeaderV1 {
    pub width: i32,
    pub height: i32,
    pub color_planes: u8,
    pub color_depth: u16,
    pub compression: BmpCompression,
    pub raw_image_size: u32,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    pub palette_color_count: u32,
    pub important_colors_count: u32,
    pub resolution_units: u16,
    pub origin: BitmapOrigin,
    pub halftone: HalftoneAlgorithm,
    pub color_encoding: u32,
    pub application_defined: u32,
}

struct ColorBlock<T> {
    pub red_mask: Option<u32>,
    pub green_mask: Option<u32>,
    pub blue_mask: Option<u32>,
    pub alpha_mask: Option<u32>,
    pub color_table: Option<Vec<T>>,
}

pub fn new(contents: &[u8]) -> Result<Bitmap, BitmapError> {
    debug!("Attempting to load bitmap.");
    if contents.len() < 14 {
        error!("File Header failed basic file size check.");
        return Err(BitmapError::UnknownHeader);
    }

    let header: Header = contents[0..14].try_into().unwrap();
    let bmp_type = decode_type(&header)?;
    let bmp_size = decode_size(&header);
    let bmp_pixel_start = decode_pixel_array_start_address(&header);

    debug!("Header type: {}", bmp_type);
    debug!("Header size: {}", bmp_size);
    debug!("Pixel data start: {}", bmp_pixel_start);

    let dib_header_section: [u8; 124] = contents[14..138].try_into().unwrap();
    let dib_header = decode_dib_header(&dib_header_section)?;

    match dib_header {
        DeviceIndependentBitmapType::Windows3_1(win_3_1_header) => {
            debug!("Win 3.1 Header Info: ");
            debug!("  Width:        {}", win_3_1_header.width);
            debug!("  Height:       {}", win_3_1_header.height);
            debug!("  Color Depth:  {}", win_3_1_header.color_depth);
            debug!("  Color Planes: {}", win_3_1_header.color_planes);
            debug!("  Compression:  {:?}", win_3_1_header.compression);
            debug!("  Image Data Size: {}", win_3_1_header.raw_image_size);
            debug!(
                "  Horizontal Resolution: {}",
                win_3_1_header.horizontal_resolution
            );
            debug!(
                "  Vertical Resolution: {} ",
                win_3_1_header.vertical_resolution
            );
            debug!(
                "  Palette Color Count: {}",
                win_3_1_header.palette_color_count
            );
            debug!(
                "  Important Color Count: {}",
                win_3_1_header.important_colors_count
            );
            let color_block = decode_windows_31_color_block(&contents[54..], &win_3_1_header);
            debug!("Color block info: ");
            debug!("  red_bitmask:   {:?}", color_block.red_mask);
            debug!("  green_bitmask: {:?}", color_block.green_mask);
            debug!("  blue_bitmask:  {:?}", color_block.blue_mask);
            if color_block.color_table.is_some() {
                debug!("  table count:  {}", color_block.color_table.unwrap().len());
            } else {
                debug!("  table count:  None");
            }
        }
        other_type => {
            error!("Unexpectedly complex bitmap.  Oops: {:?}", other_type);
        }
    }

    Ok(Bitmap {})
}

fn decode_windows_31_pixels(color_depth: u8, stride: usize, pixel_store: &[u8]) {
    let bytes_per_color = color_depth / 4;
    let data_bytes_per_row = stride * bytes_per_color as usize;
    let padding = (4 - data_bytes_per_row % 4) % 4;

    debug!(
        "Beginning pixel read: {} {}-bpp pixels per row with {} bytes padding at the end.",
        stride, color_depth, padding
    );
}

fn decode_windows_31_color_block(
    block: &[u8],
    reference_header: &Windows3_1BitmapHeader,
) -> ColorBlock<RGBA> {
    let (offset, red, green, blue, alpha) = match reference_header.compression {
        BmpCompression::Huffman1D => (
            12,
            Some(u32::from_le_bytes(block[0..4].try_into().unwrap())),
            Some(u32::from_le_bytes(block[4..8].try_into().unwrap())),
            Some(u32::from_le_bytes(block[8..12].try_into().unwrap())),
            None,
        ),
        BmpCompression::RGBABitFieldMasks => (
            16,
            Some(u32::from_le_bytes(block[0..4].try_into().unwrap())),
            Some(u32::from_le_bytes(block[4..8].try_into().unwrap())),
            Some(u32::from_le_bytes(block[8..12].try_into().unwrap())),
            Some(u32::from_le_bytes(block[12..16].try_into().unwrap())),
        ),
        _ => (0, None, None, None, None),
    };

    let color_table = if reference_header.palette_color_count > 0 {
        let mut table = Vec::with_capacity(reference_header.palette_color_count as usize);

        for index in 0..reference_header.palette_color_count {
            let start_byte: usize = (offset + index * 4) as usize;
            let end_byte = start_byte + 4;
            let color: RGBA = block[start_byte..end_byte].try_into().unwrap();
            table.push(color);
        }

        Some(table)
    } else {
        None
    };

    ColorBlock {
        red_mask: red,
        green_mask: green,
        blue_mask: blue,
        alpha_mask: alpha,
        color_table,
    }
}

fn decode_dib_header(header: &[u8; 124]) -> Result<DeviceIndependentBitmapType, BitmapError> {
    let header_bytes: [u8; 4] = header[0..4].try_into().unwrap();
    let header_size = u32::from_le_bytes(header_bytes);

    match header_size {
        12 => Err(BitmapError::UnsupportedDIBHeader),
        64 => Err(BitmapError::UnsupportedDIBHeader),
        16 => Err(BitmapError::UnsupportedDIBHeader),
        40 => {
            let win_3_1_header_array: [u8; 40] = header[0..40].try_into().unwrap();
            let win_3_1_header = decode_windows_31_dib_header(&win_3_1_header_array)?;
            Ok(DeviceIndependentBitmapType::Windows3_1(win_3_1_header))
        }
        52 => Err(BitmapError::UnsupportedDIBHeader),
        56 => Err(BitmapError::UnsupportedDIBHeader),
        108 => Err(BitmapError::UnsupportedDIBHeader),
        124 => Err(BitmapError::UnsupportedDIBHeader),
        _ => Err(BitmapError::UnknownDIBHeader),
    }
}

fn decode_windows_31_dib_header(
    header: &BitmapInfoHeader,
) -> Result<Windows3_1BitmapHeader, BitmapError> {
    let width_bytes = header[4..8].try_into().unwrap();
    let height_bytes = header[8..12].try_into().unwrap();
    let color_plane_bytes = header[12..14].try_into().unwrap();
    let color_depth_bytes = header[14..16].try_into().unwrap();
    let compression_bytes = header[16..20].try_into().unwrap();
    let image_size_bytes = header[20..24].try_into().unwrap();
    let horizontal_resolution_bytes = header[24..28].try_into().unwrap();
    let vertical_resolution_bytes = header[28..32].try_into().unwrap();
    let num_color_bytes = header[32..36].try_into().unwrap();
    let import_color_bytes = header[36..40].try_into().unwrap();

    Ok(Windows3_1BitmapHeader {
        width: i32::from_le_bytes(width_bytes),
        height: i32::from_le_bytes(height_bytes),
        color_planes: i16::from_le_bytes(color_plane_bytes).try_into().unwrap(),
        color_depth: u16::from_le_bytes(color_depth_bytes),
        compression: decode_compression_type(compression_bytes),
        raw_image_size: u32::from_le_bytes(image_size_bytes),
        horizontal_resolution: u32::from_le_bytes(horizontal_resolution_bytes),
        vertical_resolution: u32::from_le_bytes(vertical_resolution_bytes),
        palette_color_count: u32::from_le_bytes(num_color_bytes),
        important_colors_count: u32::from_le_bytes(import_color_bytes),
    })
}

fn decode_os2_v1_dib(
    header: OS22XBitmapHeaderV1ByteArray,
) -> Result<OS22XBitmapHeaderV1, BitmapError> {
    let bitmap_info_subset = header[0..40].try_into().unwrap();

    let win31_subset = decode_windows_31_dib_header(&bitmap_info_subset)?;
    let resolution_units = u16::from_le_bytes(header[40..42].try_into().unwrap());
    let origin = match u16::from_le_bytes(header[44..46].try_into().unwrap()) {
        0 => BitmapOrigin::BottomLeft,
        _ => BitmapOrigin::TopLeft,
    };
    let halftone = decode_halftone(&header[46..56]);
    let color_encoding: u32 = u32::from_le_bytes(header[56..60].try_into().unwrap());
    let application_defined: u32 = u32::from_le_bytes(header[60..64].try_into().unwrap());

    Ok(OS22XBitmapHeaderV1 {
        width: win31_subset.width,
        height: win31_subset.height,
        color_planes: win31_subset.color_planes,
        color_depth: win31_subset.color_depth,
        compression: win31_subset.compression,
        raw_image_size: win31_subset.raw_image_size,
        horizontal_resolution: win31_subset.horizontal_resolution,
        vertical_resolution: win31_subset.vertical_resolution,
        palette_color_count: win31_subset.palette_color_count,
        important_colors_count: win31_subset.important_colors_count,
        resolution_units,
        origin,
        halftone,
        color_encoding,
        application_defined,
    })
}

fn decode_halftone(halftone_fields: &[u8]) -> HalftoneAlgorithm {
    let halftone_variant: u16 = u16::from_le_bytes(halftone_fields[0..2].try_into().unwrap());
    let halftone_param1: u32 = u32::from_le_bytes(halftone_fields[2..6].try_into().unwrap());
    let halftone_param2: u32 = u32::from_le_bytes(halftone_fields[6..8].try_into().unwrap());

    match halftone_variant {
        0 => HalftoneAlgorithm::None,
        1 => HalftoneAlgorithm::ErrorDiffusion(halftone_param1),
        2 => HalftoneAlgorithm::PANDA((halftone_param1, halftone_param2)),
        3 => HalftoneAlgorithm::SuperCircle((halftone_param1, halftone_param2)),
        _ => HalftoneAlgorithm::None,
    }
}

fn decode_compression_type(compression_field: &[u8; 4]) -> BmpCompression {
    match u32::from_le_bytes(*compression_field) {
        0 => BmpCompression::None,
        1 => BmpCompression::RunLengthEncoding8BPP,
        2 => BmpCompression::RunLengthEncoding4BPP,
        3 => BmpCompression::Huffman1D,
        4 => BmpCompression::RunLengthEncoding24,
        5 => BmpCompression::None,
        6 => BmpCompression::RGBABitFieldMasks,
        11 => BmpCompression::None,
        12 => BmpCompression::RunLengthEncoding8,
        13 => BmpCompression::RunLengthEncoding4,
        _ => BmpCompression::None,
    }
}

fn decode_type(header: &Header) -> Result<BitmapType, BitmapError> {
    match (header[0], header[1]) {
        (0x42, 0x4D) => Ok(BitmapType::Windows),
        (0x42, 0x41) => Ok(BitmapType::OS2BitmapArray),
        (0x43, 0x49) => Ok(BitmapType::OS2ColorIcon),
        (0x43, 0x50) => Ok(BitmapType::OS2ColorPointer),
        (0x49, 0x43) => Ok(BitmapType::OS2Icon),
        (0x50, 0x54) => Ok(BitmapType::OS2Pointer),
        _ => Err(BitmapError::UnknownHeader),
    }
}

fn decode_size(header: &Header) -> u32 {
    let size_ref: [u8; 4] = header[2..6].try_into().unwrap();
    u32::from_le_bytes(size_ref)
}

fn decode_pixel_array_start_address(header: &Header) -> u32 {
    let start_addr_ref: [u8; 4] = header[10..14].try_into().unwrap();
    u32::from_le_bytes(start_addr_ref)
}

impl Bitmap {}
