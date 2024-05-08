//use cgmath::InnerSpace;
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