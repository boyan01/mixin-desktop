//! Image-space zoom and viewport bounds, independent of native input delivery.

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Camera {
    pub image: (f32, f32),
    pub viewport: (f32, f32),
    pub scale: f32,
    pub origin: (f32, f32),
    pub rotation: u8,
}

impl Camera {
    pub fn new(image: (f32, f32), viewport: (f32, f32)) -> Self {
        let mut camera = Self {
            image: valid_size(image),
            viewport: valid_size(viewport),
            scale: 1.0,
            origin: (0.0, 0.0),
            rotation: 0,
        };
        camera.fit();
        camera
    }

    pub fn dimensions(self) -> (f32, f32) {
        if self.rotation.is_multiple_of(2) {
            self.image
        } else {
            (self.image.1, self.image.0)
        }
    }

    pub fn rendered(self) -> (f32, f32) {
        let (w, h) = self.dimensions();
        (w * self.scale, h * self.scale)
    }

    pub fn fit_scale(self) -> f32 {
        let (w, h) = self.dimensions();
        (self.viewport.0 / w).min(self.viewport.1 / h)
    }

    pub fn limits(self) -> (f32, f32) {
        (
            (self.fit_scale() / 8.0).min(0.125),
            self.fit_scale().max(1.0) * 16.0,
        )
    }

    pub fn center(self) -> (f32, f32) {
        (self.viewport.0 / 2.0, self.viewport.1 / 2.0)
    }

    pub fn fit(&mut self) {
        self.scale = self.fit_scale();
        let (w, h) = self.rendered();
        self.origin = ((self.viewport.0 - w) / 2.0, (self.viewport.1 - h) / 2.0);
    }

    /// Keep the same image point under the pointer, except where a viewport edge
    /// requires clamping. A pointer in a letterbox anchors the nearest image edge.
    pub fn zoom(&mut self, factor: f32, pointer: (f32, f32)) {
        if !factor.is_finite() || factor <= 0.0 || !finite(pointer) {
            return;
        }
        let (min, max) = self.limits();
        let next = (self.scale * factor).clamp(min, max);
        if next == self.scale {
            return;
        }
        let (w, h) = self.rendered();
        let anchor = (
            pointer.0.clamp(self.origin.0, self.origin.0 + w),
            pointer.1.clamp(self.origin.1, self.origin.1 + h),
        );
        let ratio = next / self.scale;
        self.origin = (
            anchor.0 - (anchor.0 - self.origin.0) * ratio,
            anchor.1 - (anchor.1 - self.origin.1) * ratio,
        );
        self.scale = next;
        self.clamp();
    }

    pub fn pan(&mut self, delta: (f32, f32)) {
        if !finite(delta) {
            return;
        }
        self.origin.0 += delta.0;
        self.origin.1 += delta.1;
        self.clamp();
    }

    pub fn resize(&mut self, viewport: (f32, f32)) {
        let viewport = valid_size(viewport);
        if viewport == self.viewport {
            return;
        }
        let fitting = (self.scale - self.fit_scale()).abs() < f32::EPSILON * 8.0;
        let previous = self.center();
        self.viewport = viewport;
        if fitting {
            self.fit();
        } else {
            let next = self.center();
            self.origin.0 += next.0 - previous.0;
            self.origin.1 += next.1 - previous.1;
            self.zoom(1.0, next);
            self.clamp();
        }
    }

    pub fn turn(&mut self) {
        let fitting = (self.scale - self.fit_scale()).abs() < f32::EPSILON * 8.0;
        let center = self.center();
        let (w, h) = self.rendered();
        let point = (
            (center.0 - self.origin.0) / w,
            (center.1 - self.origin.1) / h,
        );
        self.rotation = (self.rotation + 1) % 4;
        if fitting {
            self.fit();
        } else {
            let (min, max) = self.limits();
            self.scale = self.scale.clamp(min, max);
            let (w, h) = self.rendered();
            self.origin = (center.0 - (1.0 - point.1) * w, center.1 - point.0 * h);
            self.clamp();
        }
    }

    fn clamp(&mut self) {
        let rendered = self.rendered();
        let bound = |origin: f32, image: f32, viewport: f32| {
            if image <= viewport {
                (viewport - image) / 2.0
            } else {
                origin.clamp(viewport - image, 0.0)
            }
        };
        self.origin = (
            bound(self.origin.0, rendered.0, self.viewport.0),
            bound(self.origin.1, rendered.1, self.viewport.1),
        );
    }
}

fn finite(point: (f32, f32)) -> bool {
    point.0.is_finite() && point.1.is_finite()
}

fn valid_size(size: (f32, f32)) -> (f32, f32) {
    let valid = |value: f32| {
        if value.is_finite() && value > 0.0 {
            value
        } else {
            1.0
        }
    };
    (valid(size.0), valid(size.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: f32, right: f32) {
        assert!((left - right).abs() < 0.01, "{left} != {right}");
    }

    fn bounded(camera: Camera) {
        let rendered = camera.rendered();
        for (origin, image, viewport) in [
            (camera.origin.0, rendered.0, camera.viewport.0),
            (camera.origin.1, rendered.1, camera.viewport.1),
        ] {
            assert!(origin.is_finite());
            if image <= viewport {
                close(origin, (viewport - image) / 2.0);
            } else {
                assert!(origin <= 0.01 && origin >= viewport - image - 0.01);
            }
        }
    }

    #[test]
    fn zooming_a_panned_image_keeps_the_pointer_image_point() {
        let mut camera = Camera::new((1600.0, 1200.0), (800.0, 600.0));
        camera.zoom(3.0, (400.0, 300.0));
        camera.pan((120.0, -80.0));
        let pointer = (175.0, 425.0);
        let before = (
            (pointer.0 - camera.origin.0) / camera.scale,
            (pointer.1 - camera.origin.1) / camera.scale,
        );
        camera.zoom(1.2, pointer);
        close((pointer.0 - camera.origin.0) / camera.scale, before.0);
        close((pointer.1 - camera.origin.1) / camera.scale, before.1);
    }

    #[test]
    fn repeated_zoom_pan_rotation_and_resize_never_exposes_an_unnecessary_edge() {
        for image in [
            (1600.0, 1200.0),
            (300.0, 2400.0),
            (4000.0, 80.0),
            (32.0, 32.0),
        ] {
            let mut camera = Camera::new(image, (800.0, 600.0));
            for pointer in [(0.0, 0.0), (800.0, 600.0), (200.0, 450.0), (-100.0, 1000.0)] {
                for factor in [1.2, 500.0, 0.5, 0.0001, 2.0] {
                    camera.zoom(factor, pointer);
                    camera.pan((50000.0, -50000.0));
                    bounded(camera);
                    camera.turn();
                    bounded(camera);
                    camera.resize((480.0, 720.0));
                    bounded(camera);
                    camera.resize((800.0, 600.0));
                    bounded(camera);
                }
            }
        }
    }

    #[test]
    fn zooming_past_a_limit_does_not_move_the_image() {
        let mut camera = Camera::new((1600.0, 1200.0), (800.0, 600.0));
        for (factor, pointer) in [(1e10, (0.0, 0.0)), (1e-10, (800.0, 600.0))] {
            camera.zoom(factor, pointer);
            let at_limit = camera;
            camera.zoom(factor, (200.0, 100.0));
            assert_eq!(camera, at_limit);
        }
    }

    #[test]
    fn resizing_a_fitted_image_refits_but_a_zoomed_image_preserves_the_center() {
        let mut camera = Camera::new((1600.0, 1200.0), (800.0, 600.0));
        camera.resize((400.0, 300.0));
        close(camera.scale, 0.25);
        camera.zoom(4.0, camera.center());
        let before = (
            (200.0 - camera.origin.0) / camera.scale,
            (150.0 - camera.origin.1) / camera.scale,
        );
        camera.resize((600.0, 400.0));
        close((300.0 - camera.origin.0) / camera.scale, before.0);
        close((200.0 - camera.origin.1) / camera.scale, before.1);
    }

    #[test]
    fn invalid_input_does_not_corrupt_the_camera() {
        let mut camera = Camera::new((f32::NAN, 0.0), (0.0, f32::INFINITY));
        let before = camera;
        camera.zoom(f32::NAN, (0.0, 0.0));
        camera.zoom(2.0, (f32::INFINITY, 0.0));
        camera.pan((f32::NAN, 1.0));
        assert_eq!(camera, before);
        bounded(camera);
    }

    #[test]
    fn resizing_and_rotation_reapply_zoom_limits_without_accumulating_input() {
        let mut camera = Camera::new((8000.0, 200.0), (20.0, 20.0));
        camera.zoom(0.0001, camera.center());
        camera.resize((2000.0, 1000.0));
        assert!(camera.scale >= camera.limits().0);
        camera.turn();
        assert!(camera.scale >= camera.limits().0);
        bounded(camera);
        camera.zoom(1e10, camera.center());
        let at_maximum = camera.scale;
        camera.zoom(0.9, camera.center());
        assert!(camera.scale < at_maximum);
    }
}
