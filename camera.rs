use image::{RgbaImage, Rgba};
#[cfg(any(target_os = "ios", target_os = "macos"))]
extern "C" {
    fn start_camera_capture();
    fn check_camera_access() -> *const std::ffi::c_char;
    fn get_latest_frame() -> *mut std::ffi::c_void;
    fn get_latest_frame_stride() -> i32;
    fn get_initial_frame_size() -> i32;
    fn get_initial_frame_width() -> i32;
    fn get_initial_frame_height() -> i32;
}

pub struct Camera;

impl Camera {
    pub fn access() -> Result<String, String> {
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        let camera_access_status = unsafe { check_camera_access() };
        
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        if !camera_access_status.is_null() {
            let cstr = unsafe { std::ffi::CStr::from_ptr(camera_access_status) };
            let status = cstr.to_string_lossy().into_owned();
            return Ok(status)
        }

        Err("Failed to get camera access status".to_string())
    }

    pub fn capture() {
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        unsafe {
            start_camera_capture();
        }
    }

    pub fn get() -> Option<RgbaImage> {
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        unsafe {
            let ptr = get_latest_frame();
            let size = get_initial_frame_size();
            let stride = get_latest_frame_stride() as usize;
            let width = get_initial_frame_width() as u32;
            let height = get_initial_frame_height() as u32;
    
            if ptr.is_null() || size <= 0 || width == 0 || height == 0 {
                return None;
            }
    
            let slice = std::slice::from_raw_parts(ptr as *const u8, size as usize);
            let mut image = RgbaImage::new(width, height);
    
            let mut pixels = image.pixels_mut();
    
            for y in 0..height {
                let row_start = y as usize * stride;
                for x in 0..width {
                    let src_index = row_start + x as usize * 4;
                    if src_index + 3 >= slice.len() {
                        continue;
                    }
    
                    let r = slice[src_index + 2];
                    let g = slice[src_index + 1]; 
                    let b = slice[src_index];
                    let a = slice[src_index + 3]; 
    
                    let pixel = pixels.next().unwrap();
                    *pixel = Rgba([r, g, b, a]);
                }
            }
    
            #[cfg(target_os = "ios")]
            return Some(image::imageops::rotate90(&image));
            #[cfg(not(target_os = "ios"))]
            return Some(image);
        }
        None
    }
}
