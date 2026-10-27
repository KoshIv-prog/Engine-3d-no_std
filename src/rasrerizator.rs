use crate::{floorf, Pixel, VertexProcessed, ceilf, FRAME_WIDTH, FRAME_HEIGHT};


pub fn rasterize(vertex_idx0:&VertexProcessed,vertex_idx1:&VertexProcessed,vertex_idx2:&VertexProcessed, framebuffer:&mut [Pixel], z_buffer:&mut [f32]) {
    let area = edge(
        vertex_idx0.screen_x,
        vertex_idx0.screen_y,
        vertex_idx1.screen_x,
        vertex_idx1.screen_y,
        vertex_idx2.screen_x,
        vertex_idx2.screen_y,
    );

    if area > -0.00001 && area < 0.00001 {
        return;
    }

    let min_x = unsafe {
        floorf(
            vertex_idx0.screen_x
                .min(vertex_idx1.screen_x)
                .min(vertex_idx2.screen_x)
                .max(0.0)
        ) as i32
    };

    let max_x = unsafe {
        ceilf(
            vertex_idx0.screen_x
                .max(vertex_idx1.screen_x)
                .max(vertex_idx2.screen_x)
                .min((FRAME_WIDTH - 1) as f32)
        ) as i32
    };

    let min_y = unsafe {
        floorf(
            vertex_idx0.screen_y
                .min(vertex_idx1.screen_y)
                .min(vertex_idx2.screen_y)
                .max(0.0)
        ) as i32
    };

    let max_y = unsafe {
        ceilf(
            vertex_idx0.screen_y
                .max(vertex_idx1.screen_y)
                .max(vertex_idx2.screen_y)
                .min((FRAME_HEIGHT - 1) as f32)
        ) as i32
    };



    if min_x>max_x || min_y>max_y {
        return;
    }

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let px_half = px as f32 + 0.5;
            let py_half = py as f32 + 0.5;

            let e0 = edge(
                vertex_idx1.screen_x,
                vertex_idx1.screen_y,
                vertex_idx2.screen_x,
                vertex_idx2.screen_y,
                px_half,
                py_half,
            );

            let e1 = edge(
                vertex_idx2.screen_x,
                vertex_idx2.screen_y,
                vertex_idx0.screen_x,
                vertex_idx0.screen_y,
                px_half,
                py_half,
            );

            let e2 = edge(
                vertex_idx0.screen_x,
                vertex_idx0.screen_y,
                vertex_idx1.screen_x,
                vertex_idx1.screen_y,
                px_half,
                py_half,
            );
            if area > 0.0 {
                if e0 < 0.0 || e1 < 0.0 || e2 < 0.0 {
                    continue;
                }
            } else {
                if e0 > 0.0 || e1 > 0.0 || e2 > 0.0 {
                    continue;
                }
            }
            let inv_area = 1.0 / area;
            let w0 = e0 * inv_area;
            let w1 = e1 * inv_area;
            let w2 = e2 * inv_area;


            let pix_index = (py * FRAME_WIDTH + px) as usize;

            let inv_w =
                w0 * vertex_idx0.inv_w
                    + w1 * vertex_idx1.inv_w
                    + w2 * vertex_idx2.inv_w;

            if inv_w > z_buffer[pix_index] {
                z_buffer[pix_index] = inv_w;


                let r_over_w = w0 * vertex_idx0.r_over_w + w1 * vertex_idx1.r_over_w + w2 * vertex_idx2.r_over_w;
                let g_over_w = w0 * vertex_idx0.g_over_w + w1 * vertex_idx1.g_over_w + w2 * vertex_idx2.g_over_w;
                let b_over_w = w0 * vertex_idx0.b_over_w + w1 * vertex_idx1.b_over_w + w2 * vertex_idx2.b_over_w;

                let inv_w =
                    w0 * vertex_idx0.inv_w
                        + w1 * vertex_idx1.inv_w
                        + w2 * vertex_idx2.inv_w;

                let r = r_over_w / inv_w;
                let g = g_over_w / inv_w;
                let b = b_over_w / inv_w;

                let mut pixel = framebuffer[pix_index];

                pixel.r = (r * 255.0) as u8;
                pixel.g = (g * 255.0) as u8;
                pixel.b = (b * 255.0) as u8;

                framebuffer[pix_index] = pixel;
            }
        }
    }
}

pub fn edge(
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    px: f32,
    py: f32,
) -> f32 {
    (bx - ax) * (py - ay) - (by - ay) * (px - ax)
}