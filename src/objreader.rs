use windows_sys::core::PCWSTR;
use crate::{loader, objreader, VertexModel};
use crate::mesh::{Mesh, MeshLoadView, MeshView};

pub struct ObjCounts {
    pub vertices: usize,
    pub indices: usize,
}

pub fn count_obj(
    text: &str,
) -> Option<ObjCounts> {
    let mut vertex_count = 0usize;
    let mut index_count = 0usize;

    for line in text.lines() {
        let mut parts =
            line.split_ascii_whitespace();

        let kind = match parts.next() {
            Some(x) => x,
            None => continue,
        };

        if kind.starts_with('#') {
            continue;
        }

        match kind {
            "v" => {
                // x y z
                parts.next()?;
                parts.next()?;
                parts.next()?;

                vertex_count =
                    vertex_count.checked_add(1)?;
            }

            "f" => {
                let mut face_vertices = 0usize;

                for token in parts {
                    if token.starts_with('#') {
                        break;
                    }

                    face_vertices =
                        face_vertices.checked_add(1)?;
                }

                if face_vertices < 3 {
                    return None;
                }

                let triangle_count =
                    face_vertices - 2;

                let new_indices =
                    triangle_count.checked_mul(3)?;

                index_count =
                    index_count.checked_add(
                        new_indices
                    )?;
            }

            _ => {
                // vn, vt, mtllib, usemtl...
            }
        }
    }

    Some(ObjCounts {
        vertices: vertex_count,
        indices: index_count,
    })
}

/*
pub fn parse_obg(text :&str, mesh_view: &mut MeshLoadView) -> Option<()> {
    let mut vertex_id :usize = 0;
    let mut indice_id :usize = 0;

    for line in text.lines() {
        let mut parts = line.split_ascii_whitespace();

        match parts.next() {
            Some("v") => {

                let part =parts.next()?;
                let x = match part.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => {
                        return None;
                    }
                };
                let part =parts.next()?;
                let y = match part.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => {
                        return None;
                    }
                };
                let part =parts.next()?;
                let z = match part.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => {
                        return None;
                    }
                };
                mesh_view.vertices[vertex_id] = VertexModel {
                    x_model: x,
                    y_model: y,
                    z_model: z,
                    r: 255,
                    g: 255,
                    b: 255,
                };
                vertex_id = vertex_id.checked_add(1)?;
            }

            Some("f") => {
                for _ in 0..3 {
                    let token = parts.next()?;
                    let position_str = token.split('/').next()?;
                    let index = position_str.parse::<i32>().ok()?;

                    mesh_view.indices[indice_id] = index;
                    indice_id = indice_id.checked_add(1)?;
                }
            }
            _ => {}
        }
    }

    for i in 0..mesh_view.indices.len() {
        if mesh_view.indices[i] < 0 {
            mesh_view.indices[i] = (indice_id as i64 - mesh_view.indices[i] as i64) as i32;
        }
    }
    Some(())
}*/

pub fn parse_obj(
    text: &str,
    mesh_view: &mut MeshLoadView<'_>,
) -> Option<()> {
    let mut vertex_id = 0usize;
    let mut index_id = 0usize;

    for line in text.lines() {
        let mut parts =
            line.split_ascii_whitespace();

        let kind = match parts.next() {
            Some(x) => x,
            None => continue,
        };

        if kind.starts_with('#') {
            continue;
        }

        match kind {
            "v" => {
                if vertex_id >= mesh_view.vertices.len() {
                    return None;
                }

                let x =
                    parts.next()?
                        .parse::<f32>()
                        .ok()?;

                let y =
                    parts.next()?
                        .parse::<f32>()
                        .ok()?;

                let z =
                    parts.next()?
                        .parse::<f32>()
                        .ok()?;

                mesh_view.vertices[vertex_id] =
                    VertexModel {
                        x_model: x,
                        y_model: y,
                        z_model: z,

                        r: 255,
                        g: 255,
                        b: 255,
                    };

                vertex_id =
                    vertex_id.checked_add(1)?;
            }

            "f" => {

                let first_token =
                    parts.next()?;

                let second_token =
                    parts.next()?;

                let first =
                    parse_obj_vertex_index(
                        first_token,
                        vertex_id,
                    )?;

                let mut previous =
                    parse_obj_vertex_index(
                        second_token,
                        vertex_id,
                    )?;

                let mut created_triangle = false;

                for token in parts {
                    if token.starts_with('#') {
                        break;
                    }

                    let current =
                        parse_obj_vertex_index(
                            token,
                            vertex_id,
                        )?;

                    let next_index_id =
                        index_id.checked_add(3)?;

                    if next_index_id >
                        mesh_view.indices.len()
                    {
                        return None;
                    }

                    mesh_view.indices[index_id] =
                        first;

                    mesh_view.indices[index_id + 1] =
                        previous;

                    mesh_view.indices[index_id + 2] =
                        current;

                    index_id = next_index_id;

                    // next triangle
                    // first, current, next
                    previous = current;

                    created_triangle = true;
                }

                if !created_triangle {
                    return None;
                }
            }

            _ => {
            }
        }
    }
    
    if vertex_id != mesh_view.vertices.len() {
        return None;
    }

    if index_id != mesh_view.indices.len() {
        return None;
    }

    Some(())
}

fn parse_obj_vertex_index(
    token: &str,
    current_vertex_count: usize,
) -> Option<u32> {

    let position_str =
        token.split('/').next()?;

    if position_str.is_empty() {
        return None;
    }

    let raw =
        position_str.parse::<i32>().ok()?;

    let index: usize;

    if raw > 0 {
        index = (raw - 1) as usize;
    } else if raw < 0 {
        let resolved =
            current_vertex_count as isize
                + raw as isize;

        if resolved < 0 {
            return None;
        }

        index = resolved as usize;
    } else {
        return None;
    }


    if index >= current_vertex_count {
        return None;
    }

    if index > u32::MAX as usize {
        return None;
    }

    Some(index as u32)
}