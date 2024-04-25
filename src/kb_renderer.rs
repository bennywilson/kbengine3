use std::sync::Arc;
use instant::Instant;
use wgpu::util::DeviceExt;
use wgpu_text::{glyph_brush::{Section as TextSection, Text}, BrushBuilder, TextBrush};
use ab_glyph::FontRef;
use cgmath::Vector3;
use crate::{kb_config::KbConfig, kb_object::{GameObject, GameObjectType}, kb_resource::*, log, PERF_SCOPE};

#[allow(dead_code)]
pub struct KbDeviceResources<'a> {
    surface: wgpu::Surface<'a>,
    surface_config: wgpu::SurfaceConfiguration,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    num_indices: usize,
    render_textures: Vec<KbTexture>,
    depth_textures: Vec<KbTexture>,
    sprite_resources: KbSpriteResources,
  ///  pub sprite_uniform: SpriteUniform,
    //model_constant_buffer: wgpu::Buffer,
   // model_bind_group: wgpu::BindGroup,
    postprocess_pipeline: wgpu::RenderPipeline,
    postprocess_uniform: PostProcessUniform,
    postprocess_constant_buffer: wgpu::Buffer,
    postprocess_uniform_bind_group: wgpu::BindGroup,
    postprocess_bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    brush: TextBrush<FontRef<'a>>,
    pub max_instances: u32,
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
pub struct KbRenderer<'a> {
    device_resources: Option<KbDeviceResources<'a>>,
    pub size: winit::dpi::PhysicalSize<u32>,
    postprocess_mode: KbPostProcessMode,
    start_time: Instant,
    frame_times: Vec<f32>,
    frame_timer: Instant,
    frame_count: u32,
    game_config: KbConfig,
    window_id: winit::window::WindowId,
}

impl<'a> KbDeviceResources<'a> {
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
		if new_size.width > 0 && new_size.height > 0 {
			self.surface_config.width = new_size.width;
			self.surface_config.height = new_size.height;
			self.surface.configure(&self.device, &self.surface_config);
            self.depth_textures[0] = KbTexture::new_depth_texture(&self.device, &self.surface_config);
            // todo: resize other render targets
		}
    }

     pub async fn new(window: Arc::<winit::window::Window>, game_config: &KbConfig) -> Self {
        log!("Creating instance");
        
        // Instance + Surface
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: game_config.graphics_backend,
            ..Default::default()
        });

        log!("Creating Surface.");

        let surface = instance.create_surface(window.clone()).unwrap();
    
        // Adapter
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

         let empty_instance = KbDrawInstance {
            pos_scale: [0.0, 0.0, 0.0, 0.0],
            uv_scale_bias: [0.0, 0.0, 0.0, 0.0],
            per_instance_data: [0.0, 0.0, 0.0, 0.0],
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

        let mut render_textures = Vec::<KbTexture>::new();
        let render_texture = KbTexture::new_render_texture(&device, &surface_config).unwrap();
        render_textures.push(render_texture);

        let mut depth_textures = Vec::<KbTexture>::new();
        let depth_texture = KbTexture::new_depth_texture(&device, &surface_config);
        depth_textures.push(depth_texture);

        log!("Creating Font");

        let brush = BrushBuilder::using_font_bytes(include_bytes!("../game_assets/Bold.ttf")).unwrap()
                .build(&device, surface_config.width, surface_config.height, surface_config.format);
 
                
        // Post Process Pipeline
        let postprocess_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("postprocess_uber.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../game_assets/postprocess_uber.wgsl").into()),
        });
        
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
                    // This should match the filterable field of the
                    // corresponding Texture entry above.
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
                buffers: &[KbVertex::desc(), KbDrawInstance::desc()],
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
                        let texture_bytes = include_bytes!("../game_assets/PostProcessFilter.png");
        let postprocess_texture = KbTexture::from_bytes(&device, &queue, texture_bytes, "PostProcessFilter.png").unwrap();
        let postprocess_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &postprocess_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&postprocess_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&postprocess_texture.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&render_textures[0].view),
                    },
                ],
                label: Some("postprocess_bind_group"),
            }
        );
        let sprite_resources = KbSpriteResources::new(&device, &queue, &surface_config);

	    KbDeviceResources {
            surface_config,
            surface,
            adapter,
            device,
            queue,
            num_indices: 6,
            render_textures,
            depth_textures,
            //sprite_uniform,
           // model_constant_buffer,
           // model_bind_group,
            postprocess_pipeline,
            postprocess_uniform,
            postprocess_constant_buffer,
            postprocess_uniform_bind_group,
            postprocess_bind_group,
            sprite_resources,
            instance_buffer,
            brush,
            max_instances
        }    
    }
}

impl<'a> KbRenderer<'a> {

    pub fn new(window: Arc<winit::window::Window>, game_config: KbConfig) -> Self {
        log!("GameRenderer::new() called...");

        KbRenderer {
            device_resources: None,
            size: window.inner_size(),
            start_time: Instant::now(),
            postprocess_mode: KbPostProcessMode::Passthrough,
            frame_times: Vec::<f32>::new(),
            frame_timer: Instant::now(),
            frame_count: 0,
            game_config,
            window_id: window.id()
        }
    }

    pub async fn init_renderer(&mut self, window: Arc::<winit::window::Window>) {
        log!("init_renderer() called...");

        match &self.device_resources {
            Some(_) => {}
            None => {
                self.device_resources = Some(KbDeviceResources::new(window, &self.game_config).await);
            }
        }

        log!("init_renderer() complete");
    }
 
    pub fn begin_frame(&mut self) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        PERF_SCOPE!("begin_frame())");

        let device_resources = &self.device_resources.as_mut().unwrap();

		let final_texture = device_resources.surface.get_current_texture().unwrap();
        let final_view = final_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        (final_texture, final_view)
    }

    pub fn end_frame(&self, final_tex: wgpu::SurfaceTexture) {
        PERF_SCOPE!("end_frame())");

        final_tex.present();
    }

    pub fn get_encoder(&mut self, label: &str) -> wgpu::CommandEncoder {
        let device_resources = &self.device_resources.as_mut().unwrap();
		let encoder = device_resources.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some(label),
		});

        encoder
    }

    pub fn submit_encoder(&mut self, command_encoder: wgpu::CommandEncoder) {
        let device_resources = &self.device_resources.as_mut().unwrap();
        device_resources.queue.submit(std::iter::once(command_encoder.finish()));
    }

    pub fn get_sorted_render_objects(&self, game_objects: &Vec<GameObject>) -> (Vec<GameObject>, Vec<GameObject>, Vec<GameObject>) {
        PERF_SCOPE!("sorting render objects");
        let mut skybox_render_objs = Vec::<GameObject>::new();
        let mut cloud_render_objs = Vec::<GameObject>::new();
        let mut game_render_objs = Vec::<GameObject>::new();

        for game_obj in game_objects {
            let new_game_obj = game_obj.clone();
            if matches!(game_obj.object_type, GameObjectType::Skybox) {
                skybox_render_objs.push(new_game_obj);
            } else if matches!(game_obj.object_type, GameObjectType::Cloud) {
                cloud_render_objs.push(new_game_obj.clone());
            } else {
                game_render_objs.push(new_game_obj.clone());
            }
        }
 
        skybox_render_objs.sort_by(|a,b| a.position.z.partial_cmp(&b.position.z).unwrap());
        cloud_render_objs.sort_by(|a,b| a.position.z.partial_cmp(&b.position.z).unwrap());
        game_render_objs.sort_by(|a,b| a.position.z.partial_cmp(&b.position.z).unwrap());

        (game_render_objs, skybox_render_objs, cloud_render_objs)
    }

    pub fn render_pass(&mut self, pass_type: KbRenderPassType, encoder: &mut wgpu::CommandEncoder, should_clear: bool, game_objects: &Vec<GameObject>) {
        let device_resources = &mut self.device_resources.as_mut().unwrap();
        let mut frame_instances = Vec::<KbDrawInstance>::new();

        // Create instances
        let u_scale = 1.0 / 8.0;
        let v_scale = 1.0 / 8.0;
        let extra_scale = 1.0;
        let extra_offset: Vector3<f32> = Vector3::<f32>::new(0.0, -0.35, 0.0);

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

            let new_instance = KbDrawInstance {
                pos_scale: [game_object_position.x, game_object_position.y, game_object.scale.x * extra_scale, game_object.scale.y * extra_scale],
                uv_scale_bias: [u_scale * mul, v_scale, u_offset, v_offset],
                per_instance_data: [game_object.random_val, 0.0, 0.0, 0.0],
            };
            frame_instances.push(new_instance);
        }
        
        device_resources.queue.write_buffer(&device_resources.instance_buffer, 0, bytemuck::cast_slice(frame_instances.as_slice()));
  
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

        let sprite_resources = &mut device_resources.sprite_resources;
        device_resources.queue.write_buffer(&sprite_resources.model_constant_buffer, 0, bytemuck::cast_slice(&[sprite_resources.sprite_uniform]));
  
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[color_attachment],
            depth_stencil_attachment:  Some(wgpu::RenderPassDepthStencilAttachment {
                view: &device_resources.depth_textures[0].view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        if matches!(pass_type, KbRenderPassType::Opaque) {
            render_pass.set_pipeline(&sprite_resources.opaque_render_pipeline);
        } else {
            render_pass.set_pipeline(&sprite_resources.transparent_render_pipeline);
        }

        sprite_resources.sprite_uniform.screen_dimensions = [self.game_config.window_width as f32, self.game_config.window_height as f32, (self.game_config.window_height as f32) / (self.game_config.window_width as f32), 0.0];
        sprite_resources.sprite_uniform.time[0] = self.start_time.elapsed().as_secs_f32();

        #[cfg(target_arch = "wasm32")]
        {
            sprite_resources.sprite_uniform.time[1] = 1.0 / 2.2;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            sprite_resources.sprite_uniform.time[1] = 1.0;
        }

        render_pass.set_bind_group(0, &sprite_resources.tex_bind_group, &[]);
        render_pass.set_bind_group(1, &sprite_resources.model_bind_group, &[]);
        render_pass.set_vertex_buffer(0, sprite_resources.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, device_resources.instance_buffer.slice(..));
        render_pass.set_index_buffer(sprite_resources.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..frame_instances.len() as _);
    }
    
    pub fn set_postprocess_mode(&mut self, postprocess_mode: KbPostProcessMode) { 
        self.postprocess_mode = postprocess_mode;
    }

    pub fn render_postprocess(&mut self, encoder: &mut wgpu::CommandEncoder, final_view: &wgpu::TextureView) {
        let device_resources = &mut self.device_resources.as_mut().unwrap();

        let color_attachment = Some(
            wgpu::RenderPassColorAttachment {
                view: &final_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
            }});

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[color_attachment],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&device_resources.postprocess_pipeline);
        render_pass.set_bind_group(0, &device_resources.postprocess_bind_group, &[]);
        render_pass.set_bind_group(1, &device_resources.postprocess_uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, device_resources.sprite_resources.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, device_resources.instance_buffer.slice(..));
        render_pass.set_index_buffer(device_resources.sprite_resources.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        device_resources.postprocess_uniform.time_mode_unused_unused[0] = self.start_time.elapsed().as_secs_f32();
        device_resources.postprocess_uniform.time_mode_unused_unused[1] = {
            match self.postprocess_mode {
                KbPostProcessMode::Desaturation => { 1.0 }
                KbPostProcessMode::ScanLines => { 2.0 }
                KbPostProcessMode::Warp => { 3.0 }
                _ => { 0.0 }
            }
        };

        device_resources.queue.write_buffer(&device_resources.sprite_resources.model_constant_buffer, 0, bytemuck::cast_slice(&[device_resources.postprocess_uniform]));

        render_pass.draw_indexed(0..6, 0, 0..1); 
    }

    pub fn render_debug_text(&mut self, command_encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, num_game_objects: u32) { 
        let device_resources = &mut self.device_resources.as_mut().unwrap();

        let color_attachment = {
            Some(wgpu::RenderPassColorAttachment {
                view: view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })
        };

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Text Pass"),
            color_attachments: &[color_attachment],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        let mut total_frame_times = 0.0;
        let frame_time_iter = self.frame_times.iter();
        for frame_time in frame_time_iter {
            total_frame_times = total_frame_times + frame_time;
        }

        let avg_frame_time = total_frame_times / (self.frame_times.len() as f32);
        let frame_rate = 1.0 / avg_frame_time;
        let frame_time_string = format!(   "Press [0] to disable postprocess.   [1] Desaturation    [2] Scan lines   [3]  Warp.\n\n\
                                            FPS: {:.0} \n\
                                            Frame time: {:.2} ms\n\
                                            Num Game Objects: {}\n\
                                            Elapsed time: {:.0} secs\n\
                                            Back End: {:?}\n\
                                            Graphics: {}\n",
                                            frame_rate, avg_frame_time * 1000.0, num_game_objects, 0.0, device_resources.adapter.get_info().backend, device_resources.adapter.get_info().name.as_str());

        let section = TextSection::default().add_text(Text::new(&frame_time_string));
        device_resources.brush.resize_view(self.game_config.window_width as f32, self.game_config.window_height as f32, &device_resources.queue);
        let _ = &mut device_resources.brush.queue(&device_resources.device, &device_resources.queue, vec![&section]).unwrap();
        device_resources.brush.draw(&mut render_pass);

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
    }

	pub fn render_frame(&mut self, game_objects: &Vec<GameObject>) -> Result<(), wgpu::SurfaceError> {

        PERF_SCOPE!("render_frame()");

        let (final_tex, final_view) = self.begin_frame();

       
        let (game_render_objs, skybox_render_objs, cloud_render_objs) = self.get_sorted_render_objects(game_objects);

        {
            PERF_SCOPE!("Skybox Pass (Opaque)");
            let mut command_encoder = self.get_encoder("Skybox Pass");
            self.render_pass(KbRenderPassType::Opaque, &mut command_encoder, true, &skybox_render_objs);
            self.submit_encoder(command_encoder);
        }

        {
            PERF_SCOPE!("Skybox Pass (Transparent)");
            let mut command_encoder = self.get_encoder("Transparent");
            self.render_pass(KbRenderPassType::Transparent, &mut command_encoder, false, &cloud_render_objs);
            self.submit_encoder(command_encoder);
        }

        {
            PERF_SCOPE!("World Objects Pass");
            let mut command_encoder = self.get_encoder("World Object");
            self.render_pass(KbRenderPassType::Opaque, &mut command_encoder, false, &game_render_objs);
            self.submit_encoder(command_encoder);
        }

        {
            PERF_SCOPE!("Postprocess pass");
            let mut command_encoder = self.get_encoder("Postprocess Pass");
            self.render_postprocess(&mut command_encoder, &final_view);
            self.submit_encoder(command_encoder);
        }

        {
            PERF_SCOPE!("Debug text pass");
            let mut command_encoder = self.get_encoder("Debug Text Pass");
            self.render_debug_text(&mut command_encoder, &final_view, game_objects.len() as u32);
            self.submit_encoder(command_encoder);
        }
        self.end_frame(final_tex);
  
        Ok(())
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        let device_resources = &mut self.device_resources.as_mut().unwrap();
        device_resources.resize(size);
        self.game_config.window_width = size.width;
        self.game_config.window_height = size.height;
    }

    pub fn window_id(&self) -> winit::window::WindowId {
        self.window_id
    }
}