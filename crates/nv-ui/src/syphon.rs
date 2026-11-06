//! Syphon output, on a Mac: the visuals shared live with Resolume, VDMX,
//! MadMapper, OBS or any other Syphon client, as a source they can mix.
//!
//! Each frame the stage -- the visualization alone, never the controls -- is
//! copied on the GPU into a texture of our own and handed to Syphon, which
//! shares it with the other app through an IOSurface. Nothing crosses the
//! CPU and nothing is encoded. While no client is connected, nothing at all
//! is copied.
//!
//! wgpu 22 does not hand out the Metal texture behind a wgpu texture, so it
//! goes the other way: the Metal texture is made here, on wgpu's own Metal
//! device, and wrapped into wgpu, which copies the stage into it.

use objc2::runtime::ProtocolObject;
use objc2_metal::MTLTexture;
use wgpu::hal::api::Metal;

/// The name other apps list this source under.
pub const SERVER_NAME: &str = "NeanderthalVJ";

pub struct Output {
    server: syphon_rs::Server,
    target: Option<Target>,
}

/// The texture the stage is copied into: the Metal one Syphon reads, and
/// the same memory as a wgpu texture to copy into.
struct Target {
    raw: metal::Texture,
    texture: wgpu::Texture,
    size: (u32, u32),
    format: wgpu::TextureFormat,
}

impl Output {
    /// Start sharing. Fails if Syphon is unavailable, or if it would run on
    /// a different GPU from the visuals (a Mac with two GPUs).
    pub fn new(device: &wgpu::Device) -> Result<Self, String> {
        let server =
            syphon_rs::Server::new(SERVER_NAME, 16, 16).map_err(|e| format!("Syphon: {e}"))?;
        let theirs = server.metal_device() as *const _ as *const std::ffi::c_void;
        // SAFETY: on a Mac the device is a Metal one; only its address is read.
        let same = unsafe {
            device.as_hal::<Metal, _, _>(|d| {
                d.map(|d| {
                    let ours: &metal::DeviceRef = &d.raw_device().lock();
                    ours as *const metal::DeviceRef as *const std::ffi::c_void == theirs
                })
            })
        }
        .flatten()
        .unwrap_or(false);
        if !same {
            return Err("Syphon: it would run on a different GPU from the visuals".into());
        }
        Ok(Self { server, target: None })
    }

    /// Whether another app is receiving the frames.
    pub fn has_clients(&self) -> bool {
        self.server.has_clients()
    }

    /// Copy `rect` of `source` (x, y, width, height in pixels) and publish it.
    /// `source` must allow `COPY_SRC`.
    pub fn publish(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &wgpu::Texture,
        rect: (u32, u32, u32, u32),
    ) -> Result<(), String> {
        let (x, y, w, h) = rect;
        if w == 0 || h == 0 {
            return Ok(());
        }
        let format = source.format();
        if self.target.as_ref().map(|t| (t.size, t.format)) != Some(((w, h), format)) {
            self.target = Some(Target::new(device, format, w, h)?);
        }
        let target = self.target.as_ref().expect("made above");

        let mut enc = device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("syphon") });
        enc.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: source,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: &target.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        let done = queue.submit([enc.finish()]);
        // Syphon reads on its own command queue: the copy must have finished
        device.poll(wgpu::Maintain::WaitForSubmissionIndex(done));

        // SAFETY: a `metal::TextureRef` is the MTLTexture object itself, so
        // its address is the object's; `target.raw` keeps it alive.
        let texture: &ProtocolObject<dyn MTLTexture> = unsafe {
            &*(&*target.raw as *const metal::TextureRef as *const ProtocolObject<dyn MTLTexture>)
        };
        self.server.publish_metal_texture(texture, true).map_err(|e| format!("Syphon: {e}"))
    }
}

impl Target {
    fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        w: u32,
        h: u32,
    ) -> Result<Self, String> {
        use wgpu::TextureFormat as F;
        let pixel_format = match format {
            F::Bgra8Unorm => metal::MTLPixelFormat::BGRA8Unorm,
            F::Bgra8UnormSrgb => metal::MTLPixelFormat::BGRA8Unorm_sRGB,
            F::Rgba8Unorm => metal::MTLPixelFormat::RGBA8Unorm,
            F::Rgba8UnormSrgb => metal::MTLPixelFormat::RGBA8Unorm_sRGB,
            F::Rgba16Float => metal::MTLPixelFormat::RGBA16Float,
            other => return Err(format!("Syphon: the window's {other:?} frames cannot be shared")),
        };
        // SAFETY: the texture is made on wgpu's own Metal device, with the
        // size and format it is then wrapped into wgpu with.
        let raw = unsafe {
            device.as_hal::<Metal, _, _>(|d| {
                d.map(|d| {
                    let desc = metal::TextureDescriptor::new();
                    desc.set_texture_type(metal::MTLTextureType::D2);
                    desc.set_pixel_format(pixel_format);
                    desc.set_width(w as u64);
                    desc.set_height(h as u64);
                    desc.set_storage_mode(metal::MTLStorageMode::Private);
                    desc.set_usage(
                        metal::MTLTextureUsage::ShaderRead | metal::MTLTextureUsage::RenderTarget,
                    );
                    d.raw_device().lock().new_texture(&desc)
                })
            })
        }
        .flatten()
        .ok_or("Syphon: no Metal device")?;

        let size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
        // SAFETY: `raw` is a live 2D texture of this size and format on the
        // device it is wrapped for.
        let texture = unsafe {
            let hal = wgpu::hal::metal::Device::texture_from_raw(
                raw.clone(),
                format,
                metal::MTLTextureType::D2,
                1,
                1,
                wgpu::hal::CopyExtent { width: w, height: h, depth: 1 },
            );
            device.create_texture_from_hal::<Metal>(
                hal,
                &wgpu::TextureDescriptor {
                    label: Some("syphon frame"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
            )
        };
        Ok(Self { raw, texture, size: (w, h), format })
    }
}
