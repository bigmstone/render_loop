use std::{borrow::Cow, f32::consts, mem, sync::Arc};

use wgpu::util::DeviceExt;
use winit::event_loop::EventLoop;

use rl_graphics::{
    object::{Object, Vertex},
    Graphics, Renderable,
};

fn vertex(pos: [i8; 3], tc: [i8; 2]) -> Vertex {
    Vertex {
        _pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32, 1.0],
        _tex_coord: [tc[0] as f32, tc[1] as f32],
    }
}

fn create_vertices() -> Object {
    let vertex_data = [
        // top (0, 0, 1)
        vertex([-1, -1, 1], [0, 0]),
        vertex([1, -1, 1], [1, 0]),
        vertex([1, 1, 1], [1, 1]),
        vertex([-1, 1, 1], [0, 1]),
        // bottom (0, 0, -1)
        vertex([-1, 1, -1], [1, 0]),
        vertex([1, 1, -1], [0, 0]),
        vertex([1, -1, -1], [0, 1]),
        vertex([-1, -1, -1], [1, 1]),
        // right (1, 0, 0)
        vertex([1, -1, -1], [0, 0]),
        vertex([1, 1, -1], [1, 0]),
        vertex([1, 1, 1], [1, 1]),
        vertex([1, -1, 1], [0, 1]),
        // left (-1, 0, 0)
        vertex([-1, -1, 1], [1, 0]),
        vertex([-1, 1, 1], [0, 0]),
        vertex([-1, 1, -1], [0, 1]),
        vertex([-1, -1, -1], [1, 1]),
        // front (0, 1, 0)
        vertex([1, 1, -1], [1, 0]),
        vertex([-1, 1, -1], [0, 0]),
        vertex([-1, 1, 1], [0, 1]),
        vertex([1, 1, 1], [1, 1]),
        // back (0, -1, 0)
        vertex([1, -1, 1], [0, 0]),
        vertex([-1, -1, 1], [1, 0]),
        vertex([-1, -1, -1], [1, 1]),
        vertex([1, -1, -1], [0, 1]),
    ];

    let index_data: &[u16] = &[
        0, 1, 2, 2, 3, 0, // top
        4, 5, 6, 6, 7, 4, // bottom
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // front
        20, 21, 22, 22, 23, 20, // back
    ];

    Object {
        vertices: vertex_data.to_vec(),
        indices: index_data.to_vec(),
    }
}

fn create_texels(size: usize) -> Vec<u8> {
    (0..size * size)
        .map(|id| {
            // get high five for recognizing this ;)
            let cx = 3.0 * (id % size) as f32 / (size - 1) as f32 - 2.0;
            let cy = 2.0 * (id / size) as f32 / (size - 1) as f32 - 1.0;
            let (mut x, mut y, mut count) = (cx, cy, 0);
            while count < 0xFF && x * x + y * y < 4.0 {
                let old_x = x;
                x = x * x - y * y + cx;
                y = 2.0 * old_x * y + cy;
                count += 1;
            }
            count
        })
        .collect()
}

fn generate_matrix(aspect_ratio: f32) -> nalgebra::Matrix4<f32> {
    let projection =
        nalgebra::Perspective3::new(consts::FRAC_PI_4, aspect_ratio, 1.0, 10.0).into_inner();
    let view = nalgebra::Matrix4::look_at_rh(
        &nalgebra::Point3::new(1.5f32, -5.0, 3.0),
        &nalgebra::Point3::new(0.0, 0.0, 0.0),
        &nalgebra::Vector3::z(),
    );
    projection * view
}

struct Cube {
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    index_count: usize,
}

impl Cube {
    fn new(graphics: &Graphics) -> Self {
        let object = create_vertices();

        let vertex_buf = graphics
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&object.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buf = graphics
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&object.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        // Create pipeline layout
        let bind_group_layout =
            graphics
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: wgpu::BufferSize::new(64),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                sample_type: wgpu::TextureSampleType::Uint,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                    ],
                });
        let pipeline_layout =
            graphics
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        // Create the texture
        let size = 256u32;
        let texels = create_texels(size as usize);
        let texture_extent = wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        };
        let texture = graphics.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        graphics.queue.write_texture(
            texture.as_image_copy(),
            &texels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(size),
                rows_per_image: None,
            },
            texture_extent,
        );

        // Create other resources
        let mx_total =
            generate_matrix(graphics.config.width as f32 / graphics.config.height as f32);
        let mx_ref: &[[f32; 4]; 4] = mx_total.as_ref();
        let uniform_buf = graphics
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Uniform Buffer"),
                contents: bytemuck::cast_slice(mx_ref),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group
        let bind_group = graphics
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                ],
                label: None,
            });

        let shader = graphics
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
            });

        let vertex_size = mem::size_of::<Vertex>();
        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: vertex_size as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 4 * 4,
                    shader_location: 1,
                },
            ],
        }];

        let render_pipeline =
            graphics
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: None,
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: "vs_main",
                        compilation_options: Default::default(),
                        buffers: &vertex_buffers,
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fs_main",
                        compilation_options: Default::default(),
                        targets: &[Some(graphics.config.view_formats[0].into())],
                    }),
                    primitive: wgpu::PrimitiveState {
                        cull_mode: Some(wgpu::Face::Back),
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                });

        let index_count = object.indices.len();
        Self {
            vertex_buf,
            index_buf,
            render_pipeline,
            bind_group,
            index_count,
        }
    }
}

impl Renderable for Cube {
    fn render<'a>(&'a mut self, rpass: &mut wgpu::RenderPass<'a>, _queue: &wgpu::Queue) {
        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
        rpass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        rpass.pop_debug_group();
        rpass.insert_debug_marker("Draw!");
        rpass.draw_indexed(0..self.index_count as u32, 0, 0..1);
    }
}

pub fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        // We keep wgpu at Error level, as it's very noisy.
        .filter_module("wgpu_core", log::LevelFilter::Info)
        .filter_module("wgpu_hal", log::LevelFilter::Error)
        .filter_module("naga", log::LevelFilter::Error)
        .parse_default_env()
        .init();

    let event_loop = EventLoop::new().unwrap();
    let builder = winit::window::WindowBuilder::new();
    let window = Arc::new(builder.build(&event_loop).unwrap());
    let mut graphics = pollster::block_on(Graphics::new(window)).unwrap();

    let cube = Cube::new(&graphics);

    graphics.renderables.push(Box::new(cube));

    pollster::block_on(graphics.run(event_loop));
}
