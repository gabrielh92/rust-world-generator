pub mod vec;

pub trait Normalize<T> {
	fn normalize_scalar(self, min: T, max: T) -> T;
}

impl Normalize<f32> for f32 {
	fn normalize_scalar(self, min: f32, max: f32) -> f32 {
		(self - min) / (max - min)
	}
}

impl Normalize<f64> for f64 {
	fn normalize_scalar(self, min: f64, max: f64) -> f64 {
		(self - min) / (max - min)
	}
}