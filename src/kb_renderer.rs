use std::sync::Arc;
use instant::Instant;
use wgpu_text::{glyph_brush::{Section as TextSection, Text}, BrushBuilder, TextBrush};
use ab_glyph::FontRef;
use cgmath::Vector3;
use crate::{kb_config::KbConfig, kb_object::{GameObject, GameObjectType}, kb_resource::*, kb_pipeline::*, log, PERF_SCOPE};


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

        let postprocess_pipeline = &mut device_resources.postprocess_pipeline;
        render_pass.set_pipeline(&postprocess_pipeline.postprocess_pipeline);
        render_pass.set_bind_group(0, &postprocess_pipeline.postprocess_bind_group, &[]);
        render_pass.set_bind_group(1, &postprocess_pipeline.postprocess_uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, device_resources.sprite_pipeline.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, device_resources.instance_buffer.slice(..));
        render_pass.set_index_buffer(device_resources.sprite_pipeline.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        postprocess_pipeline.postprocess_uniform.time_mode_unused_unused[0] = self.start_time.elapsed().as_secs_f32();
        postprocess_pipeline.postprocess_uniform.time_mode_unused_unused[1] = {
            match self.postprocess_mode {
                KbPostProcessMode::Desaturation => { 1.0 }
                KbPostProcessMode::ScanLines => { 2.0 }
                KbPostProcessMode::Warp => { 3.0 }
                _ => { 0.0 }
            }
        };

        device_resources.queue.write_buffer(&postprocess_pipeline.postprocess_constant_buffer, 0, bytemuck::cast_slice(&[postprocess_pipeline.postprocess_uniform]));

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
            let mut command_encoder = self.get_encoder("Skybox Pass (Opaque)");
            let mut device_resources = self.device_resources.as_mut().unwrap();
            device_resources.sprite_pipeline.render(&mut command_encoder, 
                &mut device_resources.queue,
                (device_resources.surface_config.width, device_resources.surface_config.height),
                (&device_resources.render_textures[0].view, &device_resources.depth_textures[0].view),
                &device_resources.instance_buffer,
                true, self.start_time, &skybox_render_objs, KbRenderPassType::Opaque);
            self.submit_encoder(command_encoder);
        }

        {
            PERF_SCOPE!("Skybox Pass (Transparent)");
            let mut command_encoder = self.get_encoder("Skybox Pass (Transparent)");
            let mut device_resources = self.device_resources.as_mut().unwrap();
            device_resources.sprite_pipeline.render(&mut command_encoder, 
                &mut device_resources.queue,
                (device_resources.surface_config.width, device_resources.surface_config.height),
                (&device_resources.render_textures[0].view, &device_resources.depth_textures[0].view),
                &device_resources.instance_buffer,
                false, self.start_time, &cloud_render_objs, KbRenderPassType::Transparent);
            self.submit_encoder(command_encoder);
        }

        {
            PERF_SCOPE!("World Objects Pass");
            let mut command_encoder = self.get_encoder("World Objects");
            let mut device_resources = self.device_resources.as_mut().unwrap();
            device_resources.sprite_pipeline.render(&mut command_encoder, 
                &mut device_resources.queue,
                (device_resources.surface_config.width, device_resources.surface_config.height),
                (&device_resources.render_textures[0].view, &device_resources.depth_textures[0].view),
                &device_resources.instance_buffer,
                false, self.start_time, &game_render_objs, KbRenderPassType::Opaque);
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