use wgpu::util::DeviceExt;

use crate::{kb_assets::*, kb_resource::*, log};

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct KbBulletHoleUniform {
    pub world: [[f32; 4]; 4],
    pub inv_world: [[f32; 4]; 4],
    pub mvp_matrix: [[f32; 4]; 4],
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4],
    pub camera_dir: [f32; 4],
    pub screen_dimensions: [f32; 4],
    pub time: [f32; 4],
    pub model_color: [f32; 4],
    pub custom_data_1: [f32; 4],
    pub sun_color: [f32; 4],
}
pub const MAX_UNIFORMS: usize = 100;

#[allow(dead_code)]
pub struct KbBulletHoleRenderGroup {
    pub pipeline: wgpu::RenderPipeline,
    pub uniform: KbBulletHoleUniform,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

#[allow(dead_code)]
impl KbBulletHoleRenderGroup {
    pub async fn new(
        shader_path: &str,
        device_resources: &KbDeviceResources<'_>,
        asset_manager: &mut KbAssetManager,
    ) -> Self {
        log!("Creating KbBulletHoleRenderGroup with shader {shader_path}");
        let device = &device_resources.device;
        let surface_config = &device_resources.surface_config;

        // Uniform buffer
        let uniform = KbBulletHoleUniform {
            ..Default::default()
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("KbBulletHoleRenderGroup::uniform_buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("KbModelRenderGroup_uniform_bind_group_layout"),
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("KbModelRenderGroup_uniform_bind_group"),
        });

        log!("  Creating pipeline");

        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("KbModelRenderGroup_render_pipeline_layout"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        let shader_handle = asset_manager
            .load_shader(shader_path, &device_resources)
            .await;
        let model_shader = asset_manager.get_shader(&shader_handle);

        let mut cull_mode = Some(wgpu::Face::Back);
        if shader_path.contains("decal") {
            cull_mode = None;
        }

    
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("KbBulletHoleRenderGroup::pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &model_shader,
                entry_point: "vs_main",
                buffers: &[KbVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &model_shader,
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
                cull_mode,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        KbBulletHoleRenderGroup {
            pipeline,
            uniform,
            uniform_buffer,
            uniform_bind_group,
        }
    }

    pub fn render(
        &mut self,
    ) {
 
    }
}
