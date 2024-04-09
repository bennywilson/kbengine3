use winit::window::Window;
use std::fs::File;
use std::io::Read;
use wgpu::util::DeviceExt;
use wgpu_text::{glyph_brush::{Section as TextSection, Text}, BrushBuilder, TextBrush};
use ab_glyph::{FontRef};

use crate::game_texture;
use crate::game_object::*;
use cgmath::Vector3;

#[repr(C)]  // Do what C does.  The order, size, and alignment are what you expect from C, C++S
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
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
 
const VERTICES: &[Vertex] = &[
    Vertex { position: [1.0, 1.0, 0.0], tex_coords: [1.0, 0.0], },
    Vertex { position: [-1.0, 1.0, 0.0], tex_coords: [0.0, 0.0], },
    Vertex { position: [-1.0, -1.0, 0.0], tex_coords: [0.0, 1.0], },
    Vertex { position: [1.0, -1.0, 0.0], tex_coords: [1.0, 1.0], },
];

const INDICES: &[u16] = &[
    0, 1, 3,
    3, 1, 2,
];

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceBuffer {
    pos_scale: [f32; 4],
    uv_scale_bias: [f32; 4],
}

impl InstanceBuffer {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceBuffer>() as wgpu::BufferAddress,
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
            ],
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelUniform {
    pub location: [f32; 4],
    pub uv_offset: [f32; 4],
}

#[allow(dead_code)] 
pub struct Renderer<'a> {
    surface: wgpu::Surface<'a>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
   //ow: Window,
    num_vertices: usize,
    num_indices: usize,
    texture_atlases_bind_group: Vec<wgpu::BindGroup>,
    pub model_uniform: ModelUniform,
    model_constant_buffer: wgpu::Buffer,
    model_bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    brush: TextBrush<FontRef<'a>>,
    frame_times: Vec<f32>,
    frame_timer: std::time::Instant,
    frame_count: u32,
    pub max_instances: usize,
//    power_preference: wgpu::PowerPreference,
}
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}
impl<'a> Renderer<'a> {


    pub fn default() -> Self {
        Renderer {

        }O
    }

    pub async fn new(window: std::sync::Arc::<winit::window::Window>, graphics_back_name: &str, graphics_power_pref: &str, max_instances: usize) -> Self {
        log!("Allocating renderer!");
    let size = window.inner_size();

        let graphics_back_end = match graphics_back_name {
            "dx12" => { wgpu::Backends::DX12 }
            "webgpu" => { wgpu::Backends::BROWSER_WEBGPU }
            "vulkan" => { wgpu::Backends::VULKAN }
            _ => { wgpu::Backends::all() }
        };

        let power_preference = match graphics_power_pref {
            "high" => { wgpu::PowerPreference::HighPerformance }
            "low" => { wgpu::PowerPreference::LowPower }
            _ => { wgpu::PowerPreference::None }
        };


          log!("Step 1!");
        // Instance + Surface
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: graphics_back_end,
            ..Default::default()
        });
          log!("Step 2!");

        let surface = instance.create_surface(window.clone()).unwrap();
          log!("Step 3!");
     
        // Adapter
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

		let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                label: None,
            },
            None, // Trace path
        ).await.unwrap();

		let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Immediate,//.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2
        };

        surface.configure(&device, &config);

        // Load Texture
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                    // This should match the filterable field of the
                    // corresponding Texture entry above.
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });
       

        let texture_names: &[&str] = &["GameAssets/SpriteSheet.png"];
        let mut texture_atlases_bind_group = Vec::<wgpu::BindGroup>::new();

        for texture_name in texture_names {

            let mut f = File::open(texture_name).expect("no file found");
            let mut texture_bytes = Vec::<u8>::new();
            match f.read_to_end(&mut texture_bytes) {
                Err(e) => { panic!("Failed to reach texture {texture_name} due to {e}"); }
                _ => (),
            }

            let texture = game_texture::Texture::from_bytes(&device, &queue, bytemuck::cast_slice(texture_bytes.as_slice()), texture_name).unwrap();
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
                        }
                    ],
                    label: Some("character_tex_bind_group"),
                }
            );

            texture_atlases_bind_group.push(tex_bind_group);
        }

        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("BasicSprite.wgsl").into()),
        });
        
        // Model Buffer
        let model_uniform = ModelUniform {
            ..Default::default()
        };

        let model_constant_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Model Constant Buffer"),
                contents: bytemuck::cast_slice(&[model_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let model_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }
            ],
            label: Some("model_bind_group_layout"),
        });

        // Render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &model_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), InstanceBuffer::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { 
                    format: config.format,
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

        let empty_instance = InstanceBuffer {
            pos_scale: [0.0, 0.0, 0.0, 0.0],
            uv_scale_bias: [0.0, 0.0, 0.0, 0.0],
        };

        let empty_instance_data = vec![empty_instance; max_instances as usize];
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&empty_instance_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
            }
        );

        let model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &model_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: model_constant_buffer.as_entire_binding(),
                }
            ],
            label: Some("model_bind_group"),
        });

        // Font
         let brush = BrushBuilder::using_font_bytes(include_bytes!("Bold.ttf")).unwrap()
                .build(&device, config.width, config.height, config.format);
 
	Self {
            surface,
            adapter,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_vertices: 3,
            num_indices: 6,
            texture_atlases_bind_group,
            model_uniform,
            model_constant_buffer,
            model_bind_group,
            instance_buffer,
            brush,
            frame_times: Vec::<f32>::new(),
            frame_timer: std::time::Instant::now(),
            frame_count: 0,
            max_instances
        }
    }

	pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
		if new_size.width > 0 && new_size.height > 0 {
			self.size = new_size;
			self.config.width = new_size.width;
			self.config.height = new_size.height;
			self.surface.configure(&self.device, &self.config);
		}
	}

	pub fn render(&mut self, game_objects: &Vec<GameObject>, elapsed_game_time: f32) -> Result<(), wgpu::SurfaceError> {
		let output = self.surface.get_current_texture()?;
		let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
		let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("Render Encoder"),
		});

        let mut frame_instances = Vec::<InstanceBuffer>::new();
        let extra_scale = 1.0;
        let extra_offset: Vector3<f32> = Vector3::<f32>::new(0.0, -0.35, 0.0);

        // Create instances
        {
            let u_scale = 1.0 / 8.0;
            let v_scale = 1.0 / 8.0;

            // Create a copy of the GameObject list that we can sort
            let game_object_iter = game_objects.iter();
            let mut render_object_list: Vec<&GameObject> = Vec::<&GameObject>::new();
            for game_object in game_object_iter {
                render_object_list.push(&game_object);
            }
            render_object_list.sort_by(|a,b| a.position.z.partial_cmp(&b.position.z).unwrap());

            // Build Instance buffer from sorted Game Object list
            let game_object_iter = render_object_list.iter();
            for game_object in game_object_iter {

                let game_object_position = game_object.position + extra_offset;
                let sprite_index = game_object.sprite_index + game_object.anim_frame;
                let mut u_offset = ((sprite_index % 8) as f32) * u_scale;
                let v_offset = ((sprite_index / 8) as f32) * v_scale;
                let mul = if game_object.direction.x > 0.0 { 1.0 } else { -1.0 };
                if mul < 0.0 {
                    u_offset = u_offset + u_scale;
                }
                 let new_instance = InstanceBuffer {
                    pos_scale: [game_object_position.x, game_object_position.y, game_object.scale.x * extra_scale, game_object.scale.y * extra_scale],
                    uv_scale_bias: [u_scale * mul, v_scale, u_offset, v_offset],
                };
                frame_instances.push(new_instance);
            }

             self.queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(frame_instances.as_slice()));
        }

        // Sprite Pass
		{
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.texture_atlases_bind_group[0], &[]);
            render_pass.set_bind_group(1, &self.model_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            self.queue.write_buffer(&self.model_constant_buffer, 0, bytemuck::cast_slice(&[self.model_uniform]));

            render_pass.draw_indexed(0..6, 0, 0..frame_instances.len() as _);

            let mut total_frame_times = 0.0;
            let frame_time_iter = self.frame_times.iter();
            for frame_time in frame_time_iter {
                total_frame_times = total_frame_times + frame_time;
            }

            let avg_frame_time = total_frame_times / (self.frame_times.len() as f32);
            let frame_rate = 1.0 / avg_frame_time;
            let frame_time_string = format!(   "FPS: {:.0} \n\
                                                Frame time: {:.2} ms\n\
                                                Num Game Objects: {}\n\
                                                Elapsed time: {:.0} secs\n\
                                                Back End: {:?}\n\
                                                Graphics: {}\n",
                                                frame_rate, avg_frame_time * 1000.0, game_objects.len(), elapsed_game_time, self.adapter.get_info().backend, self.adapter.get_info().name.as_str());
                                                
            let section = TextSection::default().add_text(Text::new(&frame_time_string));
            self.brush.resize_view(self.config.width as f32, self.config.height as f32, &self.queue);
            self.brush.queue(&self.device, &self.queue, vec![&section]).unwrap();
            self.brush.draw(&mut render_pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Frame rate update
        self.frame_count = self.frame_count + 1;
        if self.frame_count > 16 {
            let elapsed_time = self.frame_timer.elapsed().as_secs_f32();
            let avg_frame_time = elapsed_time/ (self.frame_count as f32);
            if self.frame_times.len() > 10 {
                self.frame_times.remove(0);
            }
            self.frame_times.push(avg_frame_time);

            self.frame_timer = std::time::Instant::now();
            self.frame_count = 0;
        }
        Ok(())
    }
}