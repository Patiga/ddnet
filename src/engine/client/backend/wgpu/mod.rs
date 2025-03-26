use core::f32;
use std::{collections::HashMap, num::NonZero};

use ddnet_base::StrRef;
use pollster::FutureExt;
use wgpu::{util::DeviceExt, BufferSlice};

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
        fn update_viewport(&mut self, x: i32, y: i32, w: u32, h: u32, by_resize: bool);
        fn swap(&mut self);
        fn render(
            &mut self,
            blend_mode: i32,
            wrap_mode: i32,
            texture: i32,
            screen_tl_x: f32,
            screen_tl_y: f32,
            screen_br_x: f32,
            screen_br_y: f32,
            clipping: bool,
            clip_x: u32,
            clip_y: u32,
            clip_w: u32,
            clip_h: u32,
            primitive: u32,
            primitive_count: u32,
            vertices: &[u8],
        );
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
    pipelines: Pipelines,
    samplers: Samplers,
    state_matrix: StateMatrix,
    bind_groups: BindGroups,
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    /// Maps slot to texture
    textures_2d: HashMap<i32, wgpu::Texture>,
    clear_color: wgpu::Color,
    /// All following fields are initialized only upon Init call.
    /// The surface_texture needs to be dropped before the surface!
    surface_texture: Option<wgpu::SurfaceTexture>,
    surface: Option<wgpu::Surface<'a>>,
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
    let adapter_limits = adapter.limits();
    let required_limits = wgpu::Limits {
        max_texture_dimension_2d: adapter_limits.max_texture_dimension_2d,
        ..wgpu::Limits::downlevel_webgl2_defaults()
    };
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )
        .block_on()
        .unwrap();
    let format = wgpu::TextureFormat::Bgra8Unorm;
    let surface_configuration = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: 0,
        height: 0,
        present_mode: wgpu::PresentMode::AutoNoVsync,
        desired_maximum_frame_latency: 0,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
    };
    let samplers = Samplers::new(&device);
    let state_matrix = StateMatrix::new(0., 0., 1., 1., &device);
    let bind_groups = BindGroups::new(&samplers, &state_matrix, &device);
    let pipelines = Pipelines::new(&device, &bind_groups, format);
    let vertex_buffer = VertexBuffer::new(&device);
    let index_buffer = IndexBuffer::new(&device);

    let mut backend = RustWgpuBackend {
        instance,
        adapter,
        device,
        queue,
        reconfigure_surface: false,
        surface_configuration,
        pipelines,
        samplers,
        state_matrix,
        bind_groups,
        vertex_buffer,
        index_buffer,
        textures_2d: HashMap::new(),
        clear_color: wgpu::Color::RED,
        surface_texture: None,
        surface: None,
        command_encoder: None,
        render_pass: None,
    };
    backend.create_texture(-1, 4, 0, 1, 1, &[255, 255, 255, 255]);
    Box::new(backend)
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

    fn update_viewport(&mut self, _x: i32, _y: i32, w: u32, h: u32, by_resize: bool) {
        if by_resize {
            self.reconfigure_surface = true;
            self.surface_configuration.width = w;
            self.surface_configuration.height = h;
        }
        /* This code caused crashes upon entering full screen and should not be needed.
        if let Some(render_pass) = &mut self.render_pass {
            render_pass.set_viewport(x as f32, y as f32, w as f32, h as f32, 0., 1.);
        };
        */
    }

    /// Finishes the last frame, and starts the new frame.
    pub fn swap(&mut self) {
        self.vertex_buffer.finalize_buffers(&self.queue);
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
            self.reconfigure_surface = false;
            surface.configure(&self.device, &self.surface_configuration);
        }
        let surface_texture = surface.get_current_texture().unwrap();
        let mut command_encoder = self.device.create_command_encoder(&Default::default());
        let mut render_pass = command_encoder
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
        render_pass.set_pipeline(&self.pipelines.pipeline);
        render_pass.set_index_buffer(
            self.index_buffer.buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        self.surface_texture = Some(surface_texture);
        self.command_encoder = Some(command_encoder);
        self.render_pass = Some(render_pass);
    }

    fn clear(&mut self, r: f64, g: f64, b: f64, a: f64) {
        self.clear_color = wgpu::Color { r, g, b, a };
    }

    #[allow(clippy::too_many_arguments)]
    fn render(
        &mut self,
        _blend_mode: i32,
        wrap_mode: i32,
        texture: i32,
        screen_tl_x: f32,
        screen_tl_y: f32,
        screen_br_x: f32,
        screen_br_y: f32,
        clipping: bool,
        clip_x: u32,
        clip_y: u32,
        clip_w: u32,
        clip_h: u32,
        primitive: u32,
        primitive_count: u32,
        vertices: &[u8],
    ) {
        #[derive(Debug)]
        enum Primitive {
            Lines,
            Quads,
            Triangles,
        }
        let primitive = match primitive {
            0 => panic!("Invalid primitive"),
            1 => Primitive::Lines,
            2 => Primitive::Quads,
            3 => Primitive::Triangles,
            _ => panic!("Unknown primitive"),
        };
        let address_mode = match wrap_mode {
            0 => wgpu::AddressMode::Repeat,
            1 => wgpu::AddressMode::ClampToEdge,
            _ => panic!("Unknown address mode"),
        };
        self.state_matrix.update_bind_group(
            screen_tl_x,
            screen_tl_y,
            screen_br_x,
            screen_br_y,
            &mut self.bind_groups,
            &self.device,
        );
        let Some(render_pass) = &mut self.render_pass else {
            panic!("Render call without render pass");
        };
        let Some(surface_texture) = &self.surface_texture else {
            panic!("Render call without surface texture");
        };
        let target_width = surface_texture.texture.width();
        let target_height = surface_texture.texture.height();
        if clipping {
            // Mirror the clipping vertically for wgpu
            let clip_y = surface_texture.texture.height() - clip_y - clip_h;
            let clipped_clip_w = clip_w.min(target_width - clip_y);
            let clipped_clip_h = clip_h.min(target_height - clip_y);
            // Might need to be clipped to surface texture size
            render_pass.set_scissor_rect(clip_x, clip_y, clipped_clip_w, clipped_clip_h);
        } else {
            render_pass.set_scissor_rect(0, 0, target_width, target_height);
        }
        render_pass.set_bind_group(0, Some(self.bind_groups.get_sampler(address_mode)), &[]);
        render_pass.set_bind_group(1, Some(&self.bind_groups.last_state_matrix), &[]);
        render_pass.set_bind_group(2, Some(self.bind_groups.get_texture(texture).unwrap()), &[]);
        let vertex_slice = self
            .vertex_buffer
            .reserve_vertices(vertices, &self.device, &self.queue);
        render_pass.set_vertex_buffer(0, vertex_slice);
        if !self
            .index_buffer
            .ensure_length(primitive_count as usize, &self.device)
        {
            render_pass.set_index_buffer(
                self.index_buffer.buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
        }
        match primitive {
            Primitive::Lines => {}
            Primitive::Quads => render_pass.draw_indexed(0..primitive_count * 6, 0, 0..1),
            Primitive::Triangles => render_pass.draw(0..primitive_count * 3, 0..1),
        }
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
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            data,
        );
        self.bind_groups.add_texture(slot, &texture, &self.device);
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

struct Samplers {
    repeat: wgpu::Sampler,
    clamp: wgpu::Sampler,
}

struct Pipelines {
    pipeline: wgpu::RenderPipeline,
}

impl Pipelines {
    fn new(device: &wgpu::Device, bind_groups: &BindGroups, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[
                &bind_groups.sampler_bind_group_layout,
                &bind_groups.state_matrix_bind_group_layout,
                &bind_groups.texture_bind_group_layout,
            ],
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("sprites_textured.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 5 * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2, 1 => Float32x2, 2 => Unorm8x4
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::all(),
                })],
            }),
            multiview: None,
            cache: None,
        });
        Self { pipeline }
    }
}

impl Samplers {
    fn new(device: &wgpu::Device) -> Self {
        let [repeat, clamp] =
            [wgpu::AddressMode::Repeat, wgpu::AddressMode::ClampToEdge].map(|address_mode| {
                device.create_sampler(&wgpu::SamplerDescriptor {
                    address_mode_u: address_mode,
                    address_mode_v: address_mode,
                    address_mode_w: address_mode,
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    mipmap_filter: wgpu::FilterMode::Linear,
                    ..Default::default()
                })
            });
        Self { repeat, clamp }
    }

    fn get(&self, address_mode: wgpu::AddressMode) -> &wgpu::Sampler {
        match address_mode {
            wgpu::AddressMode::ClampToEdge => &self.clamp,
            wgpu::AddressMode::Repeat => &self.repeat,
            _ => panic!("Invalid sampler address mode"),
        }
    }
}

struct BindGroups {
    sampler_bind_group_layout: wgpu::BindGroupLayout,
    state_matrix_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler_repeat: wgpu::BindGroup,
    sampler_clamp: wgpu::BindGroup,
    last_state_matrix: wgpu::BindGroup,
    textures: HashMap<i32, wgpu::BindGroup>,
}

impl BindGroups {
    fn new(samplers: &Samplers, state_matrix: &StateMatrix, device: &wgpu::Device) -> Self {
        let sampler_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Sampler"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                }],
            });
        let state_matrix_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("State Matrix"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(NonZero::new(2 * 4 * 4).unwrap()),
                    },
                    count: None,
                }],
            });
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            });
        let [sampler_repeat, sampler_clamp] =
            [wgpu::AddressMode::Repeat, wgpu::AddressMode::ClampToEdge].map(|address_mode| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Sampler Bind Group"),
                    layout: &sampler_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(samplers.get(address_mode)),
                    }],
                })
            });
        let last_state_matrix =
            Self::state_matrix_bind_group(state_matrix, &state_matrix_bind_group_layout, device);
        Self {
            sampler_bind_group_layout,
            state_matrix_bind_group_layout,
            texture_bind_group_layout,
            sampler_repeat,
            sampler_clamp,
            last_state_matrix,
            textures: HashMap::default(),
        }
    }

    fn get_sampler(&mut self, address_mode: wgpu::AddressMode) -> &wgpu::BindGroup {
        match address_mode {
            wgpu::AddressMode::Repeat => &self.sampler_repeat,
            wgpu::AddressMode::ClampToEdge => &self.sampler_clamp,
            _ => panic!("Invalid address mode"),
        }
    }

    fn state_matrix_bind_group(
        state_matrix: &StateMatrix,
        layout: &wgpu::BindGroupLayout,
        device: &wgpu::Device,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &state_matrix.buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        })
    }

    fn update_state_matrix(&mut self, state_matrix: &StateMatrix, device: &wgpu::Device) {
        self.last_state_matrix = Self::state_matrix_bind_group(
            state_matrix,
            &self.state_matrix_bind_group_layout,
            device,
        )
    }

    fn add_texture(&mut self, slot: i32, texture: &wgpu::Texture, device: &wgpu::Device) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.texture_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(
                    &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            }],
        });
        self.textures.insert(slot, bind_group);
    }

    fn get_texture(&mut self, slot: i32) -> Option<&wgpu::BindGroup> {
        self.textures.get(&slot)
    }
}

struct StateMatrix {
    tl_x: f32,
    tl_y: f32,
    br_x: f32,
    br_y: f32,
    buffer: wgpu::Buffer,
}

impl StateMatrix {
    fn new(tl_x: f32, tl_y: f32, br_x: f32, br_y: f32, device: &wgpu::Device) -> Self {
        let state_matrix: [[f32; 2]; 4] = [
            [2. / (br_x - tl_x), 0.],
            [0., (2. / (tl_y - br_y))],
            [0., 0.],
            [
                -((br_x + tl_x) / (br_x - tl_x)),
                -((tl_y + br_y) / (tl_y - br_y)),
            ],
        ];
        Self {
            tl_x: 0.,
            tl_y: 0.,
            br_x: 0.,
            br_y: 0.,
            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("State matrix"),
                contents: bytemuck::cast_slice(&state_matrix),
                usage: wgpu::BufferUsages::UNIFORM,
            }),
        }
    }

    fn update_bind_group<'a>(
        &mut self,
        tl_x: f32,
        tl_y: f32,
        br_x: f32,
        br_y: f32,
        bind_groups: &'a mut BindGroups,
        device: &wgpu::Device,
    ) -> &'a wgpu::BindGroup {
        if self.tl_x == tl_x && self.tl_y == tl_y && self.br_x == br_x && self.br_y == br_y {
            &bind_groups.last_state_matrix
        } else {
            *self = Self::new(tl_x, tl_y, br_x, br_y, device);
            bind_groups.update_state_matrix(self, device);
            &bind_groups.last_state_matrix
        }
    }
}

struct VertexBuffer {
    cache: Vec<u8>,
    buffer: wgpu::Buffer,
}

impl VertexBuffer {
    fn new(device: &wgpu::Device) -> Self {
        let cache = Vec::with_capacity(1024);
        let buffer = Self::create_buffer(cache.capacity(), device);
        Self { cache, buffer }
    }

    fn reserve_vertices(
        &mut self,
        vertex_data: &[u8],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> BufferSlice {
        if self.cache.capacity() - self.cache.len() >= vertex_data.len() {
            // Enough space
            let offset = self.cache.len();
            let cap_before = self.cache.capacity();
            self.cache.extend_from_slice(vertex_data);
            assert_eq!(cap_before, self.cache.capacity());
            self.buffer
                .slice(offset as u64..(offset + vertex_data.len()) as u64)
        } else {
            queue.write_buffer(&self.buffer, 0, self.cache.as_slice());
            let offset = self.cache.len();
            self.cache.extend_from_slice(vertex_data);
            let new_capacity = self.cache.capacity();
            self.buffer = Self::create_buffer(new_capacity, device);
            self.buffer
                .slice(offset as u64..(offset + vertex_data.len()) as u64)
        }
    }

    fn create_buffer(cap: usize, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: cap as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn finalize_buffers(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, self.cache.as_slice());
        self.cache.clear();
    }
}

struct IndexBuffer {
    len: usize,
    buffer: wgpu::Buffer,
}

impl IndexBuffer {
    fn new(device: &wgpu::Device) -> Self {
        let len = 8;
        Self {
            len,
            buffer: Self::buffer_for_len(8, device),
        }
    }
    /// Returns false, if the buffer was too small and was remade.
    fn ensure_length(&mut self, len: usize, device: &wgpu::Device) -> bool {
        if self.len < len {
            self.len = len.next_power_of_two();
            self.buffer = Self::buffer_for_len(self.len, device);
            false
        } else {
            true
        }
    }

    fn buffer_for_len(len: usize, device: &wgpu::Device) -> wgpu::Buffer {
        let buffer: Vec<u32> = (0..len as u32)
            .flat_map(|i| [0, 1, 2, 0, 3, 2].map(|x| x + i * 4))
            .collect();
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: bytemuck::cast_slice(buffer.as_slice()),
            usage: wgpu::BufferUsages::INDEX,
        })
    }
}
