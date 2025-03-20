use std::collections::HashMap;

use ddnet_base::StrRef;
use pollster::FutureExt;
use wgpu::util::DeviceExt;

mod window_handle;

#[cxx::bridge]
mod ffi {
    extern "C++" {
        include!("base/rust.h");

        type StrRef<'a> = ddnet_base::StrRef<'a>;
    }
    extern "Rust" {
        fn BackendWgpuGreetings(names: &[StrRef<'_>]);

        type RustWgpuBackend<'a>;
        fn init_rust_wgpu_backend() -> Box<RustWgpuBackend<'static>>;
        unsafe fn init_window(&mut self, window: *mut u8, width: u32, height: u32);
        fn create_texture(
            &mut self,
            slot: i32,
            bytes_per_pixel: u32,
            flags: i32,
            width: u32,
            height: u32,
            data: &[u8],
        );
        fn update_texture(
            &mut self,
            slot: i32,
            x: u32,
            y: u32,
            width: u32,
            height: u32,
            data: &[u8],
        );
        fn destroy_texture(&mut self, slot: i32);
        fn clear(&mut self, r: f64, g: f64, b: f64, a: f64);
    }
}

/// Example for a Rust function callable from C++.
///
/// Prints a greeting from the wgpu backend module to stdout, mentioning the
/// passed names.
#[allow(non_snake_case)]
fn BackendWgpuGreetings(names: &[StrRef<'_>]) {
    if names.is_empty() {
        println!("Hello!");
    } else {
        let names: Vec<_> = names.into_iter().map(StrRef::to_str).collect();
        println!("Hello, {}!", names.join(", "));
    }
}

struct RustWgpuBackend<'a> {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: Option<wgpu::Surface<'a>>,
    /// Maps slot to texture
    textures_2d: HashMap<i32, wgpu::Texture>,
}

fn init_rust_wgpu_backend() -> Box<RustWgpuBackend<'static>> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })
        .block_on()
        .unwrap();
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )
        .block_on()
        .unwrap();

    Box::new(RustWgpuBackend {
        instance,
        adapter,
        device,
        queue,
        surface: None,
        textures_2d: HashMap::new(),
    })
}

impl RustWgpuBackend<'_> {
    fn init_window(&mut self, window: *mut u8, width: u32, height: u32) {
        let surface = unsafe {
            let window = window as *mut window_handle::SDL_SysWMinfo;
            let window = *window;
            self.instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window).unwrap())
                .unwrap()
        };
        surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: wgpu::TextureFormat::Bgra8Unorm,
                width,
                height,
                present_mode: wgpu::PresentMode::AutoVsync,
                desired_maximum_frame_latency: 0,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
            },
        );
        self.surface = Some(surface);
    }
    /// `bytes_per_pixel` determine the texture format. 4 -> Rgba, 1 -> R.
    fn create_texture(
        &mut self,
        slot: i32,
        bytes_per_pixel: u32,
        _flags: i32,
        width: u32,
        height: u32,
        data: &[u8],
    ) {
        let format = match bytes_per_pixel {
            1 => wgpu::TextureFormat::R8Unorm,
            4 => wgpu::TextureFormat::Rgba8Unorm,
            _ => panic!("Invalid amount of bytes per pixel: {bytes_per_pixel}"),
        };
        let expected_size = bytes_per_pixel as usize * width as usize * height as usize;
        assert_eq!(data.len(), expected_size);
        let texture = self.device.create_texture_with_data(
            &self.queue,
            &wgpu::TextureDescriptor {
                label: Some(&format!("Texture {slot}")),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            data,
        );
        self.textures_2d.insert(slot, texture);
    }

    fn update_texture(&mut self, slot: i32, x: u32, y: u32, width: u32, height: u32, data: &[u8]) {
        let Some(texture) = self.textures_2d.get(&slot) else {
            panic!("Updating non-existing texture");
        };
        let bytes_per_pixel = texture.format().components();
        let expected_size = bytes_per_pixel as usize * width as usize * height as usize;
        assert_eq!(data.len(), expected_size);
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_pixel as u32 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    fn destroy_texture(&mut self, slot: i32) {
        if self.textures_2d.remove(&slot).is_none() {
            panic!("Destroyed non-existing textures ({slot})");
        }
    }

    fn clear(&self, r: f64, g: f64, b: f64, a: f64) {
        let Some(surface) = &self.surface else {
            panic!("Clear without surface");
        };
        let frame = surface.get_current_texture().unwrap();
        let mut command_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Normal Frame"),
                });
        let render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Just clear with color"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default()),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        std::mem::drop(render_pass);
        self.queue.submit([command_encoder.finish()]);
        frame.present();
    }
}
