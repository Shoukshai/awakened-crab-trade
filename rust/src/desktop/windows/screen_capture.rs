//! Desktop Duplication reads hardware-composited cloud video on Windows 10+.
use super::ocr::Image;
use crate::price_layout::Rect;
use windows::{
    core::Interface,
    Win32::{
        Foundation::HMODULE,
        Graphics::{
            Direct3D::D3D_DRIVER_TYPE_UNKNOWN,
            Direct3D11::*,
            Dxgi::{Common::*, *},
        },
    },
};
struct Acquired(IDXGIOutputDuplication);
impl Drop for Acquired {
    fn drop(&mut self) {
        unsafe {
            let _ = self.0.ReleaseFrame();
        }
    }
}
pub fn capture(rect: Rect) -> Result<Image, String> {
    unsafe { capture_inner(rect) }.map_err(|e| e.to_string())
}
unsafe fn capture_inner(rect: Rect) -> windows::core::Result<Image> {
    let factory: IDXGIFactory1 = CreateDXGIFactory1()?;
    let mut index = 0;
    while let Ok(adapter) = factory.EnumAdapters1(index) {
        index += 1;
        let mut output_index = 0;
        while let Ok(output) = adapter.EnumOutputs(output_index) {
            output_index += 1;
            let desc = output.GetDesc()?;
            let r = desc.DesktopCoordinates;
            if !desc.AttachedToDesktop.as_bool()
                || desc.Rotation != DXGI_MODE_ROTATION_IDENTITY
                || rect.x < r.left
                || rect.y < r.top
                || rect.x + rect.width > r.right
                || rect.y + rect.height > r.bottom
            {
                continue;
            }
            let mut device = None;
            let mut context = None;
            D3D11CreateDevice(
                &adapter,
                D3D_DRIVER_TYPE_UNKNOWN,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )?;
            let device = device.ok_or_else(windows::core::Error::from_thread)?;
            let context = context.ok_or_else(windows::core::Error::from_thread)?;
            let output: IDXGIOutput1 = output.cast()?;
            let duplication = output.DuplicateOutput(&device)?;
            let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
            let mut resource = None;
            duplication.AcquireNextFrame(120, &mut info, &mut resource)?;
            let _release = Acquired(duplication);
            let texture: ID3D11Texture2D = resource
                .ok_or_else(windows::core::Error::from_thread)?
                .cast()?;
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            texture.GetDesc(&mut desc);
            desc.Width = rect.width as u32;
            desc.Height = rect.height as u32;
            desc.Usage = D3D11_USAGE_STAGING;
            desc.BindFlags = 0;
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
            desc.MiscFlags = 0;
            if desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM {
                return Err(windows::core::Error::new(
                    windows::core::HRESULT(0x80004005u32 as i32),
                    "Unsupported desktop pixel format",
                ));
            }
            let mut staging = None;
            device.CreateTexture2D(&desc, None, Some(&mut staging))?;
            let staging = staging.ok_or_else(windows::core::Error::from_thread)?;
            let area = D3D11_BOX {
                left: (rect.x - r.left) as u32,
                top: (rect.y - r.top) as u32,
                front: 0,
                right: (rect.x + rect.width - r.left) as u32,
                bottom: (rect.y + rect.height - r.top) as u32,
                back: 1,
            };
            context.CopySubresourceRegion(&staging, 0, 0, 0, 0, &texture, 0, Some(&area));
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;
            let width = rect.width as usize;
            let height = rect.height as usize;
            let mut pixels = Vec::with_capacity(width * height * 4);
            for y in 0..height {
                pixels.extend_from_slice(std::slice::from_raw_parts(
                    (mapped.pData as *const u8).add(y * mapped.RowPitch as usize),
                    width * 4,
                ));
            }
            context.Unmap(&staging, 0);
            return Ok(Image {
                width,
                height,
                pixels,
            });
        }
    }
    Err(windows::core::Error::new(
        windows::core::HRESULT(0x80004005u32 as i32),
        "No matching desktop duplication output",
    ))
}
