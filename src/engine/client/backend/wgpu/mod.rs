use std::collections::HashMap;

use ddnet_base::StrRef;
use pollster::FutureExt;
use wgpu::util::DeviceExt;

#[cxx::bridge]
mod ffi {
    extern "C++" {
        include!("base/rust.h");

        type StrRef<'a> = ddnet_base::StrRef<'a>;
    }
    extern "Rust" {
        fn BackendWgpuGreetings(names: &[StrRef<'_>]);

        type RustWgpuBackend;
        fn init_rust_wgpu_backend() -> Box<RustWgpuBackend>;
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

struct RustWgpuBackend {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,

    /// Maps slot to texture
    textures_2d: HashMap<i32, wgpu::Texture>,
}

fn init_rust_wgpu_backend() -> Box<RustWgpuBackend> {
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
        textures_2d: HashMap::new(),
    })
}

impl RustWgpuBackend {
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
