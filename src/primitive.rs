use super::model::Model;
use super::vertex::Vertex;

/// Generate a quad primitive model.
pub fn create_quad() -> Model {
    Model::from(
        vec![
            Vertex { pos: [ 1f32, -1f32], uv: [1.0, 1.0] },
            Vertex { pos: [-1f32, -1f32], uv: [0.0, 1.0] },
            Vertex { pos: [-1f32,  1f32], uv: [0.0, 0.0] },
            Vertex { pos: [ 1f32,  1f32], uv: [1.0, 0.0] },
        ],
        vec![
            0, 1, 2,
            2, 3, 0,
        ],
    )
}
