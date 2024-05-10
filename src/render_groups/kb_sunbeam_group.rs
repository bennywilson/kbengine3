use wgpu::util::DeviceExt;

use crate::{kb_assets::*, kb_config::*, kb_game_object::*, kb_resource::*};

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct KbSunbeamUniform {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos:[f32; 4],
    pub camera_dir:[f32; 4],
    pub screen_dimensions: [f32; 4],
}

pub struct KbSunbeamRenderGroup {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub sunbeam_uniform: KbSunbeamUniform,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

impl KbSunbeamRenderGroup {
    pub async fn new(device_resources: &KbDeviceResources<'_>, asset_manager: &mut KbAssetManager) -> Self {
        let device = &device_resources.device;
        let surface_config = &device_resources.surface_config;

        // Post Process Pipeline
        let sunbeam_shader_handle = asset_manager.load_shader("/engine_assets/shaders/sunbeam_mask.wgsl", &device_resources).await;
        let sunbeam_shader = asset_manager.get_shader(&sunbeam_shader_handle);

        let sunbeam_uniform = KbSunbeamUniform {
            ..Default::default()
        };
        let uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("uniform_buffer"),
                contents: bytemuck::cast_slice(&[sunbeam_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }
            ],
            label: Some("uniform_bind_group_layout"),
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }
            ],
            label: Some("bind_group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &sunbeam_shader,
                entry_point: "vs_main",
                buffers: &[KbVertex::desc(), KbSpriteDrawInstance::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &sunbeam_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { 
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
           primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX
            }
        );
        KbSunbeamRenderGroup {
            pipeline,
            sunbeam_uniform,
            uniform_buffer,
            uniform_bind_group,
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn render(&mut self, device_resources: &mut KbDeviceResources, camera: &KbCamera, game_config: &KbConfig) {
		let mut command_encoder = device_resources.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("KbSunbeamRenderGroup::render()"),
		});

        let color_attachment = Some(
            wgpu::RenderPassColorAttachment {
                view: &device_resources.render_textures[0].view,//&device_resources.render_textures[2].view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,//Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0, }),
                    store: wgpu::StoreOp::Store,
            }});

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Sunbeams"),
            color_attachments: &[color_attachment],
            depth_stencil_attachment: Some(
                wgpu::RenderPassDepthStencilAttachment {
                view: &device_resources.render_textures[1].view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        let proj_matrix = cgmath::perspective(cgmath::Deg(game_config.fov), game_config.window_width as f32 / game_config.window_height as f32, 0.1, 10000.0);
        let (view_matrix, view_dir, _) = camera.calculate_view_matrix();
        let sunbeam_uniform = KbSunbeamUniform {
            view_proj: (proj_matrix * view_matrix).into(),
            camera_pos: [camera.get_position().x, camera.get_position().y, camera.get_position().z, 0.0],
            camera_dir: [view_dir.x, view_dir.y, view_dir.z, 0.0],
            screen_dimensions: [game_config.window_width as f32, game_config.window_height as f32, (game_config.window_height as f32) / (game_config.window_width as f32), 0.0],

        };
        device_resources.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[sunbeam_uniform]));

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, device_resources.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

               // render_pass.set_bind_group(0, &model.tex_bind_group, &[]);

        render_pass.draw_indexed(0..6, 0, 0..1);
        drop(render_pass);

        device_resources.queue.submit(std::iter::once(command_encoder.finish()));
    }
}