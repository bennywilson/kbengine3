use std::collections::HashMap;

//use crate::log;
use crate::kb_resource::*;

pub trait KbHandle {
	fn is_valid(&self) -> bool;
}

#[derive(Clone, Hash)] pub struct KbTextureHandle { index: u32 }
impl KbHandle for KbTextureHandle {	fn is_valid(&self) -> bool { self.index != u32::MAX } }
impl PartialEq for KbTextureHandle { fn eq(&self, other: &Self) -> bool { self.index == other.index } }
impl Eq for KbTextureHandle{}

pub struct KbAssetManager {
	names_to_texture_handles:  HashMap<String, KbTextureHandle>,
	handles_to_textures: HashMap<KbTextureHandle, KbTexture>,
	next_texture_handle: KbTextureHandle,
}

impl KbAssetManager {
	pub fn new() -> Self {
		let names_to_texture_handles = HashMap::<String, KbTextureHandle>::new();
		let handles_to_textures = HashMap::<KbTextureHandle, KbTexture>::new();
		let next_texture_handle = KbTextureHandle { index: 0 };

		KbAssetManager {
			names_to_texture_handles,
			handles_to_textures,
			next_texture_handle
		}
	}

	pub fn load_texture(&mut self, file_path: &str, device_resource: &KbDeviceResources) -> KbTextureHandle {
		match self.names_to_texture_handles.get(file_path) {
			Some(handle) => {
				return handle.clone()
			}
			_ => {}
		}

		let new_texture = KbTexture::from_file(file_path, device_resource).unwrap();
		let new_handle = self.next_texture_handle.clone();
		self.next_texture_handle.index = self.next_texture_handle.index + 1;
		self.handles_to_textures.insert(new_handle.clone(), new_texture);
		self.names_to_texture_handles.insert(file_path.to_string(), new_handle.clone());

		new_handle.clone()
	}

	pub fn get_texture(&self, texture_handle: &KbTextureHandle) -> &KbTexture {
		&self.handles_to_textures[texture_handle]
	}
}