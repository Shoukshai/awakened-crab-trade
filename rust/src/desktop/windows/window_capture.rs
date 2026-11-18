//! HWND capture through the Windows compositor, independent of monitor/GPU choice.
use super::{
    ocr::{Apartment, Image},
    screen_capture::read_texture,
};
use crate::price_layout::Rect;
use std::time::{Duration, Instant};
use windows::{
    core::{factory, Interface},
    Graphics::{
        Capture::*,
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
    },
    Win32::{
        Foundation::{E_POINTER, HMODULE, HWND, RECT},
        Graphics::{Direct3D::*, Direct3D11::*, Dwm::*, Dxgi::*},
        System::WinRT::{Direct3D11::*, Graphics::Capture::IGraphicsCaptureItemInterop},
    },
};

struct Pool(Direct3D11CaptureFramePool);
impl Drop for Pool {
    fn drop(&mut self) {
        let _ = self.0.Close();
    }
}
struct Session(GraphicsCaptureSession);
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.0.Close();
    }
}
struct Frame(Direct3D11CaptureFrame);
impl Drop for Frame {
    fn drop(&mut self) {
        let _ = self.0.Close();
    }
}

pub fn capture(
    window: HWND,
    rect: Rect,
    check: &impl Fn() -> Result<(), String>,
) -> Result<Image, String> {
    let _apartment = Apartment::new()?;
    check()?;
    let (device, context, pool, session) = unsafe { prepare(window) }.map_err(|e| e.to_string())?;
    // Both closable handles survive until after the final frame has been read.
    let _session = session;
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(2) {
        check()?;
        match pool.0.TryGetNextFrame() {
            Ok(frame) => {
                let frame = Frame(frame);
                let image = unsafe { read_frame(window, rect, &device, &context, &frame.0) }
                    .map_err(|e| e.to_string())?;
                check()?;
                if crate::capture::frame::has_content(&image.pixels) {
                    return Ok(image);
                }
            }
            Err(error) if error.code() == E_POINTER => {}
            Err(error) => return Err(error.to_string()),
        }
        std::thread::sleep(Duration::from_millis(15));
    }
    Err("No readable compositor frame within 2 seconds".into())
}

unsafe fn prepare(
    window: HWND,
) -> windows::core::Result<(ID3D11Device, ID3D11DeviceContext, Pool, Session)> {
    let interop: IGraphicsCaptureItemInterop = factory::<GraphicsCaptureItem, _>()?;
    let item: GraphicsCaptureItem = interop.CreateForWindow(window)?;
    let size = item.Size()?;
    if size.Width <= 0
        || size.Height <= 0
        || i64::from(size.Width) * i64::from(size.Height) > 24_000_000
    {
        return Err(windows::core::Error::new(
            windows::Win32::Foundation::E_FAIL,
            "Unsupported compositor frame size",
        ));
    }
    let mut device = None;
    let mut context = None;
    let create = |driver, device: &mut _, context: &mut _| {
        D3D11CreateDevice(
            None::<&IDXGIAdapter>,
            driver,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(device),
            None,
            Some(context),
        )
    };
    create(D3D_DRIVER_TYPE_HARDWARE, &mut device, &mut context)
        .or_else(|_| create(D3D_DRIVER_TYPE_WARP, &mut device, &mut context))?;
    let device = device.ok_or_else(windows::core::Error::from_thread)?;
    let context = context.ok_or_else(windows::core::Error::from_thread)?;
    let dxgi: IDXGIDevice = device.cast()?;
    let winrt: IDirect3DDevice = CreateDirect3D11DeviceFromDXGIDevice(&dxgi)?.cast()?;
    let pool = Pool(Direct3D11CaptureFramePool::CreateFreeThreaded(
        &winrt,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        2,
        size,
    )?);
    let session = Session(pool.0.CreateCaptureSession(&item)?);
    // Older supported Windows builds may lack this optional property.
    let _ = session.0.SetIsCursorCaptureEnabled(false);
    session.0.StartCapture()?;
    Ok((device, context, pool, session))
}

unsafe fn read_frame(
    window: HWND,
    client: Rect,
    device: &ID3D11Device,
    context: &ID3D11DeviceContext,
    frame: &Direct3D11CaptureFrame,
) -> windows::core::Result<Image> {
    let size = frame.ContentSize()?;
    let (x, y) = if size.Width == client.width && size.Height == client.height {
        (0, 0)
    } else {
        let mut bounds = RECT::default();
        DwmGetWindowAttribute(
            window,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            (&mut bounds as *mut RECT).cast(),
            std::mem::size_of::<RECT>() as u32,
        )?;
        if size.Width != bounds.right - bounds.left || size.Height != bounds.bottom - bounds.top {
            return Err(windows::core::Error::new(
                windows::Win32::Foundation::E_FAIL,
                "Cloud window changed size during capture",
            ));
        }
        (client.x - bounds.left, client.y - bounds.top)
    };
    let access: IDirect3DDxgiInterfaceAccess = frame.Surface()?.cast()?;
    let texture: ID3D11Texture2D = access.GetInterface()?;
    read_texture(device, context, &texture, Rect { x, y, ..client })
}
