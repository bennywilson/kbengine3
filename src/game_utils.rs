#[allow(unused_macros)]
#[macro_export]
macro_rules! game_random {
    ($ty:ty, $min:literal, $max:literal) =>{
		{
			let mut buf: [u8; 4] = [0, 0, 0, 0];
			let _ = getrandom::getrandom(&mut buf);
			let mut t = buf[0] as u32;
			t = t + (buf[1] as u32) << 8;
			t = t + (buf[2] as u32) << 16;
			t = t + (buf[3] as u32) << 25;
			let t = (t as f32 / u32::MAX as f32);
			$min + ($max - $min) * (t as $ty)
		}	 
    }
}