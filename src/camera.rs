use crate::math::Mat4;

pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub z: f32,

    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,

    pub fov_x: f32,
    pub near: f32,
    pub far: f32,

    pub view: Mat4,
    pub projection: Mat4,
    pub view_projection: Mat4,

    pub view_dirty: bool,
    pub projection_dirty: bool,
}
impl Camera {
    pub fn new(
        fov_x: f32,
        near: f32,
        far: f32,
    ) -> Self {
        Camera {
            x: 0.0,
            y: 0.0,
            z: 0.0,

            pitch: 0.0,
            yaw: 0.0,
            roll: 0.0,

            fov_x,
            near,
            far,

            view: Mat4::identity(),
            projection: Mat4::identity(),
            view_projection: Mat4::identity(),

            view_dirty: true,
            projection_dirty: true,
        }
    }
    pub fn rebuild_view(&mut self) {
        let translation = Mat4::translation(
            -self.x,
            -self.y,
            -self.z,
        );

        let rx = Mat4::rotation_x(-self.pitch);
        let ry = Mat4::rotation_y(-self.yaw);
        let rz = Mat4::rotation_z(-self.roll);

        let temp0 = Mat4::mul(&rz, &translation);
        let temp1 = Mat4::mul(&ry, &temp0);

        self.view = Mat4::mul(&rx, &temp1);

        self.view_dirty = false;
    }
    pub fn rebuild_projection(
        &mut self,
        aspect: f32,
    ) {
        self.projection = Mat4::perspective(
            self.fov_x,
            aspect,
            self.near,
            self.far,
        );

        self.projection_dirty = false;
    }
    pub fn rebuild_view_projection(&mut self) {
        self.view_projection =
            Mat4::mul(
                &self.projection,
                &self.view,
            );
    }
}
