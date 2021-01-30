use amethyst::{
    core::ecs::{DispatcherBuilder, World},
    error::Error,
    prelude::*,
    renderer::{
        bundle::{RenderOrder, RenderPlan, RenderPlugin, Target},
        pipeline::{PipelineDescBuilder, PipelinesBuilder},
        rendy::{
            command::{QueueId, RenderPassEncoder},
            factory::Factory,
            graph::{
                render::{PrepareResult, RenderGroup, RenderGroupDesc},
                GraphContext, NodeBuffer, NodeImage,
            },
            hal::{self, device::Device, format::Format, pso, pso::ShaderStageFlags},
            mesh::{AsVertex, VertexFormat},
            shader::{Shader, SpirvShader},
        },
        submodules::{DynamicUniform, DynamicVertexBuffer},
        system::GraphAuxData,
        types::Backend,
        util, ChangeDetection,
    },
};
use glsl_layout::Uniform;

use audio::frequency_sensor::Features as AudioFeatures;

mod shaders;
pub use shaders::update::UniformData;

mod texture;

/// Warpgrid visualizer
#[derive(Clone, Debug, PartialEq)]
pub struct WarpGridDesc;

impl WarpGridDesc {
    /// Create instance of WarpGrid renderer
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for WarpGridDesc {
    fn default() -> Self {
        Self {}
    }
}

impl<B: Backend> RenderGroupDesc<B, GraphAuxData> for WarpGridDesc {
    fn build(
        self,
        _ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        _queue: QueueId,
        _world: &GraphAuxData,
        framebuffer_width: u32,
        framebuffer_height: u32,
        subpass: hal::pass::Subpass<'_, B>,
        _buffers: Vec<NodeBuffer>,
        _images: Vec<NodeImage>,
    ) -> Result<Box<dyn RenderGroup<B, GraphAuxData>>, pso::CreationError> {
        let uniform_data = DynamicUniform::new(factory, pso::ShaderStageFlags::FRAGMENT)?;
        let uniform_index = DynamicUniform::new(factory, pso::ShaderStageFlags::FRAGMENT)?;

        // let uniforms = UniformsDesc::new(factory)?;
        let mut vertex = DynamicVertexBuffer::new();

        use shaders::update::VertexArgs;
        vertex.write(
            factory,
            0,
            4,
            Some(
                [
                    VertexArgs {
                        pos: [-1., -1.].into(),
                    },
                    VertexArgs {
                        pos: [1., -1.].into(),
                    },
                    VertexArgs {
                        pos: [-1., 1.].into(),
                    },
                    VertexArgs {
                        pos: [1., 1.].into(),
                    },
                ]
                .iter(),
            ),
        );

        let (pipeline, pipeline_layout) = build_pipeline(
            factory,
            subpass,
            framebuffer_width,
            framebuffer_height,
            vec![uniform_data.raw_layout(), uniform_index.raw_layout()],
        )?;

        Ok(Box::new(WarpGrid::<B> {
            pipeline,
            pipeline_layout,
            uniform_data,
            uniform_index,
            vertex,
            vertex_count: 4,
            change: Default::default(),
        }))
    }
}

#[derive(Debug)]
pub struct WarpGrid<B: Backend> {
    pipeline: B::GraphicsPipeline,
    pipeline_layout: B::PipelineLayout,
    uniform_data: DynamicUniform<B, UniformData>,
    uniform_index: DynamicUniform<B, i32>,
    vertex: DynamicVertexBuffer<B, shaders::update::VertexArgs>,
    vertex_count: usize,
    change: ChangeDetection,
}

impl<B: Backend> RenderGroup<B, GraphAuxData> for WarpGrid<B> {
    fn prepare(
        &mut self,
        factory: &Factory<B>,
        _queue: QueueId,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        aux: &GraphAuxData,
    ) -> PrepareResult {
        let params = aux.resources.get::<UniformData>().unwrap();
        // let audio_features = aux.resources.get::<AudioFeatures>().unwrap();

        self.uniform_data.write(factory, index, params.std140());
        // self.uniform_index
        //     .write(factory, index, audio_features.get_index());

        self.change.prepare_result(index, false)
    }

    fn draw_inline(
        &mut self,
        mut encoder: RenderPassEncoder<'_, B>,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        _aux: &GraphAuxData,
    ) {
        encoder.bind_graphics_pipeline(&self.pipeline);
        self.uniform_data
            .bind(index, &self.pipeline_layout, 1, &mut encoder);
        unsafe {
            encoder.draw(0..self.vertex_count as u32, 0..1);
        }
    }

    fn dispose(self: Box<Self>, factory: &mut Factory<B>, _aux: &GraphAuxData) {
        unsafe {
            factory.device().destroy_graphics_pipeline(self.pipeline);
            factory
                .device()
                .destroy_pipeline_layout(self.pipeline_layout);
        }
    }
}

fn build_pipeline<B: Backend>(
    factory: &Factory<B>,
    subpass: hal::pass::Subpass<'_, B>,
    framebuffer_width: u32,
    framebuffer_height: u32,
    layouts: Vec<&B::DescriptorSetLayout>,
) -> Result<(B::GraphicsPipeline, B::PipelineLayout), pso::CreationError> {
    println!("New pipeline with layout {:#?}", layouts);

    let pipeline_layout = unsafe {
        factory
            .device()
            .create_pipeline_layout(layouts, None as Option<(_, _)>)
    }?;

    use shaders::update::{VertexArgs, FRAGMENT, VERTEX};

    // Load the shaders
    let shader_vertex = unsafe { VERTEX.module(factory).unwrap() };
    let shader_fragment = unsafe { FRAGMENT.module(factory).unwrap() };

    // Build the pipeline
    let pipes = PipelinesBuilder::new()
        .with_pipeline(
            PipelineDescBuilder::new()
                // This Pipeline uses our custom vertex description and does not use instancing
                .with_vertex_desc(&[(VertexArgs::vertex(), pso::VertexInputRate::Vertex)])
                .with_input_assembler(pso::InputAssemblerDesc::new(pso::Primitive::TriangleStrip))
                // Add the shaders
                .with_shaders(util::simple_shader_set(
                    &shader_vertex,
                    Some(&shader_fragment),
                ))
                .with_layout(&pipeline_layout)
                .with_subpass(subpass)
                .with_framebuffer_size(framebuffer_width, framebuffer_height)
                // We are using alpha blending
                .with_blend_targets(vec![pso::ColorBlendDesc {
                    mask: pso::ColorMask::ALL,
                    blend: Some(pso::BlendState::ALPHA),
                }]),
        )
        .build(factory, None);

    // Destoy the shaders once loaded
    unsafe {
        factory.destroy_shader_module(shader_vertex);
        factory.destroy_shader_module(shader_fragment);
    }

    // Handle the Errors
    match pipes {
        Err(e) => {
            unsafe {
                factory.device().destroy_pipeline_layout(pipeline_layout);
            }
            Err(e)
        }
        Ok(mut pipes) => Ok((pipes.remove(0), pipeline_layout)),
    }
}

#[derive(Debug)]
pub struct WarpGridRender {}

impl Default for WarpGridRender {
    fn default() -> Self {
        Self {}
    }
}

impl<B: Backend> RenderPlugin<B> for WarpGridRender {
    fn on_build<'a, 'b>(
        &mut self,
        _world: &mut World,
        resources: &mut Resources,
        _builder: &mut DispatcherBuilder,
    ) -> Result<(), Error> {
        resources.insert(UniformData::new(16, 120));
        Ok(())
    }

    fn on_plan(
        &mut self,
        plan: &mut RenderPlan<B>,
        _factory: &mut Factory<B>,
        _world: &World,
        _resources: &Resources,
    ) -> Result<(), Error> {
        plan.extend_target(Target::Main, |ctx| {
            ctx.add(RenderOrder::Transparent, WarpGridDesc::new().builder())?;
            Ok(())
        });
        Ok(())
    }
}
