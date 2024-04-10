use instant::Instant;
use wgpu::util::DeviceExt;
use wgpu_text::{glyph_brush::{Section as TextSection, Text}, BrushBuilder, TextBrush};
use ab_glyph::FontRef;

use crate::{game_texture, game_object::*, GameConfig, log};

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
pub struct DeviceResources<'a> {
    surface: wgpu::Surface<'a>,
    surface_config: wgpu::SurfaceConfiguration,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_vertices: usize,
    num_indices: usize,
    texture_atlases_bind_group: Vec<wgpu::BindGroup>,
    pub model_uniform: ModelUniform,
    model_constant_buffer: wgpu::Buffer,
    model_bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    brush: TextBrush<FontRef<'a>>,
    pub max_instances: u32,
}

#[allow(dead_code)] 
pub struct GameRenderer<'a> {
    device_resources: Option<DeviceResources<'a>>,
    pub size: winit::dpi::PhysicalSize<u32>,
    frame_times: Vec<f32>,
    frame_timer: Instant,
    frame_count: u32,
    game_config: GameConfig,
    window_id: winit::window::WindowId,
}

impl<'a> DeviceResources<'a> {
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
		if new_size.width > 0 && new_size.height > 0 {
			self.surface_config.width = new_size.width;
			self.surface_config.height = new_size.height;
			self.surface.configure(&self.device, &self.surface_config);
		}
    }

     pub async fn new(window: std::sync::Arc::<winit::window::Window>, game_config: &GameConfig) -> Self {
            
        // Instance + Surface
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: game_config.graphics_backend,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();
     
        // Adapter
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: game_config.graphics_power_pref,
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

        let size = window.inner_size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2
        };

        surface.configure(&device, &surface_config);

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
       
        let mut texture_atlases_bind_group = Vec::<wgpu::BindGroup>::new();
        let texture_bytes = include_bytes!("../game_assets/SpriteSheet.png");
        let texture = game_texture::Texture::from_bytes(&device, &queue, texture_bytes, "SpriteSheet.png").unwrap();
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
        

        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../game_assets/BasicSprite.wgsl").into()),
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

        let max_instances = game_config.max_render_instances;
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
         let brush = BrushBuilder::using_font_bytes(include_bytes!("../game_assets/Bold.ttf")).unwrap()
                .build(&device, surface_config.width, surface_config.height, surface_config.format);
 
	    DeviceResources {
            surface_config,
            surface,
            adapter,
            device,
            queue,
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
            max_instances
        }    
    }
}

impl<'a> GameRenderer<'a> {

    pub fn new(window: std::sync::Arc<winit::window::Window>, game_config: GameConfig) -> Self {
        log!("GameRenderer::new() called...");

        GameRenderer {
            device_resources: None,
            size: window.inner_size(),
            frame_times: Vec::<f32>::new(),
            frame_timer: Instant::now(),
            frame_count: 0,
            game_config,
            window_id: window.id()
        }
    }

    pub async fn init_renderer(&mut self, window: std::sync::Arc::<winit::window::Window>) {
        log!("init_renderer() called...");

        if self.device_resources.is_some()  {
            return;
        }

        self.device_resources = Some(DeviceResources::new(window, &self.game_config).await);
    }
 
	pub fn render_frame(&mut self, game_objects: &Vec<GameObject>) -> Result<(), wgpu::SurfaceError> {
        let device_resources = &mut self.device_resources.as_mut().unwrap();
		let output = device_resources.surface.get_current_texture()?;
		let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
		let mut encoder = device_resources.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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

             device_resources.queue.write_buffer(&device_resources.instance_buffer, 0, bytemuck::cast_slice(frame_instances.as_slice()));
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

            render_pass.set_pipeline(&device_resources.render_pipeline);
            render_pass.set_bind_group(0, &device_resources.texture_atlases_bind_group[0], &[]);
            render_pass.set_bind_group(1, &device_resources.model_bind_group, &[]);
            render_pass.set_vertex_buffer(0, device_resources.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, device_resources.instance_buffer.slice(..));
            render_pass.set_index_buffer(device_resources.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            device_resources.queue.write_buffer(&device_resources.model_constant_buffer, 0, bytemuck::cast_slice(&[device_resources.model_uniform]));

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
                                                frame_rate, avg_frame_time * 1000.0, game_objects.len(), 0.0, device_resources.adapter.get_info().backend, device_resources.adapter.get_info().name.as_str());
                                                
            let section = TextSection::default().add_text(Text::new(&frame_time_string));
            device_resources.brush.resize_view(self.game_config.window_width as f32, self.game_config.window_height as f32, &device_resources.queue);
            let _ = &mut device_resources.brush.queue(&device_resources.device, &device_resources.queue, vec![&section]).unwrap();
            device_resources.brush.draw(&mut render_pass);
        }

        device_resources.queue.submit(std::iter::once(encoder.finish()));
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

            self.frame_timer = Instant::now();
            self.frame_count = 0;
        }
        Ok(())
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        let device_resources = &mut self.device_resources.as_mut().unwrap();
        device_resources.resize(size);
    }

    pub fn window_id(&self) -> winit::window::WindowId {
        self.window_id
    }
}