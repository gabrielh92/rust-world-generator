use std::hash::Hash;
use std::ops::{Add, Sub, Mul, Neg};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn distance(self, other: Vec2) -> f32 {
        (self - other).length()
    }

    pub fn normalized(self) -> Vec2 {
        let len = self.length();
        if len < 1e-8 { Vec2::ZERO } else { Vec2::new(self.x / len, self.y / len) }
    }

    pub fn dot(self, other: Vec2) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub fn lerp(self, other: Vec2, t: f32) -> Vec2 {
        Vec2::new(self.x + (other.x - self.x) * t, self.y + (other.y - self.y) * t)
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Vec2 { Vec2::new(self.x + rhs.x, self.y + rhs.y) }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Vec2 { Vec2::new(self.x - rhs.x, self.y - rhs.y) }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: f32) -> Vec2 { Vec2::new(self.x * rhs, self.y * rhs) }
}

impl Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 { Vec2::new(-self.x, -self.y) }
}

impl From<Vec2> for egui::Vec2 {
    fn from(v: Vec2) -> Self { egui::Vec2 { x: v.x, y: v.y } }
}

impl From<Vec2> for egui::Pos2 {
    fn from(v: Vec2) -> Self { egui::Pos2 { x: v.x, y: v.y } }
}

impl Eq for Vec2 {}

impl Hash for Vec2 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.x.to_bits().hash(state);
        self.y.to_bits().hash(state);
    }
}

#[allow(dead_code)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
