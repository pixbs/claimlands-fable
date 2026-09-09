//! The prototype's camera and trackball, section 6.
//!
//! The camera never moves off the `+z` axis and never rolls: dragging spins the *planet*, which is
//! what makes it feel like a trackball rather than an orbit. Its world right and up are therefore
//! `+x` and `+y`, which is what `spinBy` reads out of `camera.matrixWorld`.

use glam::camera::rh::proj::directx::perspective;
use glam::camera::rh::view::look_at_mat4;
use glam::{Mat4, Quat, Vec3};

/// Vertical field of view, in degrees.
pub const FOV_DEG: f32 = 38.0;
/// Near plane.
pub const NEAR: f32 = 0.1;
/// Far plane.
pub const FAR: f32 = 50.0;
/// Closest the camera comes to the planet's centre.
pub const DIST_MIN: f32 = 1.35;
/// Furthest it goes.
pub const DIST_MAX: f32 = 6.0;
/// Where it starts.
pub const DIST_START: f32 = 3.3;
/// Radians of spin per pixel dragged.
pub const DRAG_RADIANS_PER_PIXEL: f32 = 0.0055;
/// What a flick's velocity is multiplied by each frame.
pub const INERTIA: f32 = 0.94;
/// Below this the flick has stopped.
pub const INERTIA_FLOOR: f32 = 1e-5;
/// One wheel notch multiplies the distance by `1 ± this`.
pub const WHEEL_STEP: f32 = 0.09;
/// Radians per frame of the optional idle drift.
pub const DRIFT: f32 = 0.0009;
/// A pointer that travels less than this in total counts as a tap, not a drag.
pub const TAP_SLOP: f32 = 6.0;

/// A camera on `+z` looking at the origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// Distance from the planet's centre, always within [`DIST_MIN`]`..=`[`DIST_MAX`].
    distance: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            distance: DIST_START,
        }
    }
}

impl Camera {
    /// Current distance.
    pub fn distance(&self) -> f32 {
        self.distance
    }

    /// Sets the distance, clamped to the prototype's range.
    pub fn set_distance(&mut self, distance: f32) {
        self.distance = distance.clamp(DIST_MIN, DIST_MAX);
    }

    /// One wheel notch: `camDist * (1 + sign(delta) * 0.09)`. Only the sign of `delta` counts, as
    /// in the prototype, so a trackpad's fine deltas do not zoom faster than a mouse's clicks.
    pub fn wheel(&mut self, delta: f32) {
        if delta == 0.0 {
            return;
        }
        let step = if delta > 0.0 { WHEEL_STEP } else { -WHEEL_STEP };
        self.set_distance(self.distance * (1.0 + step));
    }

    /// A pinch: the distance at the start of the gesture over how far the fingers have spread.
    pub fn pinch(&mut self, start_distance: f32, start_spread: f32, spread: f32) {
        if start_spread <= 0.0 || spread <= 0.0 {
            return;
        }
        self.set_distance(start_distance * start_spread / spread);
    }

    /// Where the camera sits.
    pub fn eye(&self) -> Vec3 {
        Vec3::new(0.0, 0.0, self.distance)
    }

    /// World to camera.
    pub fn view(&self) -> Mat4 {
        look_at_mat4(self.eye(), Vec3::ZERO, Vec3::Y)
    }

    /// Camera to clip, with the 0..1 depth range wgpu wants.
    pub fn projection(&self, aspect: f32) -> Mat4 {
        perspective(FOV_DEG.to_radians(), aspect.max(1e-6), NEAR, FAR)
    }

    /// What the scene uniform takes.
    pub fn view_proj(&self, aspect: f32) -> [f32; 16] {
        (self.projection(aspect) * self.view()).to_cols_array()
    }
}

/// The planet's orientation and the flick that is still decaying.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trackball {
    /// Orientation of the planet.
    pub orientation: Quat,
    /// Radians per frame still to be applied after a flick.
    pub velocity: (f32, f32),
}

impl Default for Trackball {
    fn default() -> Self {
        Self {
            orientation: Quat::IDENTITY,
            velocity: (0.0, 0.0),
        }
    }
}

impl Trackball {
    /// The prototype's `spinBy`: yaw about the camera's up, then pitch about its right, applied in
    /// front of the current orientation so the drag always follows the pointer whatever the planet
    /// has been turned to.
    pub fn spin_by(&mut self, dx: f32, dy: f32) {
        let q = Quat::from_axis_angle(Vec3::Y, dx) * Quat::from_axis_angle(Vec3::X, dy);
        self.orientation = (q * self.orientation).normalize();
    }

    /// Records a drag of `dx`, `dy` pointer pixels and keeps it as the flick velocity.
    pub fn drag(&mut self, dx: f32, dy: f32) {
        let k = DRAG_RADIANS_PER_PIXEL;
        self.spin_by(dx * k, dy * k);
        self.velocity = (dx * k, dy * k);
    }

    /// One frame of inertia, or of idle drift when the flick has died and `drift` is on. Returns
    /// `true` while something is still moving, so the caller knows to ask for another frame.
    pub fn step(&mut self, drift: bool) -> bool {
        if self.velocity.0.abs() > INERTIA_FLOOR || self.velocity.1.abs() > INERTIA_FLOOR {
            self.spin_by(self.velocity.0, self.velocity.1);
            self.velocity.0 *= INERTIA;
            self.velocity.1 *= INERTIA;
            return true;
        }
        self.velocity = (0.0, 0.0);
        if drift {
            self.spin_by(DRIFT, 0.0);
            return true;
        }
        false
    }

    /// Stops any flick, as pressing down does.
    pub fn hold(&mut self) {
        self.velocity = (0.0, 0.0);
    }

    /// The model matrix of anything on the planet.
    pub fn model(&self) -> [f32; 16] {
        Mat4::from_quat(self.orientation).to_cols_array()
    }
}

/// Distance between two pointers, the pinch measure.
pub fn spread(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn zoom_stays_inside_the_prototype_range() {
        let mut c = Camera::default();
        assert_eq!(c.distance(), 3.3);
        c.wheel(1.0);
        assert!((c.distance() - 3.3 * 1.09).abs() < 1e-5);
        c.wheel(-1.0);
        assert!((c.distance() - 3.3 * 1.09 * 0.91).abs() < 1e-5);
        // Only the sign counts, so a big trackpad delta is still one notch, not four hundred.
        let mut d = Camera::default();
        d.wheel(400.0);
        assert!((d.distance() - 3.3 * 1.09).abs() < 1e-5);

        for _ in 0..200 {
            c.wheel(1.0);
        }
        assert_eq!(c.distance(), DIST_MAX);
        for _ in 0..400 {
            c.wheel(-1.0);
        }
        assert_eq!(c.distance(), DIST_MIN);
    }

    #[test]
    fn pinch_matches_the_prototype_ratio() {
        let mut c = Camera::default();
        // Fingers spreading apart pull the camera in.
        c.pinch(3.0, 100.0, 200.0);
        assert_eq!(c.distance(), 1.5);
        c.pinch(3.0, 200.0, 100.0);
        assert_eq!(c.distance(), 6.0, "clamped at the far end");
        c.pinch(3.0, 0.0, 100.0);
        assert_eq!(c.distance(), 6.0, "a gesture that never started is ignored");
    }

    #[test]
    fn the_camera_looks_down_negative_z_with_x_right_and_y_up() {
        let c = Camera::default();
        let world_from_camera = c.view().inverse();
        // The columns `spinBy` reads out of `camera.matrixWorld`.
        assert!((world_from_camera.x_axis - Vec3::X.extend(0.0)).length() < 1e-6);
        assert!((world_from_camera.y_axis - Vec3::Y.extend(0.0)).length() < 1e-6);
        // The planet's centre lands in the middle of the viewport, in front of the near plane.
        let clip = c.projection(1.0) * c.view() * Vec3::ZERO.extend(1.0);
        let ndc = clip.truncate() / clip.w;
        assert!(ndc.x.abs() < 1e-6 && ndc.y.abs() < 1e-6);
        assert!((0.0..1.0).contains(&ndc.z), "inside the 0..1 depth range");
    }

    #[test]
    fn the_planet_surface_stays_in_view_across_the_zoom_range() {
        for distance in [DIST_MIN, DIST_START, DIST_MAX] {
            let mut c = Camera::default();
            c.set_distance(distance);
            // The point of the unit sphere facing the camera, and the one behind it.
            for z in [1.0f32, -1.0] {
                let clip = c.projection(1.6) * c.view() * Vec3::new(0.0, 0.0, z).extend(1.0);
                let ndc = clip.truncate() / clip.w;
                assert!(
                    (0.0..1.0).contains(&ndc.z),
                    "distance {distance}, z {z}: depth {} outside the range",
                    ndc.z
                );
            }
        }
    }

    #[test]
    fn a_drag_spins_the_planet_and_leaves_a_flick_that_decays() {
        let mut t = Trackball::default();
        t.drag(100.0, 0.0);
        // 100 px at 0.0055 rad/px, about the camera's up.
        let turned = t.orientation * Vec3::Z;
        assert!(turned.x > 0.0, "dragging right turns the near face right");
        assert!((t.velocity.0 - 0.55).abs() < 1e-6);

        let before = t.velocity.0;
        assert!(t.step(false));
        assert!((t.velocity.0 - before * INERTIA).abs() < 1e-6);

        t.hold();
        assert!(!t.step(false), "a held planet is still");
        assert!(t.step(true), "unless it is drifting");
    }

    #[test]
    fn inertia_dies_out_rather_than_ringing_forever() {
        let mut t = Trackball::default();
        t.drag(10.0, 4.0);
        let mut frames = 0;
        while t.step(false) {
            frames += 1;
            assert!(frames < 1000, "inertia never settled");
        }
        assert!(frames > 10, "and it does not stop in one frame either");
        assert_eq!(t.velocity, (0.0, 0.0));
    }

    #[test]
    fn spinning_keeps_the_orientation_a_rotation() {
        let mut t = Trackball::default();
        for i in 0..500 {
            t.drag(i as f32 % 7.0 - 3.0, i as f32 % 5.0 - 2.0);
        }
        assert!(
            (t.orientation.length() - 1.0).abs() < 1e-5,
            "normalising every spin keeps the quaternion unit"
        );
        let m = Mat4::from_quat(t.orientation);
        assert!((m.determinant() - 1.0).abs() < 1e-4, "no scale crept in");
    }

    #[test]
    fn spread_measures_the_pinch() {
        assert_eq!(spread((0.0, 0.0), (3.0, 4.0)), 5.0);
    }
}
