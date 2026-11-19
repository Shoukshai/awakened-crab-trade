//! Independent CPU inference when the native ONNX Runtime fails its known input.
//! Bounded exact-width plans retain the model's recognition behavior.
use std::{io::Cursor, sync::Arc, time::Instant};
use tract_onnx::prelude::*;

pub struct Backend {
    proto: tract_onnx::pb::ModelProto,
    plans: Vec<(usize, Arc<TypedRunnableModel<TypedModel>>)>,
}
impl Backend {
    pub fn load(data: &[u8]) -> Result<(Self, Vec<String>), String> {
        let loader = tract_onnx::onnx();
        let proto = loader
            .proto_model_for_read(&mut Cursor::new(data))
            .map_err(|e| e.to_string())?;
        let chars = &proto
            .metadata_props
            .iter()
            .find(|p| p.key == "character")
            .ok_or("Rust OCR dictionary is missing")?
            .value;
        let mut dictionary = vec![String::new()];
        dictionary.extend(chars.lines().map(str::to_owned));
        dictionary.push(" ".into());
        Ok((
            Self {
                proto,
                plans: Vec::new(),
            },
            dictionary,
        ))
    }

    pub fn run(&mut self, width: usize, data: Vec<f32>) -> Result<(Vec<i64>, Vec<f32>), String> {
        if width == 0 || width > super::MAX_WIDTH || data.len() != 3 * 48 * width {
            return Err("Invalid Rust OCR line tensor".into());
        }
        if let Some(index) = self.plans.iter().position(|(w, _)| *w == width) {
            let cached = self.plans.remove(index);
            self.plans.push(cached);
        } else {
            let started = Instant::now();
            let mut loader = tract_onnx::onnx();
            loader.use_output_shapes = false;
            let mut proto = self.proto.clone();
            let graph = proto.graph.as_mut().ok_or("Rust OCR graph is missing")?;
            graph.value_info.clear();
            let input = graph.input.first_mut().ok_or("Rust OCR input is missing")?;
            let ty = input
                .r#type
                .as_mut()
                .and_then(|v| v.value.as_mut())
                .ok_or("Rust OCR input type is missing")?;
            let tract_onnx::pb::type_proto::Value::TensorType(ty) = ty;
            let shape = ty.shape.as_mut().ok_or("Rust OCR input shape is missing")?;
            if shape.dim.len() != 4 {
                return Err("Unexpected Rust OCR input rank".into());
            }
            for (dim, value) in shape.dim.iter_mut().zip([1, 3, 48, width as i64]) {
                dim.value =
                    Some(tract_onnx::pb::tensor_shape_proto::dimension::Value::DimValue(value));
            }
            let plan = loader
                .model_for_proto_model(&proto)
                .and_then(|m| m.into_optimized())
                .and_then(|m| {
                    m.into_runnable_with_options(&PlanOptions {
                        executor: Some(multithread::Executor::SingleThread),
                        ..Default::default()
                    })
                })
                .map_err(|e| e.to_string())?;
            if self.plans.len() == 8 {
                self.plans.remove(0);
            }
            self.plans.push((width, Arc::new(plan)));
            eprintln!(
                "ocr rust plan width={width} cached={} compile_ms={}",
                self.plans.len(),
                started.elapsed().as_millis()
            );
        }
        // Padding changes this recognizer's predictions even on unpadded text.
        // Use exactly the same preprocessed tensor as the native runtime.
        let input = Tensor::from_shape(&[1, 3, 48, width], &data).map_err(|e| e.to_string())?;
        let plan = &self.plans.last().ok_or("Rust OCR plan is missing")?.1;
        let result = plan.run(tvec!(input.into())).map_err(|e| e.to_string())?;
        let tensor = result.first().ok_or("Rust OCR output is missing")?;
        let array = tensor.to_array_view::<f32>().map_err(|e| e.to_string())?;
        let scores = array
            .as_slice()
            .ok_or("Rust OCR output is not contiguous")?
            .to_vec();
        Ok((tensor.shape().iter().map(|&v| v as i64).collect(), scores))
    }
}
