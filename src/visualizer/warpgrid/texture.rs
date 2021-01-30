use amethyst::renderer::{
    rendy::{
        command::RenderPassEncoder,
        factory::Factory,
        hal::{self, device::Device},
        resource::{DescriptorSet, DescriptorSetLayout, Escape, Handle},
    },
    types::{Backend, Texture},
    util,
};

pub struct Textures<B: Backend> {
    layout: Handle<DescriptorSetLayout<B>>,
    set: Escape<DescriptorSet<B>>,
}

impl<B: Backend> Textures<B> {
    pub fn new(
        factory: &Factory<B>,
        descriptor_set: u32,
        number: u32,
    ) -> Result<Self, hal::pso::CreationError> {
        use hal::pso::*;

        let layout: Handle<DescriptorSetLayout<B>> = factory
            .create_descriptor_set_layout(util::set_layout_bindings(vec![(
                number,
                DescriptorType::Image {
                    ty: ImageDescriptorType::Sampled { with_sampler: true },
                },
                ShaderStageFlags::FRAGMENT,
            )]))?
            .into();

        let set = factory.create_descriptor_set(layout.clone()).unwrap();

        Ok(Self { layout, set })
    }

    /// Returns the raw `DescriptorSetLayout` for Textures
    pub fn raw_layout(&self) -> &B::DescriptorSetLayout {
        self.layout.raw()
    }
}
