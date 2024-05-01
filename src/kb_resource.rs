use ab_glyph::FontRef;
use anyhow::*;
use cgmath::SquareMatrix;
use image::GenericImageView;
use load_file::*;
use std::{collections::HashMap, mem::size_of, sync::Arc, result::Result::Ok};
use wgpu::{BindGroupLayoutEntry, BindingType, Device, DeviceDescriptor, SamplerBindingType, SurfaceConfiguration, ShaderStages, TextureSampleType, TextureViewDimension, Queue, util::DeviceExt};
use wgpu_text::{BrushBuilder, TextBrush};

use crate::{kb_assets::*, kb_config::*, kb_game_object::*, kb_utils::*, log, PERF_SCOPE};

#[repr(C)]  // Do what C does. The order, size, and alignment of fields is exactly what you would expect from C or C++""
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct KbVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    normal: [f32; 3],
}

impl KbVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<KbVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                }
            ]
        }
    }
}
 
pub const VERTICES: &[KbVertex] = &[
    KbVertex { position: [1.0, 1.0, 0.0], tex_coords: [1.0, 0.0], normal: [0.0, 0.0, 1.0] },
    KbVertex { position: [-1.0, 1.0, 0.0], tex_coords: [0.0, 0.0], normal: [0.0, 0.0, 1.0] },
    KbVertex { position: [-1.0, -1.0, 0.0], tex_coords: [0.0, 1.0], normal: [0.0, 0.0, 1.0] },
    KbVertex { position: [1.0, -1.0, 0.0], tex_coords: [1.0, 1.0], normal: [0.0, 0.0, 1.0] },
];

pub const INDICES: &[u16] = &[
    0, 1, 3,
    3, 1, 2,
];

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct KbSpriteDrawInstance {
    pub pos_scale: [f32; 4],
    pub uv_scale_bias: [f32; 4],
    pub per_instance_data: [f32; 4],
}

impl KbSpriteDrawInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<KbSpriteDrawInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 3,     // Corresponds to @location in the shader
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 2 * size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteUniform {
    pub screen_dimensions: [f32; 4],
    pub time: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PostProcessUniform {
    pub time_mode_unused_unused: [f32;4],
}

#[allow(dead_code)] 
pub struct KbTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl KbTexture {
    pub fn new_depth_texture(device: &Device, surface_config: &SurfaceConfiguration) -> Result<Self> {
        let size = wgpu::Extent3d {
            width: surface_config.width,
            height: surface_config.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor { // 4.
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::LessEqual),
                lod_min_clamp: 0.0,
                lod_max_clamp: 100.0,
                ..Default::default()
            }
        );
        Ok(KbTexture {
            texture,
            view,
            sampler
        })
    }

    pub fn new_render_texture(device: &Device, surface_config: &wgpu::SurfaceConfiguration) ->Result<Self> {        
        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                label: Some("Render Target"),
                size: wgpu::Extent3d { width: surface_config.width, height: surface_config.height, depth_or_array_layers: 1},
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: surface_config.format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }
        );

        Ok(KbTexture {
            texture,
            view,
            sampler
        })
    }

    pub fn from_file(file_path: &str, device_resources: &KbDeviceResources) ->Result<Self> {
        log!("Loading texture {}", file_path);
		let texture_bytes = load_bytes!(file_path);
        KbTexture::from_bytes(&device_resources.device, &device_resources.queue, texture_bytes, file_path)
    }

    pub fn from_bytes(device: &Device, queue: &Queue, bytes: &[u8], label: &str) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label))
    }

    pub fn from_image(device: &Device, queue: &Queue, img: &image::DynamicImage, label: Option<&str>) -> Result<Self> {
        let dimensions = img.dimensions();
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                label,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }
        );
        
        let rgba = img.to_rgba8();

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }
        );
        
        Ok(Self {
            texture,
            view,
            sampler
        })
    }
}

#[allow(dead_code)]
pub enum KbRenderPassType {
    Opaque,
    Transparent,
    PostProcess,
}

#[derive(Clone)]
pub enum KbPostProcessMode {
    Passthrough,
    Desaturation,
    ScanLines,
    Warp,
}

#[allow(dead_code)]
pub struct KbDeviceResources<'a> {
    pub surface: wgpu::Surface<'a>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub adapter: wgpu::Adapter,
    pub device: Device,
    pub queue: Queue,

    pub instance_buffer: wgpu::Buffer,
    pub brush: TextBrush<FontRef<'a>>,
    pub render_textures: Vec<KbTexture>,    // [0] is color, [1] is depth
}

impl<'a> KbDeviceResources<'a> {
    pub fn resize(&mut self, game_config: &KbConfig) {
        assert!(game_config.window_width > 0 && game_config.window_height > 0);

        self.surface_config.width = game_config.window_width;
        self.surface_config.height = game_config.window_height;
        self.surface.configure(&self.device, &self.surface_config);

        self.render_textures[0] = KbTexture::new_render_texture(&self.device, &self.surface_config).unwrap();
        self.render_textures[1] = KbTexture::new_depth_texture(&self.device, &self.surface_config).unwrap();
    }

    pub async fn new(window: Arc::<winit::window::Window>, game_config: &KbConfig) -> Self {
        log!("KbDeviceResources::new() called...");
        
        log!("  Creating instance"); 
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: game_config.graphics_backend,
            ..Default::default()
        });

        log!("  Creating surface + adapter");

        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: game_config.graphics_power_pref,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        log!("  Requesting Device");
		let (device, queue) = adapter.request_device(
            &DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                label: Some("Device Descriptor"),
            },
            None, // Trace path
        ).await.unwrap();

        let surface_config = surface.get_default_config(&adapter, game_config.window_width, game_config.window_height).unwrap();
        surface.configure(&device, &surface_config);

        let max_instances = game_config.max_render_instances;
        let instance_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("instance_buffer"),
                mapped_at_creation: false,
                size: (size_of::<KbSpriteDrawInstance>() * max_instances as usize) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
            });
       
        let mut render_textures = Vec::<KbTexture>::new();
        let render_texture = KbTexture::new_render_texture(&device, &surface_config).unwrap();
        render_textures.push(render_texture);

        let depth_texture = KbTexture::new_depth_texture(&device, &surface_config).unwrap();
        render_textures.push(depth_texture);

        log!("  Creating Font");
        let brush = BrushBuilder::using_font_bytes(load_bytes!("../engine_assets/fonts/Bold.ttf")).unwrap()
                .build(&device, surface_config.width, surface_config.height, surface_config.format);

        log!("KbDeviceResources allocated");
	    KbDeviceResources {
            surface_config,
            surface,
            adapter,
            device,
            queue,
            instance_buffer,
            brush,
            render_textures,
        }
    }
}

pub struct KbSpritePipeline {
    pub opaque_pipeline: wgpu::RenderPipeline,
    pub alpha_blend_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub uniform: SpriteUniform,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub tex_bind_group: wgpu::BindGroup,
}

impl KbSpritePipeline {
    pub fn new(device_resources: &KbDeviceResources, asset_manager: &mut KbAssetManager, game_config: &KbConfig) -> Self {
        log!("Creating KbSpritePipeline...");

        let device = &device_resources.device;
        let surface_config = &device_resources.surface_config;        
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
                },
            ],
            label: Some("kbSpritePipeline: texture_bind_group_layout"),
        });

        let sprite_tex_handle = asset_manager.load_texture("/engine_assets/textures/SpriteSheet.png", &device_resources);
        let postprocess_tex_handle = asset_manager.load_texture("/engine_assets/textures/PostProcessFilter.png", &device_resources);
        let sprite_tex = asset_manager.get_texture(&sprite_tex_handle);
        let postprocess_tex = asset_manager.get_texture(&postprocess_tex_handle);
        let tex_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&sprite_tex.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sprite_tex.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&postprocess_tex.view),
                    },
                ],
                label: Some("kbSpritePipeline: tex_bind_group"),
            }
        );

        let shader_handle = asset_manager.load_shader("/engine_assets/shaders/BasicSprite.wgsl", &device_resources);
        let shader = asset_manager.get_shader(&shader_handle);
        
        let uniform = SpriteUniform {
            ..Default::default()
        };

        let uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("sprite_uniform_buffer"),
                contents: bytemuck::cast_slice(&[uniform]),
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
            label: Some("sprite_uniform_bind_group_layout"),
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }
            ],
            label: Some("model_bind_group"),
        });

        log!("  Creating pipeline");

        // Render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let opaque_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[KbVertex::desc(), KbSpriteDrawInstance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { 
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
                depth_compare: wgpu::CompareFunction::Always,
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

        let transparent_shader_handle = asset_manager.load_shader("/engine_assets/shaders/CloudSprite.wgsl", &device_resources);
        let transparent_shader = asset_manager.get_shader(&transparent_shader_handle);

        let alpha_blend_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &transparent_shader,
                entry_point: "vs_main",
                buffers: &[KbVertex::desc(), KbSpriteDrawInstance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &transparent_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { 
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
                depth_compare: wgpu::CompareFunction::Always,
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

        log!("  Creating vertex/index buffers");

        // Vertex/Index buffer
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST
            }
        );

        let instance_buffer = device.create_buffer(
        &wgpu::BufferDescriptor {
            label: Some("instance_buffer"),
            mapped_at_creation: false,
            size: (size_of::<KbSpriteDrawInstance>() * game_config.max_render_instances as usize) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
        });

        KbSpritePipeline {
            opaque_pipeline,
            alpha_blend_pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            uniform,
            uniform_buffer,
            uniform_bind_group,
            tex_bind_group,
        }
    }

    pub fn render(&mut self, render_pass_type: KbRenderPassType, should_clear: bool, device_resources: &mut KbDeviceResources, game_config: &KbConfig, game_objects: &Vec<GameObject>) {
		let mut command_encoder = device_resources.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("KbSpritePipeline::render()"),
		});

        let mut frame_instances = Vec::<KbSpriteDrawInstance>::new();

        // Create instances
        let u_scale = 1.0 / 8.0;
        let v_scale = 1.0 / 8.0;
        let extra_scale = 1.0;
        let extra_offset: CgVec3 = CgVec3::new(0.0, -0.35, 0.0);

        let game_object_iter = game_objects.iter();
        for game_object in game_object_iter {
            PERF_SCOPE!("Creating instances");
            let game_object_position = game_object.position + extra_offset;
            let sprite_index = game_object.sprite_index + game_object.anim_frame;
            let mut u_offset = ((sprite_index % 8) as f32) * u_scale;
            let v_offset = ((sprite_index / 8) as f32) * v_scale;
            let mul = if game_object.direction.x > 0.0 { 1.0 } else { -1.0 };
            if mul < 0.0 {
                u_offset = u_offset + u_scale;
            }

            let new_instance = KbSpriteDrawInstance {
                pos_scale: [game_object_position.x, game_object_position.y, game_object.scale.x * extra_scale, game_object.scale.y * extra_scale],
                uv_scale_bias: [u_scale * mul, v_scale, u_offset, v_offset],
                per_instance_data: [game_object.random_val, 0.0, 0.0, 0.0],
            };
            frame_instances.push(new_instance);
        }
        device_resources.queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(frame_instances.as_slice()));
      
        let color_attachment = {
            if should_clear {
                Some(wgpu::RenderPassColorAttachment {
                    view: &device_resources.render_textures[0].view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.12,
                            g: 0.01,
                            b: 0.35,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })
            } else {
                Some(wgpu::RenderPassColorAttachment {
                    view: &device_resources.render_textures[0].view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })
            }
        };


        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[color_attachment],
            depth_stencil_attachment:  Some(wgpu::RenderPassDepthStencilAttachment {
                view: &device_resources.render_textures[1].view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        device_resources.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniform]));

        if matches!(render_pass_type, KbRenderPassType::Opaque) {
            render_pass.set_pipeline(&self.opaque_pipeline);
        } else {
            render_pass.set_pipeline(&self.alpha_blend_pipeline);
        }

        self.uniform.screen_dimensions = [game_config.window_width as f32, game_config.window_height as f32, (game_config.window_height as f32) / (game_config.window_width as f32), 0.0];//[self.game_config.window_width as f32, self.game_config.window_height as f32, (self.game_config.window_height as f32) / (self.game_config.window_width as f32), 0.0]));
        self.uniform.time[0] = game_config.start_time.elapsed().as_secs_f32();

        #[cfg(target_arch = "wasm32")]
        {
            self.uniform.time[1] = 1.0 / 2.2;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.uniform.time[1] = 1.0;
        }

        render_pass.set_bind_group(0, &self.tex_bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..frame_instances.len() as _);
        drop(render_pass);
        device_resources.queue.submit(std::iter::once(command_encoder.finish()));
    }
}

pub struct KbPostprocessPipeline {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub postprocess_pipeline: wgpu::RenderPipeline,
    pub postprocess_uniform: PostProcessUniform,
    pub postprocess_constant_buffer: wgpu::Buffer,
    pub postprocess_uniform_bind_group: wgpu::BindGroup,
    pub postprocess_bind_group: wgpu::BindGroup,
}

impl KbPostprocessPipeline {
    pub fn new(device_resources: &KbDeviceResources, asset_manager: &mut KbAssetManager) -> Self {
        let device = &device_resources.device;
        let surface_config = &device_resources.surface_config;
        let render_texture = &device_resources.render_textures[0];

        // Post Process Pipeline
        let postprocesst_shader_handle = asset_manager.load_shader("/engine_assets/shaders/postprocess_uber.wgsl", &device_resources);
        let postprocess_shader = asset_manager.get_shader(&postprocesst_shader_handle);
        
        let postprocess_uniform = PostProcessUniform {
            ..Default::default()
        };

        let postprocess_constant_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("postprocess_constant_buffer"),
                contents: bytemuck::cast_slice(&[postprocess_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let postprocess_uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("postprocess_uniform_bind_group_layout"),
        });


        let postprocess_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &postprocess_uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: postprocess_constant_buffer.as_entire_binding(),
                }
            ],
            label: Some("postprocess_bind_group"),
        });

        let postprocess_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
            ],
            label: Some("postprocess_bind_group_layout"),
        });

        let postprocess_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("postprocess_pipeline_layout"),
            bind_group_layouts: &[&postprocess_bind_group_layout, &postprocess_uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let postprocess_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("postprocess_pipeline"),
            layout: Some(&postprocess_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &postprocess_shader,
                entry_point: "vs_main",
                buffers: &[KbVertex::desc(), KbSpriteDrawInstance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &postprocess_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { 
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let postprocess_tex_handle = asset_manager.load_texture("/engine_assets/textures/PostProcessFilter.png", &device_resources);
        let postprocess_tex = asset_manager.get_texture(&postprocess_tex_handle);
        let postprocess_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &postprocess_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&postprocess_tex.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&postprocess_tex.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&render_texture.view),
                    },
                ],
                label: Some("postprocess_bind_group"),
            }
        );
 
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST
            }
        );
        KbPostprocessPipeline {
            postprocess_pipeline,
            postprocess_uniform,
            postprocess_constant_buffer,
            postprocess_uniform_bind_group,
            postprocess_bind_group,
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn render(&mut self, target_view: &wgpu::TextureView, device_resources: &mut KbDeviceResources, game_config: &KbConfig) {
		let mut command_encoder = device_resources.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("KbPostprocessPipeline::render()"),
		});

        let color_attachment = Some(
            wgpu::RenderPassColorAttachment {
                view: &target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
            }});

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("postprocess_render_pass"),
            color_attachments: &[color_attachment],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.postprocess_pipeline);
        render_pass.set_bind_group(0, &self.postprocess_bind_group, &[]);
        render_pass.set_bind_group(1, &self.postprocess_uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, device_resources.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        self.postprocess_uniform.time_mode_unused_unused[0] = game_config.start_time.elapsed().as_secs_f32();
        self.postprocess_uniform.time_mode_unused_unused[1] = {
            match game_config.postprocess_mode {
                KbPostProcessMode::Desaturation => { 1.0 }
                KbPostProcessMode::ScanLines => { 2.0 }
                KbPostProcessMode::Warp => { 3.0 }
                _ => { 0.0 }
            }
        };

        device_resources.queue.write_buffer(&self.postprocess_constant_buffer, 0, bytemuck::cast_slice(&[self.postprocess_uniform]));
        render_pass.draw_indexed(0..6, 0, 0..1);
        drop(render_pass);

        device_resources.queue.submit(std::iter::once(command_encoder.finish()));
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct KbModelUniform {
    pub inv_world: [[f32; 4]; 4],
    pub mvp_matrix: [[f32; 4]; 4],
    pub camera_pos:[f32; 4],
    pub camera_dir:[f32; 4],
    pub screen_dimensions: [f32; 4],
    pub time: [f32; 4],
}
pub const MAX_UNIFORMS: usize = 100;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct KbModelDrawInstance {
    pub position: [f32; 4],
    pub scale: [f32; 4],
    pub color: [f32; 4],
}
pub const MAX_MODEL_INSTANCES: usize = 1000;
pub const MAX_PARTICLE_INSTANCES: usize = 1000;

impl KbModelDrawInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<KbModelDrawInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 12,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 13,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 2 * size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 14,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct KbModel {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub num_indices: u32,

    pub textures: Vec<KbTextureHandle>,
    pub tex_bind_group: wgpu::BindGroup,

    uniform_buffers: Vec<wgpu::Buffer>,
    uniform_bind_groups: Vec<wgpu::BindGroup>,
    next_uniform_buffer: usize,
}

impl KbModel {
    pub fn new_particle(texture_file_path: &str, device_resources: &KbDeviceResources, asset_manager: &mut KbAssetManager) -> Self {
        let device = &device_resources.device;

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("KbModel_particle_vertex_buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("KbModel_particle_iertex_buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX
            }
        );

        let instance_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("KbModel_particle_instance_buffer"),
                mapped_at_creation: false,
                size: (size_of::<KbModelDrawInstance>() * MAX_PARTICLE_INSTANCES as usize) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
            }
        );

        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("KbModel_texture_bind_group_layout"),
        });

        let mut textures = Vec::<KbTextureHandle>::new();
        let texture_handle = asset_manager.load_texture(texture_file_path, &device_resources);
        textures.push(texture_handle);
        let texture = asset_manager.get_texture(&textures[0]);

        let tex_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
                label: Some("KbModel_tex_bind_group"),
            }
        );

        // Uniform buffer
        let mut uniform_buffers = Vec::<wgpu::Buffer>::new();
        let mut uniform_bind_groups = Vec::<wgpu::BindGroup>::new();
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
                label: Some("KbModelPipeline_uniform_bind_group_layout"),
        });

        let empty_uniform = KbModelUniform{ ..Default::default() };
        //let mut uniforms: Vec<KbModelUniform> = Vec::with_capacity(MAX_UNIFORMS);

        for _ in  0..MAX_UNIFORMS {
            let uniform_buffer = device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("kbModelPipeline_uniform_buffer"),
                    contents: bytemuck::cast_slice(&[empty_uniform]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                }
            );

            let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &uniform_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    }
                ],
                label: Some("KbModelPipeline_uniform_bind_group"),
            });

          //  uniforms.push(empty_uniform);
            uniform_buffers.push(uniform_buffer);
            uniform_bind_groups.push(uniform_bind_group);
        }

        KbModel {
            vertex_buffer,
            index_buffer,
            instance_buffer,
            num_indices: 6,
            textures,
            tex_bind_group,
            //uniforms,
            uniform_buffers,
            uniform_bind_groups,
            next_uniform_buffer: 0
        }
    }

    pub fn new(file_name: &str, device_resources: &mut KbDeviceResources, asset_manager: &mut KbAssetManager) -> Self {
        log!("Loading Model {file_name}");

        let device = &device_resources.device;     
        let mut indices = Vec::<u16>::new();
        let mut vertices = Vec::<KbVertex>::new();
        let mut textures = Vec::<KbTextureHandle>::new();

        // https://stackoverflow.com/questions/75846989/how-to-load-gltf-files-with-gltf-rs-crate
        let (gltf_doc, buffers, _) = gltf::import(file_name).unwrap();
        for gltf_texture in gltf_doc.textures() {
            log!("  Hitting that iteration");

            match gltf_texture.source().source() {

                gltf::image::Source::View { view: _, mime_type: _ } => { }
                gltf::image::Source::Uri { uri, mime_type: _ } => {
                    match std::env::current_dir() {
                        Ok(dir) => {
                            let file_path = format!("{}\\game_assets\\{}", dir.display(), uri);
                            let texture_handle = asset_manager.load_texture(&file_path, &device_resources);
                            textures.push(texture_handle);
                        }
                        _ => {}
                    }
                }
            }
        }

        for m in gltf_doc.meshes() {
            for p in m.primitives() {
                let r = p.reader(|buffer| Some(&buffers[buffer.index()]));
                if let Some(gltf::mesh::util::ReadIndices::U16(gltf::accessor::Iter::Standard(iter))) = r.read_indices(){
                    for v in iter {
                        indices.push(v);
                    }
                }

                let mut positions = Vec::new();
                if let Some(iter) = r.read_positions(){
                    for v in iter{
                        positions.push(v);
                    }
                }

                let mut uvs = Vec::new();
                if let Some(gltf::mesh::util::ReadTexCoords::F32(gltf::accessor::Iter::Standard(iter))) = r.read_tex_coords(0){
                    for v in iter{
                        uvs.push(v);
                    }
                }

                let mut normals = Vec::new();
                if let Some(iter) = r.read_normals(){
                    for v in iter{
                        normals.push(v);
                    }
                }

                let mut i = 0;
                while i < positions.len() {
                    let vertex = KbVertex {
                        position: positions[i],
                        tex_coords: uvs[i],
                        normal: normals[i]
                    };
                    vertices.push(vertex);
                    i = i + 1;
                }
            }
        }

        let num_indices = indices.len() as u32;

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("KbModel_vertex_buffer"),
                contents: bytemuck::cast_slice(vertices.as_slice()),
                usage: wgpu::BufferUsages::VERTEX
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices.as_slice()),
                usage: wgpu::BufferUsages::INDEX
            }
        );

        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("KbModel_texture_bind_group_layout"),
        });

        let texture = asset_manager.get_texture(&textures[0]);
        let tex_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
                label: Some("KbModel_tex_bind_group"),
            }
        );

        // Uniform buffer
        let mut uniform_buffers = Vec::<wgpu::Buffer>::new();
        let mut uniform_bind_groups = Vec::<wgpu::BindGroup>::new();
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
                label: Some("KbModelPipeline_uniform_bind_group_layout"),
        });

        let uniform = KbModelUniform{ ..Default::default() };

        for _ in 0..MAX_UNIFORMS {
            let uniform_buffer = device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("kbModelPipeline_uniform_buffer"),
                    contents: bytemuck::cast_slice(&[uniform]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                }
            );

            let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &uniform_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    }
                ],
                label: Some("KbModelPipeline_uniform_bind_group"),
            });

            uniform_buffers.push(uniform_buffer);
            uniform_bind_groups.push(uniform_bind_group);
        }
        
        let instance_buffer = device.create_buffer(
        &wgpu::BufferDescriptor {
            label: Some("instance_buffer"),
            mapped_at_creation: false,
            size: (size_of::<KbModelDrawInstance>() * MAX_UNIFORMS as usize) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
        });

        KbModel {
            vertex_buffer,
            index_buffer,
            instance_buffer,
            num_indices,
            uniform_bind_groups,
            uniform_buffers,
            textures,
            tex_bind_group,
            next_uniform_buffer: 0
        }
    }

    pub fn alloc_uniform_buffer(&mut self) -> &mut wgpu::Buffer {
        let ret_val = &mut self.uniform_buffers[self.next_uniform_buffer];
        self.next_uniform_buffer = self.next_uniform_buffer + 1;
        ret_val
    }

    pub fn get_uniform_bind_group(&self, index: usize) -> &wgpu::BindGroup {
        &self.uniform_bind_groups[index]
    }

    pub fn get_uniform_info_count(&self) -> usize {
        self.next_uniform_buffer
    }

    // Call after submitting the KbModel's draw calls for the frame
    pub fn free_uniform_buffers(&mut self) {
        self.next_uniform_buffer = 0;
    }
}

pub struct KbModelPipeline {
    pub opaque_pipeline: wgpu::RenderPipeline,
    pub alpha_blend_pipeline: wgpu::RenderPipeline,
    pub additive_pipeline: wgpu::RenderPipeline,
    pub uniform: KbModelUniform,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

impl KbModelPipeline {
    pub fn new(device_resources: &KbDeviceResources, asset_manager: &mut KbAssetManager) -> Self {
        log!("Creating KbModelPipeline...");

        let device = &device_resources.device;
        let surface_config = &device_resources.surface_config;

        log!("  Uniform data");
       
        // Uniform buffer
        let uniform = KbModelUniform{ ..Default::default() };
        let uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("kbModelPipeline_uniform_buffer"),
                contents: bytemuck::cast_slice(&[uniform]),
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
            label: Some("KbModelPipeline_uniform_bind_group_layout"),
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }
            ],
            label: Some("KbModelPipeline_uniform_bind_group"),
        });

        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("KbModelPipeline_texture_bind_group_layout"),
        });

        log!("  Creating pipeline");

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("KbModelPipeline_render_pipeline_layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let opaque_shader_handle = asset_manager.load_shader("/engine_assets/shaders/Model.wgsl", &device_resources);
        let opaque_shader = asset_manager.get_shader(&opaque_shader_handle);

        let opaque_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("KbModelPipeline_opaque_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &opaque_shader,
                entry_point: "vs_main",
                buffers: &[KbVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &opaque_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { 
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
                depth_write_enabled: true,
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

        let particle_shader_handle = asset_manager.load_shader("/engine_assets/shaders/particle.wgsl", &device_resources);
        let particle_shader = asset_manager.get_shader(&particle_shader_handle);
        let alpha_blend_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("KbModelPipeline_alpha_blend_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &particle_shader,
                entry_point: "vs_main",
                buffers: &[KbVertex::desc(), KbModelDrawInstance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &particle_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { 
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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

        let additive_blend_state = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        };

        let additive_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("KbModelPipeline_additive_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &particle_shader,
                entry_point: "vs_main",
                buffers: &[KbVertex::desc(), KbModelDrawInstance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &particle_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { 
                    format: surface_config.format,
                    blend: Some(additive_blend_state),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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

        KbModelPipeline {
            opaque_pipeline,
            alpha_blend_pipeline,
            additive_pipeline,
            uniform,
            uniform_buffer,
            uniform_bind_group,
        }
    }

    pub fn render(&mut self, should_clear: bool, device_resources: &mut KbDeviceResources, game_camera: &KbCamera, models: &mut Vec<KbModel>, actors: &HashMap<u32, KbActor>, game_config: &KbConfig) {
        let mut command_encoder = device_resources.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("KbModelPipeline::render()"),
        });

        let (color_attachment, depth_attachment) = {
            if should_clear {
                (wgpu::RenderPassColorAttachment {
                    view: &device_resources.render_textures[0].view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.12, g: 0.01, b: 0.35, a: 1.0, }),
                        store: wgpu::StoreOp::Store,
                    },
                }, wgpu::RenderPassDepthStencilAttachment {
                    view: &device_resources.render_textures[1].view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                })
            } else {
                (wgpu::RenderPassColorAttachment {
                    view: &device_resources.render_textures[0].view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                }, wgpu::RenderPassDepthStencilAttachment {
                    view: &device_resources.render_textures[1].view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                })
            }
        };

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("KbModelPipeline_render_pass"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment:  Some(depth_attachment),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        render_pass.set_pipeline(&self.opaque_pipeline);

        let (view_matrix, _, _) = game_camera.calculate_view_matrix();      
        let proj_matrix = cgmath::perspective(cgmath::Deg(game_config.fov), game_config.window_width as f32 / game_config.window_height as f32, 0.1, 10000.0);
        let fragment_texture_fix = {
            #[cfg(target_arch = "wasm32")] { 1.0 / 2.2 }
            #[cfg(not(target_arch = "wasm32"))] { 1.0 }
        };
 
        // Iterate over actors and add their uniform info to their corresponding KbModels
        let actor_iter = actors.iter();
        for actor_key_value in actor_iter {
            let actor = actor_key_value.1;
            let model_id = actor.get_model().index;
            if model_id as usize > models.len() {
                continue;
            }
            let model = &mut models[model_id as usize];

            let uniform_buffer = model.alloc_uniform_buffer();
            let mut uniform_data = KbModelUniform { ..Default::default() };
            let world_matrix = cgmath::Matrix4::from_translation(actor.get_position()) * cgmath::Matrix4::from_scale(actor.get_scale().x);
            uniform_data.inv_world = world_matrix.invert().unwrap().into();
            uniform_data.mvp_matrix = (proj_matrix * view_matrix * world_matrix).into();
            uniform_data.screen_dimensions = [game_config.window_width as f32, game_config.window_height as f32, (game_config.window_height as f32) / (game_config.window_width as f32), 0.0];
            uniform_data.time[0] = game_config.start_time.elapsed().as_secs_f32();
            uniform_data.time[1] = fragment_texture_fix;
            device_resources.queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[uniform_data]));
        }

        // Render KbModels now that uniforms are set
        let model_iter = models.iter_mut();
        for model in model_iter {
            render_pass.set_vertex_buffer(0, model.vertex_buffer.slice(..));
            render_pass.set_index_buffer(model.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            for _ in 0..model.get_uniform_info_count() {
                let uniform_bind_group = &model.get_uniform_bind_group(0);
                render_pass.set_bind_group(1, uniform_bind_group, &[]);
                render_pass.set_bind_group(0, &model.tex_bind_group, &[]);
                render_pass.draw_indexed(0..model.num_indices, 0, 0..1);
            }
        }
        
        drop(render_pass);
        device_resources.queue.submit(std::iter::once(command_encoder.finish()));

        let model_iter = models.iter_mut();
        for model in model_iter {
            model.free_uniform_buffers();
        }
    }

    pub fn render_particles(&mut self, _blend_mode: KbParticleBlendMode, device_resources: &mut KbDeviceResources, game_camera: &KbCamera, particles: &mut HashMap<KbParticleHandle, KbParticleActor>, game_config: &KbConfig) {
         let mut command_encoder = device_resources.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("KbModelPipeline::render_particles()"),
        });

        // Create instances
        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[ Some(wgpu::RenderPassColorAttachment {
                    view: &device_resources.render_textures[0].view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
            depth_stencil_attachment:  Some(wgpu::RenderPassDepthStencilAttachment {
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
       
        let (view_matrix, view_dir, _) = game_camera.calculate_view_matrix();
        let view_pos = game_camera.get_position();
        let view_pos = [view_pos.x, view_pos.y, view_pos.z, 1.0];
        let proj_matrix = cgmath::perspective(cgmath::Deg(game_config.fov), game_config.window_width as f32 / game_config.window_height as f32, 0.1, 1000000.0);
        let view_proj_matrix = proj_matrix * view_matrix;
        let fragment_texture_fix = {
            #[cfg(target_arch = "wasm32")] { 1.0 / 2.2 }
            #[cfg(not(target_arch = "wasm32"))] { 1.0 }
        };

        match _blend_mode {
            KbParticleBlendMode::AlphaBlend => { render_pass.set_pipeline(&self.alpha_blend_pipeline); }
            KbParticleBlendMode::Additive => { render_pass.set_pipeline(&self.additive_pipeline) }
        };
       
        let particle_iter = particles.iter_mut();
        for mut particle_val in particle_iter {
            let particle_actor = &mut particle_val.1;
            if particle_actor.params.blend_mode != _blend_mode {
                continue;
            }

            let position = particle_actor.get_position();
            let scale = particle_actor.get_scale();
            let model = &mut particle_val.1.model;

            // Uniform data
            model.free_uniform_buffers();
            let uniform_buffer = model.alloc_uniform_buffer();

            let world_matrix = cgmath::Matrix4::from_translation(position) * cgmath::Matrix4::from_scale(scale.x);
            let mut uniform = KbModelUniform { ..Default::default() };
            uniform.inv_world = world_matrix.invert().unwrap().into();
            uniform.mvp_matrix = (view_proj_matrix * world_matrix).into();
            uniform.camera_pos = view_pos;
            uniform.camera_dir = [view_dir.x, view_dir.y, view_dir.z, 0.0];
            uniform.screen_dimensions = [game_config.window_width as f32, game_config.window_height as f32, (game_config.window_height as f32) / (game_config.window_width as f32), 0.0];
            uniform.time[0] = game_config.start_time.elapsed().as_secs_f32();
            uniform.time[1] = fragment_texture_fix;
            device_resources.queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));

            // Instances
            let particles = &particle_val.1.particles;
            let mut particle_instances = Vec::<KbModelDrawInstance>::new();
            for particle in particles {
                let new_instance = KbModelDrawInstance {
                    position: [particle.position.x, particle.position.y, particle.position.z, particle.rotation],
                    scale: [particle.scale.x, particle.scale.y, 0.0, 0.0],
                    color: particle.color.into()
                };
                particle_instances.push(new_instance);
            }
            device_resources.queue.write_buffer(&model.instance_buffer, 0, bytemuck::cast_slice(particle_instances.as_slice())); 

            render_pass.set_vertex_buffer(0, model.vertex_buffer.slice(..));
            render_pass.set_index_buffer(model.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.set_vertex_buffer(1, model.instance_buffer.slice(..));
            let uniform_bind_group = &model.get_uniform_bind_group(0);
            render_pass.set_bind_group(1, uniform_bind_group, &[]);
            render_pass.set_bind_group(0, &model.tex_bind_group, &[]);
            render_pass.draw_indexed(0..model.num_indices, 0, 0..particle_instances.len() as u32);
        }
        drop(render_pass);
        device_resources.queue.submit(std::iter::once(command_encoder.finish()));
    }
}