use std::sync::Arc;

use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    memory::allocator::{AllocationCreateInfo, MemoryUsage, StandardMemoryAllocator},
    pipeline::graphics::vertex_input::Vertex,
};

/// Vertex for textured quads.
#[repr(C)]
#[derive(Default, Debug, Copy, Clone, BufferContents, Vertex)]
pub struct TexturedVertex {
    #[format(R32G32B32A32_SFLOAT)]
    pub color: [f32; 4],
    #[format(R32G32_SFLOAT)]
    pub position: [f32; 2],
    #[format(R32G32_SFLOAT)]
    pub tex_coords: [f32; 2],
}

/// Textured quad with vertices & indices
#[derive(Default, Debug, Copy, Clone)]
pub struct TexturedQuad {
    pub vertices: [TexturedVertex; 4],
    pub indices: [u32; 6],
}

/// A set of vertices and their indices as cpu accessible buffers
#[derive(Clone)]
pub struct Mesh {
    pub vertices: Subbuffer<[TexturedVertex]>,
    pub indices: Subbuffer<[u32]>,
}

impl TexturedQuad {
    /// Creates a new textured quad with given width and height at (0.0, 0.0)
    pub fn new(width: f32, height: f32, color: [f32; 4]) -> TexturedQuad {
        TexturedQuad {
            vertices: [
                TexturedVertex {
                    position: [-(width / 2.0), -(height / 2.0)],
                    tex_coords: [0.0, 1.0],
                    color,
                },
                TexturedVertex {
                    position: [-(width / 2.0), height / 2.0],
                    tex_coords: [0.0, 0.0],
                    color,
                },
                TexturedVertex {
                    position: [width / 2.0, height / 2.0],
                    tex_coords: [1.0, 0.0],
                    color,
                },
                TexturedVertex {
                    position: [width / 2.0, -(height / 2.0)],
                    tex_coords: [1.0, 1.0],
                    color,
                },
            ],
            indices: [0, 2, 1, 0, 3, 2],
        }
    }

    /// Converts Quad data to a mesh that can be used in drawing
    pub fn to_mesh(self, allocator: &Arc<StandardMemoryAllocator>) -> Mesh {
        Mesh {
            vertices: Buffer::from_iter(
                allocator,
                BufferCreateInfo {
                    usage: BufferUsage::VERTEX_BUFFER,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    usage: MemoryUsage::Upload,
                    ..Default::default()
                },
                self.vertices.into_iter(),
            )
            .unwrap(),
            indices: Buffer::from_iter(
                allocator,
                BufferCreateInfo {
                    usage: BufferUsage::INDEX_BUFFER,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    usage: MemoryUsage::Upload,
                    ..Default::default()
                },
                self.indices.into_iter(),
            )
            .unwrap(),
        }
    }
}
