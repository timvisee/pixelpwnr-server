use gfx::handle::Buffer;
use gfx::traits::FactoryExt;
use gfx::{Resources, Slice};

use super::vertex::Vertex;

// A 3D model for use on the GPU defined as vertices and indices.
pub struct Model {
    /// List of vertices.
    vertices: Vec<Vertex>,

    /// List of indices.
    indices: Vec<u16>,
}

impl Model {
    /// Create a new model, from the given vertices and indices.
    pub fn from(vertices: Vec<Vertex>, indices: Vec<u16>) -> Self {
        Model { vertices, indices }
    }

    /// Create an immutable vertex buffer from the model vertices, together
    /// with a slice from the model indices.
    pub fn create_vertex_buffer<F, R>(&self, factory: &mut F) -> (Buffer<R, Vertex>, Slice<R>)
    where
        F: FactoryExt<R>,
        R: Resources,
    {
        factory.create_vertex_buffer_with_slice(&self.vertices, &*self.indices)
    }
}
