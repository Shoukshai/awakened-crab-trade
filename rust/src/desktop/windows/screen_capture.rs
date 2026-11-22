//! Desktop Duplication fallback for hardware-composited cloud video.
use super::ocr::Image;
use crate::price_layout::Rect;
use windows::{
    core::Interface,
    Win32::{
        Foundation::{E_ABORT, E_FAIL, HMODULE},
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
pub struct Stream {
    // Release duplication and its context before the owning device.
    duplication: IDXGIOutputDuplication,
    context: ID3D11DeviceContext,
    device: ID3D11Device,
    area: Rect,
}
impl Stream {
    pub fn open(rect: Rect) -> Result<Self, String> {
        unsafe { Self::open_inner(rect) }.map_err(|e| e.to_string())
    }
    unsafe fn open_inner(rect: Rect) -> windows::core::Result<Self> {
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
                return Ok(Stream {
                    device,
                    context,
                    duplication,
                    area: Rect {
                        x: rect.x - r.left,
                        y: rect.y - r.top,
                        ..rect
                    },
                });
            }
        }
        Err(windows::core::Error::new(
            E_FAIL,
            "No matching desktop duplication output",
        ))
    }
    pub fn poll(
        &self,
        timeout: u32,
        check: &impl Fn() -> Result<(), String>,
    ) -> Result<Option<(Image, i64)>, String> {
        unsafe { self.poll_inner(timeout, check) }.map_err(|e| e.to_string())
    }
    unsafe fn poll_inner(
        &self,
        timeout: u32,
        check: &impl Fn() -> Result<(), String>,
    ) -> windows::core::Result<Option<(Image, i64)>> {
        check().map_err(|e| windows::core::Error::new(E_ABORT, e))?;
        let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut resource = None;
        match self
            .duplication
            .AcquireNextFrame(timeout, &mut info, &mut resource)
        {
            Err(error) if error.code() == DXGI_ERROR_WAIT_TIMEOUT => return Ok(None),
            result => result?,
        }
        let _release = Acquired(self.duplication.clone());
        if info.LastPresentTime == 0 {
            return Ok(None);
        }
        if info.ProtectedContentMaskedOut.as_bool() {
            eprintln!("ocr Desktop Duplication: Windows masked protected desktop content");
        }
        let texture: ID3D11Texture2D = resource
            .ok_or_else(windows::core::Error::from_thread)?
            .cast()?;
        let image = read_texture(&self.device, &self.context, &texture, self.area)?;
        check().map_err(|e| windows::core::Error::new(E_ABORT, e))?;
        Ok(Some((image, info.LastPresentTime)))
    }
}
pub fn capture(rect: Rect, check: &impl Fn() -> Result<(), String>) -> Result<Image, String> {
    let stream = Stream::open(rect)?;
    for _ in 0..8 {
        if let Some((image, _)) = stream.poll(120, check)? {
            if crate::capture::frame::has_content(&image.pixels) {
                return Ok(image);
            }
        }
    }
    Err("No readable desktop update within 960 ms".into())
}

/// Copy only the validated client area; both GPU backends share the CPU readback.
pub(super) unsafe fn read_texture(
    device: &ID3D11Device,
    context: &ID3D11DeviceContext,
    texture: &ID3D11Texture2D,
    area: Rect,
) -> windows::core::Result<Image> {
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    texture.GetDesc(&mut desc);
    if area.x < 0
        || area.y < 0
        || area.width <= 0
        || area.height <= 0
        || i64::from(area.x) + i64::from(area.width) > i64::from(desc.Width)
        || i64::from(area.y) + i64::from(area.height) > i64::from(desc.Height)
        || i64::from(area.width) * i64::from(area.height) > 24_000_000
    {
        return Err(windows::core::Error::new(
            E_FAIL,
            "Capture area exceeds the GPU frame",
        ));
    }
    if desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM && desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM_SRGB {
        return Err(windows::core::Error::new(
            E_FAIL,
            "Unsupported desktop pixel format",
        ));
    }
    desc.Width = area.width as u32;
    desc.Height = area.height as u32;
    desc.Usage = D3D11_USAGE_STAGING;
    desc.BindFlags = 0;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    desc.MiscFlags = 0;
    desc.MipLevels = 1;
    desc.ArraySize = 1;
    let mut staging = None;
    device.CreateTexture2D(&desc, None, Some(&mut staging))?;
    let staging = staging.ok_or_else(windows::core::Error::from_thread)?;
    let region = D3D11_BOX {
        left: area.x as u32,
        top: area.y as u32,
        front: 0,
        right: (area.x + area.width) as u32,
        bottom: (area.y + area.height) as u32,
        back: 1,
    };
    context.CopySubresourceRegion(&staging, 0, 0, 0, 0, texture, 0, Some(&region));
    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;
    let result = if mapped.pData.is_null() || mapped.RowPitch < area.width as u32 * 4 {
        Err(windows::core::Error::new(
            E_FAIL,
            "Invalid GPU readback buffer",
        ))
    } else {
        let width = area.width as usize;
        let height = area.height as usize;
        let mut pixels = Vec::with_capacity(width * height * 4);
        for y in 0..height {
            pixels.extend_from_slice(std::slice::from_raw_parts(
                (mapped.pData as *const u8).add(y * mapped.RowPitch as usize),
                width * 4,
            ));
        }
        Ok(Image {
            width,
            height,
            pixels,
        })
    };
    context.Unmap(&staging, 0);
    result
}
