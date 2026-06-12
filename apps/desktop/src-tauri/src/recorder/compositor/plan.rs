use std::collections::HashMap;

use super::cpu::compose_cpu_frame;
use crate::recorder::{
    capture_runtime::{
        CaptureRuntimeSource, CaptureRuntimeSourceHandle, CapturedFrame, CapturedFrameFormat,
    },
    errors::{capture_format_unsupported_error, capture_source_missing_error},
    types::{CanvasRect, CropRect, RecorderError, RecorderScene},
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SceneCompositorPlan {
    pub(crate) canvas_height: u32,
    pub(crate) canvas_width: u32,
    pub(crate) inputs: Vec<SceneCompositorInput>,
    pub(crate) output_format: CapturedFrameFormat,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SceneCompositorInput {
    pub(crate) crop: CropRect,
    pub(crate) rect: CanvasRect,
    pub(crate) runtime_handle: CaptureRuntimeSourceHandle,
    pub(crate) scene_source_id: String,
    pub(crate) source_id: String,
    pub(crate) z_index: i32,
}

impl SceneCompositorPlan {
    pub(crate) fn from_scene(
        scene: &RecorderScene,
        runtime_sources: &[CaptureRuntimeSource],
    ) -> Result<Self, RecorderError> {
        let source_handles = runtime_sources
            .iter()
            .map(|source| (source.source_id.as_str(), source.handle.clone()))
            .collect::<HashMap<_, _>>();
        let mut inputs = Vec::with_capacity(scene.sources.len());

        for source in &scene.sources {
            let runtime_handle = source_handles
                .get(source.source_id.as_str())
                .cloned()
                .ok_or_else(|| capture_source_missing_error(&source.source_id))?;
            inputs.push(SceneCompositorInput {
                crop: source.crop.clone(),
                rect: source.rect.clone(),
                runtime_handle,
                scene_source_id: source.id.clone(),
                source_id: source.source_id.clone(),
                z_index: source.z_index,
            });
        }

        inputs.sort_by_key(|input| input.z_index);

        Ok(Self {
            canvas_height: even_canvas_dimension(scene.canvas_height),
            canvas_width: even_canvas_dimension(scene.canvas_width),
            inputs,
            output_format: CapturedFrameFormat::Rgba,
        })
    }

    pub(crate) fn validate_composed_frame(
        &self,
        frame: &CapturedFrame,
    ) -> Result<(), RecorderError> {
        frame.validate()?;
        if frame.width != self.canvas_width || frame.height != self.canvas_height {
            return Err(capture_format_unsupported_error(format!(
                "Composed frame has dimensions {}x{}, expected {}x{}.",
                frame.width, frame.height, self.canvas_width, self.canvas_height
            )));
        }
        frame.writer_handoff_plan()?;

        Ok(())
    }

    pub(crate) fn compose_latest_frames(
        &self,
        frames_by_source_id: &HashMap<String, CapturedFrame>,
        timestamp_ms: u64,
    ) -> Result<CapturedFrame, RecorderError> {
        if let Some(frame) =
            self.compose_full_canvas_passthrough(frames_by_source_id, timestamp_ms)?
        {
            return Ok(frame);
        }

        compose_cpu_frame(self, frames_by_source_id, timestamp_ms)
    }

    fn compose_full_canvas_passthrough(
        &self,
        frames_by_source_id: &HashMap<String, CapturedFrame>,
        timestamp_ms: u64,
    ) -> Result<Option<CapturedFrame>, RecorderError> {
        let [input] = self.inputs.as_slice() else {
            return Ok(None);
        };
        let Some(frame) = frames_by_source_id.get(&input.source_id) else {
            return Ok(None);
        };
        if !input.is_full_canvas_passthrough(self.canvas_width, self.canvas_height, frame) {
            return Ok(None);
        }
        frame.writer_handoff_plan()?;

        Ok(Some(CapturedFrame::new(
            "composed",
            timestamp_ms,
            frame.width,
            frame.height,
            frame.stride,
            frame.format,
            frame.data.clone(),
        )?))
    }

    pub(crate) fn is_full_canvas_passthrough(
        &self,
        frames_by_source_id: &HashMap<String, CapturedFrame>,
    ) -> bool {
        let [input] = self.inputs.as_slice() else {
            return false;
        };
        frames_by_source_id
            .get(&input.source_id)
            .is_some_and(|frame| {
                input.is_full_canvas_passthrough(self.canvas_width, self.canvas_height, frame)
            })
    }
}

impl SceneCompositorInput {
    fn is_full_canvas_passthrough(
        &self,
        canvas_width: u32,
        canvas_height: u32,
        frame: &CapturedFrame,
    ) -> bool {
        frame.width == canvas_width
            && frame.height == canvas_height
            && near_zero(self.crop.left)
            && near_zero(self.crop.right)
            && near_zero(self.crop.top)
            && near_zero(self.crop.bottom)
            && near_zero(self.rect.x)
            && near_zero(self.rect.y)
            && near_equal(self.rect.width, canvas_width as f64)
            && near_equal(self.rect.height, canvas_height as f64)
    }
}

fn near_zero(value: f64) -> bool {
    value.abs() < 0.001
}

fn near_equal(left: f64, right: f64) -> bool {
    (left - right).abs() < 0.001
}

pub(crate) fn even_canvas_dimension(dimension: u32) -> u32 {
    dimension.saturating_add(dimension % 2)
}
