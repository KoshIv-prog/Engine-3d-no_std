use crate::{cosf, sinf, tanf, VertexClip, sqrtf};

#[derive(Clone, Copy)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[derive(Clone, Copy)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub fn face_normal(v0: &VertexClip, v1: &VertexClip, v2: &VertexClip) -> Vec3{
    let e1_x = v1.view_x - v0.view_x;
    let e1_y = v1.view_y - v0.view_y;
    let e1_z = v1.view_z - v0.view_z;

    let e2_x = v2.view_x - v0.view_x;
    let e2_y = v2.view_y - v0.view_y;
    let e2_z = v2.view_z - v0.view_z;

    Vec3{
        x: e1_y * e2_z - e1_z * e2_y,
        y: e1_z * e2_x - e1_x * e2_z,
        z: e1_x * e2_y - e1_y * e2_x
    }
}

pub fn normalize_vec3(vector: &Vec3) -> Vec3 {
    let length_sq =
        vector.x * vector.x
            + vector.y * vector.y
            + vector.z * vector.z;

    let length = unsafe {
        sqrtf(length_sq)
    };

    Vec3 {
        x: vector.x / length,
        y: vector.y / length,
        z: vector.z / length,
    }
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

    pub fn identity() -> Mat4 {
        Mat4 {
            m: [
                [
                    1.0,
                    0.0,
                    0.0,
                    0.0
                ],
                [
                    0.0,
                    1.0,
                    0.0,
                    0.0
                ],
                [
                    0.0,
                    0.0,
                    1.0,
                    0.0
                ],
                [
                    0.0,
                    0.0,
                    0.0,
                    1.0
                ],
            ]
        }
    }
    pub fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
        let mut result = Mat4 {
            m: [[0.0; 4]; 4],
        };

        for row in 0..4 {
            for col in 0..4 {
                let mut sum = 0.0;

                for k in 0..4 {
                    sum += a.m[row][k] * b.m[k][col];
                }

                result.m[row][col] = sum;
            }
        }

        result
    }
    pub const fn translation(x: f32, y: f32, z: f32) -> Mat4 {
        Mat4 {
            m: [
                [1.0, 0.0, 0.0, x],
                [0.0, 1.0, 0.0, y],
                [0.0, 0.0, 1.0, z],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
    pub const fn scale(x: f32, y: f32, z: f32) -> Mat4 {
        Mat4 {
            m: [
                [x, 0.0, 0.0, 0.0],
                [0.0, y, 0.0, 0.0],
                [0.0, 0.0, z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
    pub fn rotation_y(angle: f32) -> Mat4 {
        let cos = unsafe { cosf(angle) };
        let sin = unsafe { sinf(angle) };

        Mat4 {
            m: [
                [cos, 0.0, sin, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [-sin, 0.0, cos, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
    pub fn rotation_x(angle: f32) -> Mat4 {
        let cos = unsafe { cosf(angle) };
        let sin = unsafe { sinf(angle) };

        Mat4 {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, cos, -sin, 0.0],
                [0.0, sin, cos, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
    pub fn rotation_z(angle: f32) -> Mat4 {
        let cos = unsafe { cosf(angle) };
        let sin = unsafe { sinf(angle) };

        Mat4 {
            m: [
                [cos, -sin, 0.0, 0.0],
                [sin, cos, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
}