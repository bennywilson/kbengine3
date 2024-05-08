use std::collections::HashMap;

use crate::{kb_config::*, kb_renderer::*, kb_utils::*, make_kb_handle, log};

make_kb_handle!(KbCollisionShape, KbCollisionHandle, KbCollisionMappings);

#[derive(Clone, Copy)]
pub struct KbCollisionSphere {
	pub position: CgVec3,
	pub radius: f32
}

#[derive(Clone, Copy)]
pub struct KbCollisionAABB {
	pub position: CgVec3,
	pub extents: CgVec3
}

impl KbCollisionAABB {
	pub fn max(&self) -> CgVec3 {
		self.position + self.extents
	}

	pub fn min(&self) -> CgVec3 {
		self.position - self.extents
	}
}

#[derive(Clone, Copy)]
pub enum KbCollisionShape {
	Sphere(KbCollisionSphere),
	AABB(KbCollisionAABB)
}

pub struct KbCollisionManager {
	collision_objects: KbCollisionMappings,
}

impl KbCollisionManager {
	pub fn new() -> Self {
		log!("Initializing KbCollisionManager...");
		KbCollisionManager {
			collision_objects: KbCollisionMappings::new()
		}
	}

	pub fn add_collision(&mut self, collision: &KbCollisionShape) -> KbCollisionHandle {
		let mappings = &mut self.collision_objects;
		let new_handle = {
			if mappings.next_handle.is_valid() == false { mappings.next_handle.index = 0; }
			let new_handle = mappings.next_handle.clone();
			mappings.next_handle.index = mappings.next_handle.index + 1;
			new_handle
		};
		self.collision_objects.handles_to_assets.insert(new_handle.clone(), (*collision).clone());
		new_handle.clone()
	}

	pub fn remove_collision(&mut self, handle: &KbCollisionHandle) {
		self.collision_objects.handles_to_assets.remove(handle);
	}

	pub fn update_collision_position(&mut self, handle: &KbCollisionHandle, new_pos: &CgVec3) {
		let collision = self.collision_objects.handles_to_assets.get_mut(handle).expect("Bad collision handle");

		let new_collision: KbCollisionShape;//::Sphere(KbCollisionSphere { position: CG_VEC3_ZERO, radius: 1.0 });
		match collision {
			KbCollisionShape::Sphere(s) => {
				new_collision =  KbCollisionShape::Sphere(KbCollisionSphere { position: new_pos.clone(), radius: s.radius });
			}
			KbCollisionShape::AABB(b) => {
				new_collision =  KbCollisionShape::AABB(KbCollisionAABB { position: new_pos.clone(), extents: b.extents });
			}
		}

		self.collision_objects.handles_to_assets.insert(handle.clone(), new_collision);
	}

	pub fn cast_ray(&mut self, start: &CgVec3, dir: &CgVec3) -> (bool, Option<KbCollisionHandle>) {
		
	
		let mut closest_hit = f32::MAX;
		let mut closest_handle = KbCollisionHandle::make_invalid();
		let collision_iter = self.collision_objects.handles_to_assets.iter_mut();
		for (handle, value) in collision_iter {
			match value {
				KbCollisionShape::Sphere(_s) => { }

				KbCollisionShape::AABB(aabb) => {
					let mut t_min = aabb.min() - start;
					t_min.x /= dir.x;
					t_min.y /= dir.y;
					t_min.z /= dir.z;

					let mut t_max = aabb.max() - start;
					t_max.x /= dir.x;
					t_max.y /= dir.y;
					t_max.z /= dir.z;

					let mut actual_min = CG_VEC3_ZERO;
					let mut actual_max = CG_VEC3_ZERO;
					actual_min.x = t_min.x.min(t_max.x);
					actual_max.x = t_min.x.max(t_max.x);
					actual_min.y = t_min.y.min(t_max.y);
					actual_max.y = t_min.y.max(t_max.y);
					actual_min.z = t_min.z.min(t_max.z);
					actual_max.z = t_min.z.max(t_max.z);

					let smallest_max = actual_max.x.min(actual_max.y).min(actual_max.z);
					let largest_min = actual_min.x.max(actual_min.y).max(actual_min.z);

					if largest_min > 0.0 && smallest_max >= largest_min && largest_min < closest_hit {
						closest_hit = largest_min;
						closest_handle = handle.clone();
					}
				}
			}
		}

		(closest_handle.is_valid(), Some(closest_handle))
	}

	pub fn debug_draw(&mut self, renderer: &mut KbRenderer, config: &KbConfig) {
		let collision_iter = self.collision_objects.handles_to_assets.iter_mut();

		for (_, value) in collision_iter {
			match value {
				KbCollisionShape::Sphere(_s) => { }

				KbCollisionShape::AABB(aabb) => {
					let extent_0 = aabb.position + CgVec3::new(-aabb.extents.x, aabb.extents.y, aabb.extents.z);
					let extent_1 = aabb.position + CgVec3::new(aabb.extents.x, aabb.extents.y, aabb.extents.z);
					let extent_2 = aabb.position + CgVec3::new(aabb.extents.x, -aabb.extents.y, aabb.extents.z);
					let extent_3 = aabb.position + CgVec3::new(-aabb.extents.x, -aabb.extents.y, aabb.extents.z);

					let extent_4 = aabb.position + CgVec3::new(-aabb.extents.x, aabb.extents.y, -aabb.extents.z);
					let extent_5 = aabb.position + CgVec3::new(aabb.extents.x, aabb.extents.y, -aabb.extents.z);
					let extent_6 = aabb.position + CgVec3::new(aabb.extents.x, -aabb.extents.y, -aabb.extents.z);
					let extent_7 = aabb.position + CgVec3::new(-aabb.extents.x, -aabb.extents.y, -aabb.extents.z);

					let color = CgVec4::new(1.0, 1.0, 0.0, 1.0);

					renderer.add_line(&extent_0, &extent_1, &color, 0.05, 0.001, &config);
					renderer.add_line(&extent_1, &extent_2, &color, 0.05, 0.001, &config);
					renderer.add_line(&extent_2, &extent_3, &color, 0.05, 0.001, &config);
					renderer.add_line(&extent_3, &extent_0, &color, 0.05, 0.001, &config);

					renderer.add_line(&extent_4, &extent_5, &color, 0.05, 0.001, &config);
					renderer.add_line(&extent_5, &extent_6, &color, 0.05, 0.001, &config);
					renderer.add_line(&extent_6, &extent_7, &color, 0.05, 0.001, &config);
					renderer.add_line(&extent_7, &extent_4, &color, 0.05, 0.001, &config);

					renderer.add_line(&extent_0, &extent_4, &color, 0.05, 0.001, &config);
					renderer.add_line(&extent_1, &extent_5, &color, 0.05, 0.001, &config);
					renderer.add_line(&extent_2, &extent_6, &color, 0.05, 0.001, &config);
					renderer.add_line(&extent_3, &extent_7, &color, 0.05, 0.001, &config);
				}
			}
		}
	}
}