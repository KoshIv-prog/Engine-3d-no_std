use crate::tanf;

#[derive(Clone, Copy)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[derive(Clone, Copy)]
pub struct Mat4 {
    m: [[f32; 4]; 4],
}

impl Mat4 {
    pub fn mul_vec4(m: &Mat4, v: Vec4) -> Vec4 {
        Vec4 {
            x: m.m[0][0] * v.x
                + m.m[0][1] * v.y
                + m.m[0][2] * v.z
                + m.m[0][3] * v.w,

            y: m.m[1][0] * v.x
                + m.m[1][1] * v.y
                + m.m[1][2] * v.z
                + m.m[1][3] * v.w,

            z: m.m[2][0] * v.x
                + m.m[2][1] * v.y
                + m.m[2][2] * v.z
                + m.m[2][3] * v.w,

            w: m.m[3][0] * v.x
                + m.m[3][1] * v.y
                + m.m[3][2] * v.z
                + m.m[3][3] * v.w,
        }
    }

    pub fn perspective(
        fov_x_radians: f32,
        aspect: f32,
        near: f32,
        far: f32,
    ) -> Mat4 {
        let tan_half_fov_x =
            unsafe { tanf(fov_x_radians * 0.5) };

        Mat4 {
            m: [
                [
                    1.0 / tan_half_fov_x,
                    0.0,
                    0.0,
                    0.0,
                ],
                [
                    0.0,
                    aspect / tan_half_fov_x,
                    0.0,
                    0.0,
                ],
                [
                    0.0,
                    0.0,
                    (far + near) / (far - near),
                    -(2.0 * far * near) / (far - near),
                ],
                [
                    0.0,
                    0.0,
                    1.0,
                    0.0,
                ],
            ],
        }
    }
}