use windows::Win32::Foundation::{HMODULE, HWND};
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_10_0, D3D_FEATURE_LEVEL_11_0,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device,
    ID3D11DeviceContext, ID3D11Texture2D,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, DXGI_PRESENT, DXGI_SCALING_STRETCH,
    DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FLAG, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
    DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIDevice, IDXGIFactory2, IDXGISwapChain1,
};
use windows::core::Interface;

pub struct CompositionRenderer {
    context: ID3D11DeviceContext,
    swap_chain: IDXGISwapChain1,
    composition_device: IDCompositionDevice,
    _target: IDCompositionTarget,
    _visual: IDCompositionVisual,
    width: u32,
    height: u32,
}

impl CompositionRenderer {
    pub fn new(hwnd: *mut core::ffi::c_void, width: u32, height: u32) -> Result<Self, String> {
        // SAFETY: 所有 COM 物件皆限制在建立它們的 helper GUI thread 使用。
        unsafe { Self::new_inner(hwnd, width, height) }
    }

    unsafe fn new_inner(
        hwnd: *mut core::ffi::c_void,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        let mut device = None;
        let mut context = None;
        let feature_levels = [D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_10_0];
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&feature_levels),
                D3D11_SDK_VERSION,
                Some(&raw mut device),
                None,
                Some(&raw mut context),
            )
        }
        .map_err(|error| format!("建立 D3D11 device 失敗：{error}"))?;
        let device: ID3D11Device = device.ok_or("D3D11 device 為空")?;
        let context = context.ok_or("D3D11 context 為空")?;
        let factory: IDXGIFactory2 = unsafe { CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0)) }
            .map_err(|error| format!("建立 DXGI factory 失敗：{error}"))?;
        let desc = swap_chain_description(width, height);
        let swap_chain =
            unsafe { factory.CreateSwapChainForComposition(&device, &raw const desc, None) }
                .map_err(|error| format!("建立 composition swap chain 失敗：{error}"))?;

        let dxgi_device: IDXGIDevice = device
            .cast()
            .map_err(|error| format!("取得 DXGI device 失敗：{error}"))?;
        let composition_device: IDCompositionDevice =
            unsafe { DCompositionCreateDevice(&dxgi_device) }
                .map_err(|error| format!("建立 DirectComposition device 失敗：{error}"))?;
        let target = unsafe { composition_device.CreateTargetForHwnd(HWND(hwnd), false) }
            .map_err(|error| format!("建立 DirectComposition target 失敗：{error}"))?;
        let visual = unsafe { composition_device.CreateVisual() }
            .map_err(|error| format!("建立 DirectComposition visual 失敗：{error}"))?;
        unsafe {
            visual
                .SetContent(&swap_chain)
                .map_err(|error| format!("綁定 composition swap chain 失敗：{error}"))?;
            target
                .SetRoot(&visual)
                .map_err(|error| format!("設定 DirectComposition root 失敗：{error}"))?;
            composition_device
                .Commit()
                .map_err(|error| format!("提交 DirectComposition tree 失敗：{error}"))?;
        }

        Ok(Self {
            context,
            swap_chain,
            composition_device,
            _target: target,
            _visual: visual,
            width,
            height,
        })
    }

    pub fn render(&mut self, pixels: &[u8], width: u32, height: u32) -> Result<(), String> {
        if width == 0 || height == 0 {
            return Ok(());
        }
        if pixels.len() != width as usize * height as usize * 4 {
            return Err("DirectComposition 像素緩衝區尺寸不正確".to_string());
        }
        if self.width != width || self.height != height {
            unsafe {
                self.swap_chain.ResizeBuffers(
                    0,
                    width,
                    height,
                    DXGI_FORMAT_B8G8R8A8_UNORM,
                    DXGI_SWAP_CHAIN_FLAG(0),
                )
            }
            .map_err(|error| format!("調整 composition swap chain 失敗：{error}"))?;
            self.width = width;
            self.height = height;
        }
        let texture: ID3D11Texture2D = unsafe { self.swap_chain.GetBuffer(0) }
            .map_err(|error| format!("取得 composition back buffer 失敗：{error}"))?;
        unsafe {
            self.context
                .UpdateSubresource(&texture, 0, None, pixels.as_ptr().cast(), width * 4, 0);
            self.swap_chain
                .Present(1, DXGI_PRESENT(0))
                .ok()
                .map_err(|error| format!("呈現 composition swap chain 失敗：{error}"))?;
            self.composition_device
                .Commit()
                .map_err(|error| format!("提交 DirectComposition 畫面失敗：{error}"))?;
        }
        Ok(())
    }
}

fn swap_chain_description(width: u32, height: u32) -> DXGI_SWAP_CHAIN_DESC1 {
    DXGI_SWAP_CHAIN_DESC1 {
        Width: width,
        Height: height,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        Stereo: false.into(),
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 2,
        Scaling: DXGI_SCALING_STRETCH,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
        AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
        Flags: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composition_swap_chain_uses_bgra_premultiplied_double_buffering() {
        let description = swap_chain_description(320, 40);

        assert_eq!(description.Width, 320);
        assert_eq!(description.Height, 40);
        assert_eq!(description.Format, DXGI_FORMAT_B8G8R8A8_UNORM);
        assert_eq!(description.BufferCount, 2);
        assert_eq!(description.SwapEffect, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL);
        assert_eq!(description.AlphaMode, DXGI_ALPHA_MODE_PREMULTIPLIED);
    }
}
