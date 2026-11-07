//! Local QR rendering shared by invitations, links and text messages.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{DynamicImage, ImageFormat, Luma};
use qrcode::QrCode;
use std::io::Cursor;

pub(crate) fn source(raw: &str, minimum_size: u32) -> Option<String> {
    if raw.is_empty() {
        return None;
    }
    let code = QrCode::new(raw).ok()?;
    let bitmap = code
        .render::<Luma<u8>>()
        .min_dimensions(minimum_size, minimum_size)
        .quiet_zone(true)
        .build();
    let mut png = Cursor::new(Vec::new());
    DynamicImage::ImageLuma8(bitmap)
        .write_to(&mut png, ImageFormat::Png)
        .ok()?;
    Some(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(png.into_inner())
    ))
}
