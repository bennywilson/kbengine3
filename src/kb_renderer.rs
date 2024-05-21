use instant::Instant;
use std::{collections::HashMap, sync::Arc};
use wgpu_text::glyph_brush::{Section as TextSection, Text};

use crate::{
    kb_assets::*,
    kb_config::*,
    kb_game_object::*,
    kb_resource::*,
    kb_utils::*,
    log,
    render_groups::{
        kb_line_group::*, kb_model_group::*, kb_postprocess_group::*, kb_sprite_group::*,
        kb_sunbeam_group::*,
    },
    PERF_SCOPE,
};

#[allow(dead_code)]
pub struct KbRenderer<'a> {
    device_resources: KbDeviceResources<'a>,
    sprite_render_group: KbSpriteRenderGroup,
    postprocess_render_group: KbPostprocessRenderGroup,
    model_render_group: KbModelRenderGroup,
    line_render_group: KbLineRenderGroup,
    sunbeam_render_group: KbSunbeamRenderGroup,

    custom_world_render_groups: Vec<KbModelRenderGroup>,
    custom_foreground_render_groups: Vec<KbModelRenderGroup>,

    asset_manager: KbAssetManager,
    actor_map: HashMap<u32, KbActor>,

    particle_map: HashMap<KbParticleHandle, KbParticleActor>,
    next_particle_id: KbParticleHandle,
    active_particles: usize,

    debug_lines: Vec<KbLine>,

    game_camera: KbCamera,
    postprocess_mode: KbPostProcessMode,
    frame_times: Vec<f32>,
    frame_timer: Instant,
    frame_count: u32,
    window_id: winit::window::WindowId,

    display_debug_msg: bool,
    game_debug_msg: String,
    game_hud_msg: String,
    debug_msg_color: CgVec4,
}

impl<'a> KbRenderer<'a> {
    pub async fn new(window: Arc<winit::window::Window>, game_config: &KbConfig) -> Self {
        log!("GameRenderer::new() called...");

        let mut asset_manager = KbAssetManager::new();
        let device_resources = KbDeviceResources::new(window.clone(), game_config).await;

        let sprite_render_group =
            KbSpriteRenderGroup::new(&device_resources, &mut asset_manager, &game_config).await;
        let postprocess_render_group =
            KbPostprocessRenderGroup::new(&device_resources, &mut asset_manager).await;
        let model_render_group = KbModelRenderGroup::new(
            "/engine_assets/shaders/model.wgsl",
            &KbBlendMode::None,
            &device_resources,
            &mut asset_manager,
        )
        .await;
        let line_render_group = KbLineRenderGroup::new(
            "/engine_assets/shaders/line.wgsl",
            &device_resources,
            &mut asset_manager,
        )
        .await;
        let sunbeam_render_group =
            KbSunbeamRenderGroup::new(&device_resources, &mut asset_manager).await;
        let custom_world_render_groups = Vec::<KbModelRenderGroup>::new();
        let custom_foreground_render_groups = Vec::<KbModelRenderGroup>::new();

        let debug_lines = Vec::<KbLine>::new();

        KbRenderer {
            device_resources,
            sprite_render_group,
            model_render_group,
            postprocess_render_group,
            line_render_group,
            sunbeam_render_group,
            custom_world_render_groups,
            custom_foreground_render_groups,

            asset_manager,
            actor_map: HashMap::<u32, KbActor>::new(),
            particle_map: HashMap::<KbParticleHandle, KbParticleActor>::new(),
            next_particle_id: INVALID_PARTICLE_HANDLE,
            active_particles: 0,

            debug_lines,

            game_camera: KbCamera::new(),
            postprocess_mode: KbPostProcessMode::Passthrough,
            frame_times: Vec::<f32>::new(),
            frame_timer: Instant::now(),
            frame_count: 0,
            window_id: window.id(),

            game_debug_msg: "".to_string(),
            game_hud_msg: "".to_string(),

            display_debug_msg: false,
            debug_msg_color: CgVec4::new(0.0, 1.0, 0.0, 1.0),
        }
    }

    pub fn begin_frame(&mut self) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        PERF_SCOPE!("begin_frame())");

        let final_texture = self.device_resources.surface.get_current_texture().unwrap();
        let final_view = final_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        (final_texture, final_view)
    }

    pub fn end_frame(&self, final_tex: wgpu::SurfaceTexture) {
        PERF_SCOPE!("end_frame())");

        final_tex.present();
    }

    pub fn get_encoder(&mut self, label: &str) -> wgpu::CommandEncoder {
        let encoder = self
            .device_resources
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });

        encoder
    }

    pub fn submit_encoder(&mut self, command_encoder: wgpu::CommandEncoder) {
        self.device_resources
            .queue
            .submit(std::iter::once(command_encoder.finish()));
    }

    pub fn get_sorted_render_objects(
        &self,
        game_objects: &Vec<GameObject>,
    ) -> (Vec<GameObject>, Vec<GameObject>, Vec<GameObject>) {
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

        skybox_render_objs.sort_by(|a, b| a.position.z.partial_cmp(&b.position.z).unwrap());
        cloud_render_objs.sort_by(|a, b| a.position.z.partial_cmp(&b.position.z).unwrap());
        game_render_objs.sort_by(|a, b| a.position.z.partial_cmp(&b.position.z).unwrap());

        (game_render_objs, skybox_render_objs, cloud_render_objs)
    }

    pub fn render_debug_text(
        &mut self,
        command_encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        _num_game_objects: u32,
        game_config: &KbConfig,
    ) {
        let device_resources = &mut self.device_resources;

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Text"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        let mut total_frame_times = 0.0;
        for frame_time in &self.frame_times {
            total_frame_times = total_frame_times + frame_time;
        }
        let avg_frame_time = total_frame_times / (self.frame_times.len() as f32);
        let frame_rate = 1.0 / avg_frame_time;

        let frame_time_string = {
            if self.display_debug_msg {
                format!(
                    "Press [H] or tap here to hide help\n\
                    {}\n\\n
                    FPS: {:.0} \n\
                    Frame time: {:.2} ms\n\
                    Back End: {:?}\n\
                    Graphics: {}\n\n\
                    {}\n",
                    self.game_debug_msg,
                    frame_rate,
                    avg_frame_time * 1000.0,
                    device_resources.adapter.get_info().backend,
                    device_resources.adapter.get_info().name.as_str(),
                    self.game_hud_msg
                )
            } else {
                format!(
                    "Press [H] or tap here for help.\n\nFPS: {:.0}\n\n {}",
                    frame_rate, self.game_hud_msg
                )
            }
        };

        let section = TextSection::default()
            .add_text(
                Text::new(&frame_time_string)
                    .with_color([
                        self.debug_msg_color.x,
                        self.debug_msg_color.y,
                        self.debug_msg_color.z,
                        self.debug_msg_color.w,
                    ])
                    .with_scale(24.0 * 1.0),
            )
            .with_screen_position((10.0, 10.0));
        device_resources.brush.resize_view(
            game_config.window_width as f32,
            game_config.window_height as f32,
            &device_resources.queue,
        );
        let _ = &mut device_resources
            .brush
            .queue(
                &device_resources.device,
                &device_resources.queue,
                vec![&section],
            )
            .unwrap();
        device_resources.brush.draw(&mut render_pass);

        // Frame rate update
        self.frame_count = self.frame_count + 1;
        if self.frame_count > 16 {
            let elapsed_time = self.frame_timer.elapsed().as_secs_f32();
            let avg_frame_time = elapsed_time / (self.frame_count as f32);
            if self.frame_times.len() > 10 {
                self.frame_times.remove(0);
            }
            self.frame_times.push(avg_frame_time);

            self.frame_timer = Instant::now();
            self.frame_count = 0;
        }
    }

    pub fn render_frame(
        &mut self,
        game_objects: &Vec<GameObject>,
        game_config: &KbConfig,
    ) -> Result<(), wgpu::SurfaceError> {
        self.update_particles(game_config);
        PERF_SCOPE!("render_frame()");

        let (final_tex, final_view) = self.begin_frame();

        PERF_SCOPE!("World Opaque");
        self.model_render_group.render(
            &KbRenderGroupType::World,
            None,
            &mut self.device_resources,
            &mut self.asset_manager,
            &self.game_camera,
            &mut self.actor_map,
            game_config,
        );
        if self.actor_map.len() > 0 {
            PERF_SCOPE!("World Custom");
            for i in 0..self.custom_world_render_groups.len() {
                let render_group = &mut self.custom_world_render_groups[i];
                render_group.render(
                    &KbRenderGroupType::WorldCustom,
                    Some(i),
                    &mut self.device_resources,
                    &mut self.asset_manager,
                    &self.game_camera,
                    &mut self.actor_map,
                    game_config,
                );
            }
        }

        {
            PERF_SCOPE!("World Debug");
            self.line_render_group.render(
                &mut self.device_resources,
                &mut self.asset_manager,
                &self.game_camera,
                &mut self.debug_lines,
                game_config,
            );
        }

        if self.particle_map.len() > 0 {
            PERF_SCOPE!("World Transparent");
            self.model_render_group.render_particles(
                KbParticleBlendMode::AlphaBlend,
                &mut self.device_resources,
                &self.game_camera,
                &mut self.particle_map,
                game_config,
            );
            self.model_render_group.render_particles(
                KbParticleBlendMode::Additive,
                &mut self.device_resources,
                &self.game_camera,
                &mut self.particle_map,
                game_config,
            );
        }

        if game_config.sunbeams_enabled {
            self.sunbeam_render_group.render(
                &mut self.device_resources,
                &self.game_camera,
                game_config,
            );
        }

        if self.actor_map.len() > 0 {
            PERF_SCOPE!("Foreground Opaque");
            self.model_render_group.render(
                &KbRenderGroupType::Foreground,
                None,
                &mut self.device_resources,
                &mut self.asset_manager,
                &self.game_camera,
                &mut self.actor_map,
                game_config,
            );
            {
                PERF_SCOPE!("Foreground Custom");
                for i in 0..self.custom_foreground_render_groups.len() {
                    let render_group = &mut self.custom_foreground_render_groups[i];
                    render_group.render(
                        &KbRenderGroupType::ForegroundCustom,
                        Some(i),
                        &mut self.device_resources,
                        &mut self.asset_manager,
                        &self.game_camera,
                        &mut self.actor_map,
                        game_config,
                    );
                }
            }
        }

        let (game_render_objs, skybox_render_objs, cloud_render_objs) =
            self.get_sorted_render_objects(game_objects);

        if skybox_render_objs.len() > 0 {
            PERF_SCOPE!("Sprite Pass Sky");

            self.sprite_render_group.render(
                KbRenderPassType::Opaque,
                &mut self.device_resources,
                game_config,
                &skybox_render_objs,
            );
        }

        if cloud_render_objs.len() > 0 {
            PERF_SCOPE!("Sprite Pass Clouds");

            self.sprite_render_group.render(
                KbRenderPassType::Transparent,
                &mut self.device_resources,
                game_config,
                &cloud_render_objs,
            );
        }

        if game_render_objs.len() > 0 {
            PERF_SCOPE!("2D Game Objects");

            self.sprite_render_group.render(
                KbRenderPassType::Opaque,
                &mut self.device_resources,
                game_config,
                &game_render_objs,
            );
        }

        {
            PERF_SCOPE!("Postprocess pass");
            self.postprocess_render_group.render(
                &final_view,
                &mut self.device_resources,
                game_config,
                Some(self.postprocess_mode.clone()),
            );
        }

        {
            PERF_SCOPE!("Debug text pass");
            let mut command_encoder = self.get_encoder("Debug Text Pass");
            self.render_debug_text(
                &mut command_encoder,
                &final_view,
                game_objects.len() as u32,
                &game_config,
            );
            self.submit_encoder(command_encoder);
        }

        self.end_frame(final_tex);

        let cur_time = game_config.start_time.elapsed().as_secs_f32();
        self.debug_lines.retain_mut(|l| cur_time < l.end_time);

        Ok(())
    }

    pub async fn resize(&mut self, game_config: &KbConfig) {
        log!(
            "Resizing window to {} x {}",
            game_config.window_width,
            game_config.window_height
        );

        self.device_resources.resize(&game_config);
        self.sprite_render_group = KbSpriteRenderGroup::new(
            &self.device_resources,
            &mut self.asset_manager,
            &game_config,
        )
        .await;
        self.postprocess_render_group =
            KbPostprocessRenderGroup::new(&self.device_resources, &mut self.asset_manager).await;
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

    pub async fn add_particle_actor(
        &mut self,
        transform: &KbActorTransform,
        particle_params: &KbParticleParams,
        active: bool,
    ) -> KbParticleHandle {
        self.next_particle_id.index = {
            if self.next_particle_id.index == u32::MAX {
                0
            } else {
                self.next_particle_id.index + 1
            }
        };
        let mut particle = KbParticleActor::new(
            &transform,
            &self.next_particle_id,
            &particle_params,
            &self.device_resources,
            &mut self.asset_manager,
        )
        .await;
        particle.set_active(active);
        self.particle_map
            .insert(self.next_particle_id.clone(), particle);

        self.next_particle_id.clone()
    }

    pub fn enable_particle_actor(&mut self, handle: &KbParticleHandle, enable: bool) {
        let particle = self.particle_map.get_mut(handle).unwrap();
        particle.set_active(enable);
    }

    pub fn update_particle_transform(
        &mut self,
        handle: &KbParticleHandle,
        position: &CgVec3,
        scale: &Option<CgVec3>,
    ) {
        let particle = self.particle_map.get_mut(handle).unwrap();
        particle.set_position(&position);

        match scale {
            Some(s) => {
                particle.set_scale(s);
            }
            _ => {}
        }
    }
    pub async fn load_model(&mut self, file_path: &str) -> KbModelHandle {
        let model_handle = self
            .asset_manager
            .load_model(file_path, &mut self.device_resources)
            .await;
        model_handle
    }

    pub fn set_camera(&mut self, camera: &KbCamera) {
        self.game_camera = camera.clone();
    }

    pub fn update_particles(&mut self, game_config: &KbConfig) {
        self.active_particles = 0;
        //  let particle_iter = self.particle_map.iter_mut();
        for particle in &mut self.particle_map {
            if particle.1.is_active() {
                particle.1.tick(game_config);
                self.active_particles += 1;
            }
        }
    }

    pub async fn add_custom_render_group(
        &mut self,
        render_group_type: &KbRenderGroupType,
        blend_mode: &KbBlendMode,
        shader_path: &str,
    ) -> usize {
        let new_render_group = KbModelRenderGroup::new(
            shader_path,
            blend_mode,
            &self.device_resources,
            &mut self.asset_manager,
        )
        .await;
        let render_group: Option<KbModelRenderGroup> = Some(new_render_group);
        let handle = match *render_group_type {
            KbRenderGroupType::ForegroundCustom => {
                self.custom_foreground_render_groups
                    .push(render_group.unwrap());
                self.custom_foreground_render_groups.len()
            }

            KbRenderGroupType::WorldCustom => {
                self.custom_world_render_groups.push(render_group.unwrap());
                self.custom_world_render_groups.len()
            }

            _ => {
                panic!(
                    "KbRenderer::add_custom_render_group() - Render type {:?} not supported",
                    render_group_type
                );
            }
        } - 1;

        handle
    }

    pub fn add_line(
        &mut self,
        start: &CgVec3,
        end: &CgVec3,
        color: &CgVec4,
        thickness: f32,
        duration: f32,
        game_config: &KbConfig,
    ) {
        self.debug_lines.push(KbLine {
            start: start.clone(),
            end: end.clone(),
            color: color.clone(),
            thickness,
            end_time: game_config.start_time.elapsed().as_secs_f32() + duration,
        });
    }

    pub fn set_debug_game_msg(&mut self, msg: &str) {
        self.game_debug_msg = msg.to_string();
    }

    pub fn set_hud_msg(&mut self, msg: &str) {
        self.game_hud_msg = msg.to_string();
    }

    pub fn set_debug_font_color(&mut self, color: &CgVec4) {
        self.debug_msg_color = color.clone();
    }

    pub fn enable_help_text(&mut self) {
        self.display_debug_msg = !self.display_debug_msg;
    }

    pub fn set_postprocess_mode(&mut self, new_mode: &KbPostProcessMode) {
        self.postprocess_mode = new_mode.clone();
    }

    pub fn num_active_particles(&self) -> usize {
        self.active_particles
    }
}
