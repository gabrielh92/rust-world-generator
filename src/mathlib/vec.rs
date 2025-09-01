#[derive(Debug, Copy, Clone)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x: x, y: y }
    }
}

impl From<Vec2> for egui::Vec2 {
	fn from(v: Vec2) -> Self {
		egui::Vec2 { x: v.x, y: v.y }
	}
}


pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
