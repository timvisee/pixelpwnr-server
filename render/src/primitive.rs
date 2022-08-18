use super::model::Model;
use super::vertex::Vertex;

/// Create a quad primitive model, that fills the screen in the
/// default projection.
pub fn create_quad_max() -> Model {
    create_quad((-1f32, -1f32), (2f32, 2f32))
}

/// Generate a quad primitive model.
pub fn create_quad(pos: (f32, f32), size: (f32, f32)) -> Model {
    Model::from(
        vec![
            Vertex { pos: [ pos.0 + size.0, pos.1         ], uv: [1.0, 1.0] },
            Vertex { pos: [ pos.0         , pos.1         ], uv: [0.0, 1.0] },
            Vertex { pos: [ pos.0         , pos.1 + size.1], uv: [0.0, 0.0] },
            Vertex { pos: [ pos.0 + size.0, pos.1 + size.1], uv: [1.0, 0.0] },
        ],
        vec![
            0, 1, 2,
            2, 3, 0,
        ],
    )
}
