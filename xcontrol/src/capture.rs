use std::io::Cursor;
use windows_sys::Win32::Graphics::Gdi::*;

pub fn capture_screen() -> Option<Vec<u8>> {
    unsafe {
        let screen_dc = GetDC(std::ptr::null_mut());
        if screen_dc.is_null() {
            return None;
        }

        let width = GetDeviceCaps(screen_dc, HORZRES as i32);
        let height = GetDeviceCaps(screen_dc, VERTRES as i32);

        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.is_null() {
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            return None;
        }

        let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
        if bitmap.is_null() {
            DeleteDC(mem_dc);
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            return None;
        }

        let old = SelectObject(mem_dc, bitmap as _);
        BitBlt(mem_dc, 0, 0, width, height, screen_dc, 0, 0, SRCCOPY);
        SelectObject(mem_dc, old);

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width;
        bmi.bmiHeader.biHeight = -height;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let pixel_size = (width * height * 4) as usize;
        let mut pixels = vec![0u8; pixel_size];

        let res = GetDIBits(
            mem_dc,
            bitmap,
            0,
            height as u32,
            pixels.as_mut_ptr() as _,
            &mut bmi,
            DIB_RGB_COLORS,
        );

        DeleteObject(bitmap as _);
        DeleteDC(mem_dc);
        ReleaseDC(std::ptr::null_mut(), screen_dc);

        if res == 0 {
            return None;
        }

        match image::RgbaImage::from_raw(width as u32, height as u32, pixels) {
            Some(img) => {
                let rgb_img = image::DynamicImage::ImageRgba8(img).into_rgb8();
                let mut buffer = Cursor::new(Vec::new());
                let mut encoder =
                    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, 70);
                if encoder
                    .encode(
                        rgb_img.as_raw(),
                        rgb_img.width(),
                        rgb_img.height(),
                        image::ExtendedColorType::Rgb8,
                    )
                    .is_ok()
                {
                    Some(buffer.into_inner())
                } else {
                    None
                }
            }
            None => None,
        }
    }
}
