use std::{collections::BTreeMap, iter::FromIterator, sync::Arc};

use bevy::prelude::*;
use vulkano::{
    descriptor_set::layout::{
        DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo,
        DescriptorType,
    },
    device::Queue,
    pipeline::{layout::PipelineLayoutCreateInfo, ComputePipeline, PipelineLayout},
    shader::{EntryPoint, ShaderStages, SpecializationConstants},
};

use crate::{CANVAS_SIZE_X, CANVAS_SIZE_Y};

/// Descriptor set layout binding information for storage buffer
pub fn storage_buffer_desc() -> DescriptorSetLayoutBinding {
    DescriptorSetLayoutBinding {
        stages: ShaderStages::COMPUTE,
        ..DescriptorSetLayoutBinding::descriptor_type(DescriptorType::StorageBuffer)
    }
}

/// Descriptor set layout binding information for image buffer
pub fn storage_image_desc() -> DescriptorSetLayoutBinding {
    DescriptorSetLayoutBinding {
        stages: ShaderStages::COMPUTE,
        ..DescriptorSetLayoutBinding::descriptor_type(DescriptorType::StorageImage)
    }
}

/// Creates a compute pipeline from given shader, with given descriptor layout binding.
/// The intention here is that the descriptor layout should correspond the shader's layout.
/// Normally you would use `ComputePipeline::new`, which would generate layout for descriptor set
/// automatically. However, because I've split the shaders to various different shaders, each shader
/// that does not use a variable from my shared layout don't get the bindings for that specific
/// variable. See https://github.com/vulkano-rs/vulkano/pull/1778 and https://github.com/vulkano-rs/vulkano/issues/1556#issuecomment-821658405.
pub fn create_compute_pipeline<Css>(
    compute_queue: Arc<Queue>,
    shader_entry_point: EntryPoint,
    descriptor_layout: Vec<(u32, DescriptorSetLayoutBinding)>,
    specialization_constants: &Css,
) -> Arc<ComputePipeline>
where
    Css: SpecializationConstants,
{
    let push_constant_reqs = shader_entry_point
        .push_constant_requirements()
        .cloned()
        .into_iter()
        .collect();
    let set_layout = DescriptorSetLayout::new(
        compute_queue.device().clone(),
        DescriptorSetLayoutCreateInfo {
            bindings: BTreeMap::from_iter(descriptor_layout),
            ..Default::default()
        },
    )
    .unwrap();
    let pipeline_layout =
        PipelineLayout::new(compute_queue.device().clone(), PipelineLayoutCreateInfo {
            set_layouts: vec![set_layout],
            push_constant_ranges: push_constant_reqs,
            ..Default::default()
        })
        .unwrap();
    ComputePipeline::with_pipeline_layout(
        compute_queue.device().clone(),
        shader_entry_point,
        specialization_constants,
        pipeline_layout,
        None,
    )
    .unwrap()
}

/// Converts cursor position to world coordinates
pub fn cursor_to_world(window: &Window, camera_pos: Vec2, camera_scale: f32) -> Vec2 {
    (window.cursor_position().unwrap() - Vec2::new(window.width() / 2.0, window.height() / 2.0))
        * camera_scale
        - camera_pos
}

/// Mouse world position
#[derive(Debug, Copy, Clone)]
pub struct MousePos {
    pub world: Vec2,
}

impl MousePos {
    pub fn new(pos: Vec2) -> MousePos {
        MousePos {
            world: pos,
        }
    }

    /// Converts world position to canvas position:
    /// Inverts y and adds half canvas to the position (pixel units)
    pub fn canvas_pos(&self) -> Vec2 {
        self.world + Vec2::new(CANVAS_SIZE_X as f32 / 2.0, CANVAS_SIZE_Y as f32 / 2.0)
    }
}

/// Gets a line of canvas coordinates between previous and current mouse position
pub fn get_canvas_line(prev: Option<MousePos>, current: MousePos) -> Vec<IVec2> {
    let canvas_pos = current.canvas_pos();
    let prev = if let Some(prev) = prev {
        prev.canvas_pos()
    } else {
        canvas_pos
    };
    line_drawing::Bresenham::new(
        (prev.x.round() as i32, prev.y.round() as i32),
        (canvas_pos.x.round() as i32, canvas_pos.y.round() as i32),
    )
    .map(|pos| IVec2::new(pos.0, pos.1))
    .collect::<Vec<IVec2>>()
}
