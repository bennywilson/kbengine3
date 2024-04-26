use instant::Instant;
use std::{collections::HashMap, sync::Arc};
use wgpu_text::glyph_brush::{Section as TextSection, Text};

use crate::{kb_config::KbConfig, kb_game_object::{GameObject, GameObjectType, KbActor}, kb_resource::*, log, PERF_SCOPE};

#[allow(dead_code)] 
pub struct KbRenderer<'a> {
    device_resources: KbDeviceResources<'a>,
    sprite_pipeline: KbSpritePipeline,
    postprocess_pipeline: KbPostprocessPipeline,
    model_pipeline: KbModelPipeline,

    actor_map: HashMap::<u32, KbActor>,
    model: KbModel,

    postprocess_mode: KbPostProcessMode,
    frame_times: Vec<f32>,
    frame_timer: Instant,
    frame_count: u32,
    window_id: winit::window::WindowId,
}

impl<'a> KbRenderer<'a> {
    pub async fn new(window: Arc<winit::window::Window>, game_config: &KbConfig) -> Self {
        log!("GameRenderer::new() called...");
        let device_resources = KbDeviceResources::new(window.clone(), game_config).await;
        
        let device = &device_resources.device;
        let queue = &device_resources.queue;
        let surface_config = &device_resources.surface_config;

        let sprite_pipeline = KbSpritePipeline::new(&device, &queue, &surface_config, &game_config);
        let postprocess_pipeline = KbPostprocessPipeline::new(&device, &queue, &surface_config, &device_resources.render_textures[0]);
        let model_pipeline = KbModelPipeline::new(&device, &queue, &surface_config);
        let model = KbModel::new(device);

        KbRenderer {
            device_resources,
            sprite_pipeline,
            model_pipeline,
            postprocess_pipeline,

            actor_map: HashMap::<u32, KbActor>::new(),
            model,

            postprocess_mode: KbPostProcessMode::Passthrough,
            frame_times: Vec::<f32>::new(),
            frame_timer: Instant::now(),
            frame_count: 0,
            window_id: window.id()
        }
    }
 
    pub fn begin_frame(&mut self) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        PERF_SCOPE!("begin_frame())");

		let final_texture = self.device_resources.surface.get_current_texture().unwrap();
        let final_view = final_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        (final_texture, final_view)
    }

    pub fn end_frame(&self, final_tex: wgpu::SurfaceTexture) {
        PERF_SCOPE!("end_frame())");

        final_tex.present();
    }

    pub fn get_encoder(&mut self, label: &str) -> wgpu::CommandEncoder {
		let encoder = self.device_resources.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some(label),
		});

        encoder
    }

    pub fn submit_encoder(&mut self, command_encoder: wgpu::CommandEncoder) {
        self.device_resources.queue.submit(std::iter::once(command_encoder.finish()));
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

    pub fn render_debug_text(&mut self, command_encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, num_game_objects: u32, game_config: &KbConfig) { 
        let device_resources = &mut self.device_resources;

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
        device_resources.brush.resize_view(game_config.window_width as f32, game_config.window_height as f32, &device_resources.queue);
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

	pub fn render_frame(&mut self, game_objects: &Vec<GameObject>, game_config: &KbConfig) -> Result<(), wgpu::SurfaceError> {

        PERF_SCOPE!("render_frame()");

        let (final_tex, final_view) = self.begin_frame();

       
        let (game_render_objs, skybox_render_objs, cloud_render_objs) = self.get_sorted_render_objects(game_objects);

        {
            PERF_SCOPE!("Model Pass");
            self.model_pipeline.render(KbRenderPassType::Opaque, true, &self.model, &mut self.device_resources, game_config);
        }
     /*   {
            PERF_SCOPE!("Skybox Pass (Opaque)");
            self.sprite_pipeline.render(KbRenderPassType::Opaque, true, &mut self.device_resources, game_config, &skybox_render_objs);
        }

        {
            PERF_SCOPE!("Skybox Pass (Transparent)");
            self.sprite_pipeline.render(KbRenderPassType::Transparent, false, &mut self.device_resources, game_config, &cloud_render_objs);
        }

        {
            PERF_SCOPE!("World Objects Pass");
            self.sprite_pipeline.render(KbRenderPassType::Opaque, false, &mut self.device_resources, game_config, &game_render_objs);
        }*/

        {
            PERF_SCOPE!("Postprocess pass");
            self.postprocess_pipeline.render(&final_view, &mut self.device_resources, game_config);
        }

        {
            PERF_SCOPE!("Debug text pass");
            let mut command_encoder = self.get_encoder("Debug Text Pass");
            self.render_debug_text(&mut command_encoder, &final_view, game_objects.len() as u32, &game_config);
            self.submit_encoder(command_encoder);
        }

        self.end_frame(final_tex);
 
        Ok(())
    }

    pub fn resize(&mut self, game_config: &KbConfig) {
        log!("Resizing window to {} x {}", game_config.window_width, game_config.window_height);

        self.device_resources.resize(&game_config);
        
        let device = &self.device_resources.device;
        let queue = &self.device_resources.queue;
        let surface_config = &self.device_resources.surface_config;
        self.sprite_pipeline = KbSpritePipeline::new(&device, &queue, &surface_config, &game_config);
        self.postprocess_pipeline = KbPostprocessPipeline::new(&device, &queue, &surface_config, &self.device_resources.render_textures[0]);
    }

    pub fn window_id(&self) -> winit::window::WindowId {
        self.window_id
    }

    pub fn add_or_update_actor(&mut self, actor: &KbActor) {
        self.actor_map.insert(actor.id, actor.clone());
    }

    pub fn remove_actor(&mut self, actor: &KbActor) {
        self.actor_map.remove(&actor.id);
    }
}