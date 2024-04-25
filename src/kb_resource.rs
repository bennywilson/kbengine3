use std::sync::Arc;
use ab_glyph::FontRef;
use anyhow::*;
use cgmath::Vector3;
use image::GenericImageView;
use wgpu::SurfaceConfiguration;
use wgpu_text::{glyph_brush::{Section as TextSection, Text}, BrushBuilder, TextBrush};

use crate::{kb_config::KbConfig, kb_pipeline::{KbModelPipeline, KbPostprocessPipeline, KbSpritePipeline}, log};

#[repr(C)]  // Do what C does. The order, size, and alignment of fields is exactly what you would expect from C or C++""
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct KbVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl KbVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<KbVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                }
            ]
        }
    }
}
 
pub const VERTICES: &[KbVertex] = &[
    KbVertex { position: [1.0, 1.0, 0.0], tex_coords: [1.0, 0.0], },
    KbVertex { position: [-1.0, 1.0, 0.0], tex_coords: [0.0, 0.0], },
    KbVertex { position: [-1.0, -1.0, 0.0], tex_coords: [0.0, 1.0], },
    KbVertex { position: [1.0, -1.0, 0.0], tex_coords: [1.0, 1.0], },
];

pub const INDICES: &[u16] = &[
    0, 1, 3,
    3, 1, 2,
];

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct KbDrawInstance {
    pub pos_scale: [f32; 4],
    pub uv_scale_bias: [f32; 4],
    pub per_instance_data: [f32; 4],
}

impl KbDrawInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<KbDrawInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,     // Corresponds to @location in the shader
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 2 * std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 4,
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
    pub fn new_depth_texture(device: &wgpu::Device, surface_config: &SurfaceConfiguration) -> Self {
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
        KbTexture {
            texture,
            view,
            sampler
        }
    }

    pub fn new_render_texture(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) ->Result<Self> {        
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

    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8], 
        label: &str
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label))
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>
    ) -> Result<Self> {
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
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub instance_buffer: wgpu::Buffer,
    pub brush: TextBrush<FontRef<'a>>,
    pub render_textures: Vec<KbTexture>,
    pub depth_textures: Vec<KbTexture>,

    pub sprite_pipeline: KbSpritePipeline,
    pub postprocess_pipeline: KbPostprocessPipeline,
    pub model_pipeline: KbModelPipeline,
}

impl<'a> KbDeviceResources<'a> {
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
		if new_size.width > 0 && new_size.height > 0 {
			self.surface_config.width = new_size.width;
			self.surface_config.height = new_size.height;
			self.surface.configure(&self.device, &self.surface_config);
            self.depth_textures[0] = KbTexture::new_depth_texture(&self.device, &self.surface_config);
            for texture in &mut self.render_textures {
                *texture = KbTexture::new_render_texture(&self.device, &self.surface_config).unwrap();
            }
            self.sprite_pipeline = KbSpritePipeline::new(&self.device, &self.queue, &self.surface_config);
            self.postprocess_pipeline = KbPostprocessPipeline::new(&self.device, &self.queue, &self.surface_config, &self.render_textures[0]);

		}
    }

     pub async fn new(window: Arc::<winit::window::Window>, game_config: &KbConfig) -> Self {
        log!("Creating instance"); 
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: game_config.graphics_backend,
            ..Default::default()
        });

        log!("Creating surface + adapter");

        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: game_config.graphics_power_pref,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        log!("Requesting Device");
		let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
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

        log!("Loading Texture");

        let max_instances = game_config.max_render_instances;
        let instance_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("instance_buffer"),
                mapped_at_creation: false,
                size: (std::mem::size_of::<KbDrawInstance>() * max_instances as usize) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
            });
       
        let mut render_textures = Vec::<KbTexture>::new();
        let render_texture = KbTexture::new_render_texture(&device, &surface_config).unwrap();
        render_textures.push(render_texture);

        let mut depth_textures = Vec::<KbTexture>::new();
        let depth_texture = KbTexture::new_depth_texture(&device, &surface_config);
        depth_textures.push(depth_texture);

        log!("Creating Font");

        let brush = BrushBuilder::using_font_bytes(include_bytes!("../game_assets/Bold.ttf")).unwrap()
                .build(&device, surface_config.width, surface_config.height, surface_config.format);

        let sprite_pipeline = KbSpritePipeline::new(&device, &queue, &surface_config);
        let postprocess_pipeline = KbPostprocessPipeline::new(&device, &queue, &surface_config, &render_textures[0]);
        let model_pipeline = KbModelPipeline::new(&device, &queue, &surface_config);

	    KbDeviceResources {
            surface_config,
            surface,
            adapter,
            device,
            queue,
            instance_buffer,
            brush,
            render_textures,
            depth_textures,
            sprite_pipeline,
            postprocess_pipeline,
            model_pipeline
        }
    }
}