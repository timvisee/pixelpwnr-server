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
        Model {
            vertices,
            indices,
        }
    }

    /// Get a reference to the vertices in this model.
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// Get a reference to the indices in this model.
    pub fn indices(&self) -> &[u16] {
        &self.indices
    }
}
