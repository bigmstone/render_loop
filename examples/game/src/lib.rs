mod pipeline;
mod vertices;

use std::{f32::consts, mem};

use rl_graphics::{object::Vertex, wgpu, wgpu::util::DeviceExt, Graphics, Renderable};

use {pipeline::create_pipeline, vertices::create_vertices};

pub struct Cube {
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    uniform_buf: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    index_count: usize,
}

impl Cube {
    pub fn new(graphics: &Graphics) -> Self {
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

        let (render_pipeline, bind_group) = create_pipeline(
            graphics,
            vertex_buffers.as_slice(),
            &uniform_buf,
            &texture_view,
        );

        let index_count = object.indices.len();
        Self {
            vertex_buf,
            index_buf,
            uniform_buf,
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

    fn resize(&mut self, width: u32, height: u32, queue: &wgpu::Queue) {
        let mx_total = generate_matrix(width as f32 / height as f32);
        let mx_ref: &[[f32; 4]; 4] = mx_total.as_ref();
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(mx_ref));
    }
}

fn create_texels(size: usize) -> Vec<u8> {
    (0..size * size)
        .map(|id| {
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
