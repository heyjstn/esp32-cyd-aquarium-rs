//! 2D vector, port of `lib/Matrix/PVector.h` (PVector / Vector2<float>).

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn from_angle(angle: f32) -> Self {
        Self::new(angle.cos(), angle.sin())
    }

    pub fn is_empty(&self) -> bool {
        self.x == 0.0 && self.y == 0.0
    }

    pub fn mag(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn mag_sq(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn dist(&self, other: Vec2) -> f32 {
        (*self - other).mag()
    }

    pub fn heading(&self) -> f32 {
        self.y.atan2(self.x)
    }

    pub fn normalize(&mut self) -> &mut Self {
        let len = self.mag();
        if len == 0.0 {
            return self;
        }
        *self /= len;
        self
    }

    pub fn set_mag(&mut self, magnitude: f32) -> &mut Self {
        self.normalize();
        *self *= magnitude;
        self
    }

    pub fn limit(&mut self, max: f32) -> &mut Self {
        if self.mag_sq() > max * max {
            self.normalize();
            *self *= max;
        }
        self
    }

    pub fn lerp(&self, end: Vec2, t: f32) -> Vec2 {
        Vec2::new(
            (1.0 - t) * self.x + t * end.x,
            (1.0 - t) * self.y + t * end.y,
        )
    }
}

impl core::ops::Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl core::ops::AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Vec2) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl core::ops::Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl core::ops::SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Vec2) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl core::ops::Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, s: f32) -> Vec2 {
        Vec2::new(self.x * s, self.y * s)
    }
}

impl core::ops::MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, s: f32) {
        self.x *= s;
        self.y *= s;
    }
}

impl core::ops::Div<f32> for Vec2 {
    type Output = Vec2;
    fn div(self, s: f32) -> Vec2 {
        Vec2::new(self.x / s, self.y / s)
    }
}

impl core::ops::DivAssign<f32> for Vec2 {
    fn div_assign(&mut self, s: f32) {
        self.x /= s;
        self.y /= s;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::PI;

    #[test]
    fn from_angle_unit_length() {
        let v = Vec2::from_angle(1.234);
        assert!((v.mag() - 1.0).abs() < 1e-6);
        assert!((v.heading() - 1.234).abs() < 1e-6);
    }

    #[test]
    fn mag_and_dist() {
        let v = Vec2::new(3.0, 4.0);
        assert_eq!(v.mag(), 5.0);
        assert_eq!(v.dist(Vec2::ZERO), 5.0);
    }

    #[test]
    fn set_mag_scales() {
        let mut v = Vec2::new(3.0, 4.0);
        v.set_mag(10.0);
        assert!((v.mag() - 10.0).abs() < 1e-4);
        let mut zero = Vec2::ZERO;
        zero.set_mag(5.0);
        assert!(zero.is_empty());
    }

    #[test]
    fn limit_caps_only_above_max() {
        let mut v = Vec2::new(30.0, 40.0);
        v.limit(25.0);
        assert!((v.mag() - 25.0).abs() < 1e-4);
        let mut small = Vec2::new(1.0, 0.0);
        small.limit(25.0);
        assert_eq!(small, Vec2::new(1.0, 0.0));
    }

    #[test]
    fn lerp_endpoints() {
        let a = Vec2::new(0.0, 10.0);
        let b = Vec2::new(10.0, 20.0);
        assert_eq!(a.lerp(b, 0.0), a);
        assert_eq!(a.lerp(b, 1.0), b);
        assert_eq!(a.lerp(b, 0.5), Vec2::new(5.0, 15.0));
    }

    #[test]
    fn heading_quadrants() {
        assert!((Vec2::new(1.0, 0.0).heading() - 0.0).abs() < 1e-6);
        assert!((Vec2::new(0.0, 1.0).heading() - PI / 2.0).abs() < 1e-6);
        assert!((Vec2::new(0.0, -1.0).heading() + PI / 2.0).abs() < 1e-6);
    }
}
