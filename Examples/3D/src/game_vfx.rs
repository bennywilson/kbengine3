use kb_engine3::{kb_game_object::*, kb_renderer::*, kb_utils::*};

pub struct GameVfxManager {
	pooled_gib_particles: Vec<KbParticleHandle>,
	next_pooled_gib: usize,

	pooled_impact_particles: Vec<KbParticleHandle>,
	next_pooled_impact: usize,

	pooled_smoke_particles: Vec<KbParticleHandle>,
	next_pooled_smoke: usize,

	pooled_muzzle_flashes: Vec<KbParticleHandle>,
	next_muzzle_flash: usize,

	pooled_barrel_explosions: Vec<KbParticleHandle>,
	next_barrel_explosion: usize,
}

impl GameVfxManager {
	pub fn new() -> Self {
		GameVfxManager {
			pooled_gib_particles: Vec::<KbParticleHandle>::new(),
			next_pooled_gib: 0,

			pooled_impact_particles: Vec::<KbParticleHandle>::new(),
			next_pooled_impact: 0,

			pooled_smoke_particles:Vec::<KbParticleHandle>::new(),
			next_pooled_smoke: 0,

			pooled_muzzle_flashes: Vec::<KbParticleHandle>::new(),
			next_muzzle_flash: 0,

			pooled_barrel_explosions: Vec::<KbParticleHandle>::new(),
			next_barrel_explosion: 0,
		}
	}

	pub fn spawn_gibs(&mut self, gibs_position: &CgVec3, renderer: &mut KbRenderer<'_>) {
		self.next_pooled_gib = (self.next_pooled_gib + 1) % self.pooled_gib_particles.len();
		renderer.enable_particle_actor(&self.pooled_gib_particles[self.next_pooled_gib], true);
		renderer.update_particle_transform(&self.pooled_gib_particles[self.next_pooled_gib], &gibs_position, &None);
	}

	pub fn spawn_impact(&mut self, impact_position: &CgVec3, renderer: &mut KbRenderer<'_>) {
		self.next_pooled_impact = (self.next_pooled_impact + 1) % self.pooled_impact_particles.len();
		renderer.enable_particle_actor(&self.pooled_impact_particles[self.next_pooled_impact], true);
		renderer.update_particle_transform(&self.pooled_impact_particles[self.next_pooled_impact], &impact_position, &None);
	}

	pub fn spawn_barrel_smoke(&mut self, barrel_pos: &CgVec3, renderer: &mut KbRenderer<'_>) -> (KbParticleHandle, KbParticleHandle) {
		self.next_pooled_smoke = (self.next_pooled_smoke + 1) % self.pooled_smoke_particles.len();
		let particle_handle_1 = self.pooled_smoke_particles[self.next_pooled_smoke].clone();	
		renderer.enable_particle_actor(&particle_handle_1, true);
		renderer.update_particle_transform(&particle_handle_1, &barrel_pos, &None);

		// Ember
		self.next_pooled_smoke = (self.next_pooled_smoke + 1) % self.pooled_smoke_particles.len();
		let particle_handle_2 = self.pooled_smoke_particles[self.next_pooled_smoke].clone();
		renderer.enable_particle_actor(&particle_handle_2, true);
		renderer.update_particle_transform(&particle_handle_2, &barrel_pos, &None);

		(particle_handle_1, particle_handle_2)
	}

	pub fn spawn_explosion(&mut self, explosion_position: &CgVec3, renderer: &mut KbRenderer<'_>) {
		self.next_barrel_explosion = (self.next_barrel_explosion + 1) % self.pooled_barrel_explosions.len();
		renderer.enable_particle_actor(&self.pooled_barrel_explosions[self.next_barrel_explosion], true);
		renderer.update_particle_transform(&self.pooled_barrel_explosions[self.next_barrel_explosion], &explosion_position, &None);
	}

	pub fn spawn_muzzle_flash(&mut self, position: &CgVec3, scale: &CgVec3, renderer: &mut KbRenderer<'_>) {
		self.next_muzzle_flash = (self.next_muzzle_flash + 1) % self.pooled_muzzle_flashes.len();
		renderer.enable_particle_actor(&self.pooled_muzzle_flashes[self.next_muzzle_flash], true);
		renderer.update_particle_transform(&self.pooled_muzzle_flashes[self.next_muzzle_flash], position, &Some(*scale));
	}

	pub fn update_muzzle_flashes(&mut self, position: &CgVec3, renderer: &mut KbRenderer) {
		for muzzle_flash in &mut self.pooled_muzzle_flashes {
			renderer.update_particle_transform(&muzzle_flash, &position, &None);
		}
	}

	pub fn spawn_mob_death_fx(&mut self, mob_pos: &CgVec3, renderer: &mut KbRenderer<'_>) {
		self.spawn_gibs(&mob_pos, renderer);
	}

	pub async fn init(&mut self, renderer: &mut KbRenderer<'_>) {

		// Pooled gibs
		let particle_params = KbParticleParams {
			texture_file: "/game_assets/fx/monster_gibs_t.png".to_string(),
			blend_mode: KbParticleBlendMode::AlphaBlend,

			min_burst_count: 75,
			max_burst_count: 100,

			min_particle_life: 0.1,
			max_particle_life: 0.75,

			_min_actor_life: 1.5,
			_max_actor_life: 1.5,

			min_start_spawn_rate: 9999.0,
			max_start_spawn_rate: 9999.0,

			min_start_pos: CgVec3::new(-0.5, -0.2, -0.2),
			max_start_pos: CgVec3::new(0.5, 0.2, 0.2),

			min_start_scale: CgVec3::new(0.05, 0.05, 0.05),
			max_start_scale: CgVec3::new(0.45, 0.45, 0.45),

			min_end_scale: CgVec3::new(0.5, 0.5, 0.5),
			max_end_scale: CgVec3::new(2.0, 2.0, 2.0),

			min_start_velocity: CgVec3::new(-10.0, -10.0, -10.0),
			max_start_velocity: CgVec3::new(10.0, 20.0, 10.0),

			min_start_rotation_rate: -0.00,
			max_start_rotation_rate: 0.00,

			min_start_acceleration: CgVec3::new(0.0, -35.0, 0.0),
			max_start_acceleration: CgVec3::new(0.0, -35.0, 0.0),

			min_end_velocity: CgVec3::new(0.0, 0.0, 0.0),
			max_end_velocity: CgVec3::new(0.0, 0.0, 0.0),

			start_color_0: CgVec4::new(0.9, 0.9, 0.9, 1.0),
			start_color_1: CgVec4::new(1.0, 1.0, 1.0, 1.0),

			end_color_0: CgVec4::new(0.0, 0.0, 0.0, 0.0),
			_end_color1: CgVec4::new(0.0, 0.0, 0.0, 0.0),
		};
		let particle_transform = KbActorTransform::from_position(CgVec3::new(3.0, 3.5, 0.0));
		for _ in 0..20 {
			let particle_handle = renderer.add_particle_actor(&particle_transform, &particle_params, false).await;
			self.pooled_gib_particles.push(particle_handle);
		}

		// Pooled Impacts
		let particle_params = KbParticleParams {
			texture_file: "/game_assets/fx/smoke_t.png".to_string(),
			blend_mode: KbParticleBlendMode::AlphaBlend,

			min_burst_count: 100,
			max_burst_count: 100,

			min_particle_life: 0.1,
			max_particle_life: 0.15,

			_min_actor_life: 1.5,
			_max_actor_life: 1.5,

			min_start_spawn_rate: 9999.0,
			max_start_spawn_rate: 9999.0,

			min_start_pos: CgVec3::new(-0.05, -0.05, -0.05),
			max_start_pos: CgVec3::new(0.05, 0.05, 0.05),

			min_start_scale: CgVec3::new(0.05, 0.05, 0.05),
			max_start_scale: CgVec3::new(0.15, 0.15, 0.15),

			min_end_scale: CgVec3::new(0.15, 0.15, 0.15),
			max_end_scale: CgVec3::new(0.3, 0.3, 0.3),

			min_start_velocity: CgVec3::new(-10.0, -10.0, -10.0),
			max_start_velocity: CgVec3::new(10.0, 10.0, 10.0),

			min_start_rotation_rate: -0.03,
			max_start_rotation_rate: 0.03,

			min_start_acceleration: CgVec3::new(0.0, -5.0, 0.0),
			max_start_acceleration: CgVec3::new(0.0, -5.0, 0.0),

			min_end_velocity: CgVec3::new(0.0, 0.0, 0.0),
			max_end_velocity: CgVec3::new(0.0, 0.0, 0.0),

			start_color_0: CgVec4::new(0.7, 0.7, 0.7, 1.0),
			start_color_1: CgVec4::new(0.9, 0.8, 0.8, 1.0),

			end_color_0: CgVec4::new(0.7, 0.7, 0.7, 0.0),
			_end_color1: CgVec4::new(0.9, 0.8, 0.8, 0.0),
		};
		let particle_transform = KbActorTransform::from_position(CgVec3::new(3.0, 3.5, 0.0));
		for _ in 0..20 {
			let particle_handle = renderer.add_particle_actor(&particle_transform, &particle_params, false).await;
			self.pooled_impact_particles.push(particle_handle);
		}

		// Pooled smoke
		let particle_smoke_params = KbParticleParams {
			texture_file: "/game_assets/fx/smoke_t.png".to_string(),
			blend_mode: KbParticleBlendMode::AlphaBlend,

			min_burst_count: 0,
			max_burst_count: 0,

			min_particle_life: 3.0,
			max_particle_life: 5.0,

			_min_actor_life: -1.0,
			_max_actor_life: -1.0,

			min_start_spawn_rate: 0.06,
			max_start_spawn_rate: 0.06,

			min_start_pos: CgVec3::new(-0.5, -0.2, -0.2),
			max_start_pos: CgVec3::new(0.5, 0.2, 0.2),

			min_start_scale: CgVec3::new(0.5, 0.5, 0.5),
			max_start_scale: CgVec3::new(0.8, 0.8, 0.8),

			min_end_scale: CgVec3::new(2.1, 2.1, 2.1),
			max_end_scale: CgVec3::new(3.0, 3.0, 3.0),

			min_start_velocity: CgVec3::new(-0.2, 1.0, -0.2),
			max_start_velocity: CgVec3::new(0.2, 1.0, 0.2),

			min_start_rotation_rate: -0.5,
			max_start_rotation_rate: 0.5,

			min_start_acceleration: CgVec3::new(0.0, -0.1, 0.0),
			max_start_acceleration: CgVec3::new(0.0, -0.1, 0.0),

			min_end_velocity: CgVec3::new(0.0, 0.0, 0.0),
			max_end_velocity: CgVec3::new(0.0, 0.0, 0.0),

			start_color_0: CgVec4::new(0.4, 0.04, 0.0, 1.0),
			start_color_1: CgVec4::new(0.4, 0.07, 0.0, 1.0),

			end_color_0: CgVec4::new(-0.5, -0.5, -0.5, 0.0),
			_end_color1: CgVec4::new(-0.5, -0.5, -0.5, 1.0),
		};
		let particle_transform = KbActorTransform::from_position(CgVec3::new(0.0, 3.5, 0.0));
		let _ = renderer.add_particle_actor(&particle_transform, &particle_params, true).await;

		let particle_ember_params = KbParticleParams {
			texture_file: "./game_assets/fx/ember_t.png".to_string(),
			blend_mode: KbParticleBlendMode::Additive,

			min_burst_count: 0,
			max_burst_count: 0,

			min_particle_life: 1.5,
			max_particle_life: 2.5,

			_min_actor_life: -1.0,
			_max_actor_life: -1.0,

			min_start_spawn_rate: 0.3,
			max_start_spawn_rate: 0.3,

			min_start_pos: CgVec3::new(-0.75, -0.2, -0.75),
			max_start_pos: CgVec3::new(0.75, 0.2, 0.75),
    
			min_start_scale: CgVec3::new(0.3, 0.3, 0.3),
			max_start_scale: CgVec3::new(0.5, 0.5, 0.5),

			min_end_scale: CgVec3::new(0.0, 0.0, 0.0),
			max_end_scale: CgVec3::new(0.05, 0.05, 0.05),

			min_start_velocity: CgVec3::new(-0.2, 3.0, -0.2),
			max_start_velocity: CgVec3::new(0.2, 3.0, 0.2),

			min_start_rotation_rate: -15.5,
			max_start_rotation_rate: 15.5,

			min_start_acceleration: CgVec3::new(0.0, -0.1, 0.0),
			max_start_acceleration: CgVec3::new(0.0, -0.1, 0.0),

			min_end_velocity: CgVec3::new(0.0, 0.0, 0.0),
			max_end_velocity: CgVec3::new(0.0, 0.0, 0.0),

			start_color_0: CgVec4::new(2.0, 1.0, 0.2, 1.0),
			start_color_1: CgVec4::new(2.0, 1.0, 0.2, 1.0),

			end_color_0: CgVec4::new(1.0, 0.8, -0.1, 0.0),
			_end_color1: CgVec4::new(1.0, 0.8, -0.1, 1.0),
		};
		let particle_transform = KbActorTransform::from_position(CgVec3::new(0.0, 3.5, 0.0));
		let _ = renderer.add_particle_actor(&particle_transform, &particle_params, true).await;

		for _ in 0..20 {
			let particle_handle = renderer.add_particle_actor(&particle_transform, &particle_smoke_params, false).await;
			self.pooled_smoke_particles.push(particle_handle);

			let particle_handle = renderer.add_particle_actor(&particle_transform, &particle_ember_params, false).await;
			self.pooled_smoke_particles.push(particle_handle);
		}

		// Pooled Muzzle Flashes
		let muzzle_flash_params = KbParticleParams {
			texture_file: "/game_assets/fx/muzzle_flash_t.png".to_string(),
			blend_mode: KbParticleBlendMode::Additive,

			min_burst_count: 1,
			max_burst_count: 1,

			min_particle_life: 0.1,
			max_particle_life: 0.15,

			_min_actor_life: 1.0,
			_max_actor_life: 1.0,

			min_start_spawn_rate: 999.06,
			max_start_spawn_rate: 999.06,

			min_start_pos: CgVec3::new(0.0, 0.0, 0.0),
			max_start_pos: CgVec3::new(0.0, 0.0, 0.0),

			min_start_scale: CgVec3::new(1.0, 1.0, 1.0),
			max_start_scale: CgVec3::new(1.25, 1.25, 1.25),

			min_end_scale: CgVec3::new(0.2, 0.2, 0.2),
			max_end_scale: CgVec3::new(0.3, 0.3, 0.3),

			min_start_velocity: CgVec3::new(0.0, 0.0, 0.0),
			max_start_velocity: CgVec3::new(0.0, 0.0, 0.0),

			min_start_rotation_rate: 0.0,
			max_start_rotation_rate: 0.0,

			min_start_acceleration: CgVec3::new(0.0, 0.0, 0.0),
			max_start_acceleration: CgVec3::new(0.0, 0.0, 0.0),

			min_end_velocity: CgVec3::new(0.0, 0.0, 0.0),
			max_end_velocity: CgVec3::new(0.0, 0.0, 0.0),

			start_color_0: CgVec4::new(1.0, 1.0, 1.0, 1.0),
			start_color_1: CgVec4::new(1.0, 1.0, 1.0, 1.0),

			end_color_0: CgVec4::new(0.8, 0.9, 1.0, 1.0),
			_end_color1: CgVec4::new(1.0, 1.0, 1.5, 1.0),
		};

		for _ in 0..24 {
			let particle_handle = renderer.add_particle_actor(&particle_transform, &muzzle_flash_params, false).await;
			self.pooled_muzzle_flashes.push(particle_handle);
		}

		// Barrel Explosions
		let barrel_explosion_params = KbParticleParams {
			texture_file: "/game_assets/fx/smoke_t.png".to_string(),
			blend_mode: KbParticleBlendMode::AlphaBlend,

			min_burst_count: 75,
			max_burst_count: 150,

			min_particle_life: 3.0,
			max_particle_life: 5.0,

			_min_actor_life: -1.0,
			_max_actor_life: -1.0,

			min_start_spawn_rate: 99999.06,
			max_start_spawn_rate: 99999.06,

			min_start_pos: CgVec3::new(-0.5, -0.2, -0.2),
			max_start_pos: CgVec3::new(0.5, 0.2, 0.2),

			min_start_scale: CgVec3::new(0.5, 0.5, 0.5),
			max_start_scale: CgVec3::new(0.8, 0.8, 0.8),

			min_end_scale: CgVec3::new(2.1, 2.1, 2.1),
			max_end_scale: CgVec3::new(3.0, 3.0, 3.0),

			min_start_velocity: CgVec3::new(-10.0, 0.1, -10.0),
			max_start_velocity: CgVec3::new(10.0, 20.0, 10.0),

			min_start_rotation_rate: -0.5,
			max_start_rotation_rate: 0.5,

			min_start_acceleration: CgVec3::new(0.0, -25.0, 0.0),
			max_start_acceleration: CgVec3::new(0.0, -25.0, 0.0),

			min_end_velocity: CgVec3::new(0.0, 0.0, 0.0),
			max_end_velocity: CgVec3::new(0.0, 0.0, 0.0),

			start_color_0: CgVec4::new(1.0, 0.2, 0.0, 1.0),
			start_color_1: CgVec4::new(0.5, 0.1, 0.0, 1.0),

			end_color_0: CgVec4::new(-0.5, -0.5, -0.5, 0.0),
			_end_color1: CgVec4::new(-0.5, -0.5, -0.5, 1.0),
		};
		let particle_transform = KbActorTransform::from_position(CgVec3::new(0.0, 3.5, 0.0));

		for _ in 0..24 {
			let particle_handle = renderer.add_particle_actor(&particle_transform, &barrel_explosion_params, false).await;
			self.pooled_barrel_explosions.push(particle_handle);
		}

	}
}
