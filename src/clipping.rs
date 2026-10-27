use crate::{process_vertex, VertexClip, VertexProcessed};

pub fn near_plane_clip(
    clip0: &VertexClip,
    clip1: &VertexClip,
    clip2: &VertexClip,

    processed_cache: &mut [VertexProcessed],
    idx0: usize,
    idx1: usize,
    idx2: usize,

    out: &mut [VertexProcessed; 4],
    near:f32
) -> u8{
    let mut count = 0;

    // A -> B
    clip_edge(
        clip0,
        clip1,
        &mut processed_cache[idx1],
        out,
        &mut count,
        near,
    );

    // B -> C
    clip_edge(
        clip1,
        clip2,
        &mut processed_cache[idx2],
        out,
        &mut count,
        near,
    );

    // C -> A
    clip_edge(
        clip2,
        clip0,
        &mut processed_cache[idx0],
        out,
        &mut count,
        near,
    );

    count as u8
}
fn clip_edge(
    current: &VertexClip,
    next: &VertexClip,

    next_cache: &mut VertexProcessed,

    out: &mut [VertexProcessed; 4],
    count: &mut usize,

    near: f32,
) {
    let current_inside = current.w >= near;
    let next_inside = next.w >= near;

    match (current_inside, next_inside) {

        // inside -> inside
        (true, true) => {
            emit_original(
                next,
                next_cache,
                out,
                count,
            );
        }

        // inside -> outside
        (true, false) => {
            emit_intersection(
                current,
                next,
                out,
                count,
                near,
            );
        }

        // outside -> inside
        (false, true) => {
            emit_intersection(
                current,
                next,
                out,
                count,
                near,
            );

            emit_original(
                next,
                next_cache,
                out,
                count,
            );
        }

        // outside -> outside
        (false, false) => {}
    }
}

fn emit_original(
    clip: &VertexClip,
    cache: &mut VertexProcessed,

    out: &mut [VertexProcessed; 4],
    count: &mut usize,
) {
    if !cache.processed {
        process_vertex(
            clip,
            cache,
        );
    }

    out[*count] = *cache;
    *count += 1;
}
fn emit_intersection(
    a: &VertexClip,
    b: &VertexClip,

    out: &mut [VertexProcessed; 4],
    count: &mut usize,

    near: f32,
) {
    let t =
        (near - a.w)
            / (b.w - a.w);

    let intersection = VertexClip {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
        z: a.z + (b.z - a.z) * t,

        w: near,

        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
    };

    process_vertex(
        &intersection,
        &mut out[*count],
    );

    *count += 1;
}
