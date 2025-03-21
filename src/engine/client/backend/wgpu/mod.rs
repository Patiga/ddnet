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
        fn resize_event(&mut self, width: u32, height: u32);
        fn swap(&mut self);
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
    reconfigure_surface: bool,
    surface_configuration: wgpu::SurfaceConfiguration,
    /// Maps slot to texture
    textures_2d: HashMap<i32, wgpu::Texture>,
    clear_color: wgpu::Color,
    /// Initialized only after init call
    surface: Option<wgpu::Surface<'a>>,
    surface_texture: Option<wgpu::SurfaceTexture>,
    /// Any mutation of this outside of the `swap` method will fail.
    /// The lifetime of the render_pass is "forgotten".
    /// We have to manually ensure this constraint.
    command_encoder: Option<wgpu::CommandEncoder>,
    render_pass: Option<wgpu::RenderPass<'static>>,
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
    let surface_configuration = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8Unorm,
        width: 0,
        height: 0,
        present_mode: wgpu::PresentMode::AutoVsync,
        desired_maximum_frame_latency: 0,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
    };

    Box::new(RustWgpuBackend {
        instance,
        adapter,
        device,
        queue,
        reconfigure_surface: false,
        surface_configuration,
        textures_2d: HashMap::new(),
        clear_color: wgpu::Color::RED,
        surface: None,
        surface_texture: None,
        command_encoder: None,
        render_pass: None,
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
        self.surface_configuration.width = width;
        self.surface_configuration.height = height;
        surface.configure(&self.device, &self.surface_configuration);
        self.surface = Some(surface);
    }

    fn resize_event(&mut self, width: u32, height: u32) {
        self.reconfigure_surface = true;
        self.surface_configuration.width = width;
        self.surface_configuration.height = height;
    }

    /// Finishes the last frame, and starts the new frame.
    pub fn swap(&mut self) {
        std::mem::drop(self.render_pass.take());
        if let Some(encoder) = self.command_encoder.take() {
            self.queue.submit([encoder.finish()]);
        }
        if let Some(surface_texture) = self.surface_texture.take() {
            surface_texture.present();
        }
        let Some(surface) = &self.surface else {
            panic!("Swap without surface");
        };
        if self.reconfigure_surface {
            surface.configure(&self.device, &self.surface_configuration);
        }
        let surface_texture = surface.get_current_texture().unwrap();
        let mut command_encoder = self.device.create_command_encoder(&Default::default());
        let render_pass = command_encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_texture.texture.create_view(&Default::default()),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            })
            .forget_lifetime();
        self.surface_texture = Some(surface_texture);
        self.command_encoder = Some(command_encoder);
        self.render_pass = Some(render_pass);
    }

    fn clear(&mut self, r: f64, g: f64, b: f64, a: f64) {
        self.clear_color = wgpu::Color { r, g, b, a };
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
}
