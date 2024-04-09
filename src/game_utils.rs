#[allow(unused_macros)]
#[macro_export]
macro_rules! game_random {
    ($min:literal, $max:literal, $ty:ty) =>{
		{
			let mut buf: [u8; 4] = [0, 0, 0, 0];
			let _ = getrandom::getrandom(&mut buf);
			let byte_array: [u8; 4] = buf.try_into().expect("Needed 4 bytes for a float");
			let mut t = buf[0] as u32;
			t = t + (buf[1] as u32) << 8;
			t = t + (buf[2] as u32) << 16;
			t = t + (buf[3] as u32) << 25;
			let t = (t as f32 / u32::MAX as f32);

			log!("t = {} -- {} {} {} {}", t, buf[0], buf[1], buf[2], buf[3]);
			$min + ($max - $min) * (t as $ty)
		}	 
    }
}