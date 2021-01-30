pub mod update {
    use amethyst::renderer::{
        rendy::{
            self,
            command::RenderPassEncoder,
            factory::Factory,
            hal::{format::Format, pso::CreationError},
            mesh::{AsVertex, VertexFormat},
            resource::{DescriptorSet, DescriptorSetLayout, Escape, Handle as RendyHandle},
            shader::{PathBufShaderInfo, ShaderKind, SourceLanguage, SpirvShader},
        },
        types::Backend,
        util,
    };
    use glsl_layout::*;
    use std::path::PathBuf;

    #[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Uniform)]
    #[repr(C, align(4))]
    pub struct VertexArgs {
        pub pos: vec2,
    }

    impl AsVertex for VertexArgs {
        fn vertex() -> VertexFormat {
            VertexFormat::new(((Format::Rg32Sfloat, "pos"),))
        }
    }

    #[derive(Clone, Copy, Debug, Uniform)]
    #[repr(C, align(4))]
    pub struct UniformData {
        pub value_scale: vec2,
        pub lightness_scale: vec2,
        pub alpha_scale: vec2,
        pub period: float,
        pub cycle: float,
        pub gamma: vec3,

        pub state_size: vec2,
        pub column_index: int,
    }

    impl UniformData {
        pub fn new(x: u32, y: u32) -> Self {
            Self {
                value_scale: [2., 0.].into(),
                lightness_scale: [0.6, 0.].into(),
                alpha_scale: [4.0, -4.0].into(),
                period: 0.3 * 120.,
                cycle: 0.1,
                gamma: [1.0, 1.5, 1.2].into(),

                state_size: [x as f32, y as f32].into(),
                column_index: 0,
            }
        }
    }

    // #[derive(Clone, Copy, Debug, Uniform)]
    // #[repr(C, align(4))]
    // pub struct UniformState {
    //     pub column_index: int,
    //     pub state_size: vec2,
    // }

    // impl UniformState {
    //     pub fn new(x: u32, y: u32) -> Self {
    //         Self {
    //             column_index: 0,
    //             state_size: [x as f32, y as f32].into(),
    //         }
    //     }
    // }

    lazy_static::lazy_static! {
        pub static ref VERTEX: SpirvShader = PathBufShaderInfo::new(
            PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src/visualizer/warpgrid/shaders/update.vert")),
            ShaderKind::Vertex,
            SourceLanguage::GLSL,
           "main",
        ).precompile().unwrap();

        pub static ref FRAGMENT: SpirvShader = PathBufShaderInfo::new(
            PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src/visualizer/warpgrid/shaders/update.frag")),
            ShaderKind::Fragment,
            SourceLanguage::GLSL,
            "main",
        ).precompile().unwrap();
    }
    /*
    #[derive(Debug)]
    pub struct UniformsDesc<B: Backend, T: Uniform>
    where
        T::Std140: Sized,
    {
        layout: RendyHandle<DescriptorSetLayout<B>>,
        set: Escape<DescriptorSet<B>>,
    }

    impl<B: Backend, T: Uniform> UniformsDesc<B, T>
    where
        T::Std140: Sized,
    {
        pub fn new(factory: &Factory<B>) -> Result<Self, CreationError> {
            use rendy::hal::pso::*;
            let layout: RendyHandle<DescriptorSetLayout<B>> = factory
                .create_descriptor_set_layout(util::set_layout_bindings(vec![
                    (
                        0,
                        DescriptorType::Buffer {
                            ty: BufferDescriptorType::Uniform,
                            format: BufferDescriptorFormat::Structured {
                                dynamic_offset: false,
                            },
                        },
                        ShaderStageFlags::FRAGMENT,
                    ),
                    (
                        1,
                        DescriptorType::Buffer {
                            ty: BufferDescriptorType::Uniform,
                            format: BufferDescriptorFormat::Structured {
                                dynamic_offset: false,
                            },
                        },
                        ShaderStageFlags::FRAGMENT,
                    ),
                ]))?
                .into();

                let buffer = factory
            .create_buffer(
                BufferInfo {
                    size: std::mem::size_of::<T::Std140>() as u64,
                    usage: hal::buffer::Usage::UNIFORM,
                },
                rendy::memory::Dynamic,
            )
            .unwrap();

        let set = factory.create_descriptor_set(layout.clone()).unwrap();
        let desc = hal::pso::Descriptor::Buffer(buffer.raw(), SubRange::WHOLE);
        unsafe {
            let set = set.raw();
            factory.write_descriptor_sets(Some(util::desc_write(set, 0, desc)));
        }

            let set = factory.create_descriptor_set(layout.clone())?;

            Ok(Self { layout, set })
        }

        pub fn raw_layout(&self) -> &B::DescriptorSetLayout {
            self.layout.raw()
        }

        pub fn write(&mut self, factory: &Factory<B>, item: T::Std140) {
            let mut mapped = this_image.map(factory);
            let mut writer = unsafe {
                mapped
                    .write::<u8>(factory.device(), 0..std::mem::size_of::<T::Std140>() as u64)
                    .unwrap()
            };
            let slice = unsafe { writer.slice() };

            slice.copy_from_slice(util::slice_as_bytes(&[item]));
        }

        #[inline]
        fn bind(
            &self,
            pipeline_layout: &B::PipelineLayout,
            set_id: u32,
            encoder: &mut RenderPassEncoder<'_, B>,
        ) {
            unsafe {
                encoder.bind_graphics_descriptor_sets(
                    pipeline_layout,
                    set_id,
                    Some(self.set.raw()),
                    std::iter::empty(),
                );
            }
        }
    }
    */
}
