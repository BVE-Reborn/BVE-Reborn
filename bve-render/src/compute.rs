use cgmath::Vector2;
use wgpu::*;

pub struct MipmapCompute {
    pipeline: ComputePipeline,
    bind_group_layout: BindGroupLayout,
}

impl MipmapCompute {
    pub fn new(device: &Device) -> Self {
        let shader_source = include_shader!(comp "mipmap");
        let shader_module = device.create_shader_module(&read_spirv(std::io::Cursor::new(&shader_source[..])).unwrap());

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            bindings: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::StorageTexture {
                        dimension: TextureViewDimension::D2,
                        component_type: TextureComponentType::Uint,
                        format: TextureFormat::Rgba8Uint,
                        readonly: true,
                    },
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::StorageTexture {
                        dimension: TextureViewDimension::D2,
                        component_type: TextureComponentType::Uint,
                        format: TextureFormat::Rgba8Uint,
                        readonly: false,
                    },
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            layout: &pipeline_layout,
            compute_stage: ProgrammableStageDescriptor {
                module: &shader_module,
                entry_point: "main",
            },
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }

    pub fn compute_mipmaps(&self, device: &Device, texture: &Texture, dimensions: Vector2<u32>) -> CommandBuffer {
        let smallest = dimensions.x.min(dimensions.y) as f32;
        let mips = smallest.log2().floor();
        let child_size = (dimensions / 2);

        let parent = texture.create_view(&TextureViewDescriptor {
            dimension: TextureViewDimension::D2,
            format: TextureFormat::Rgba8Uint,
            aspect: TextureAspect::All,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            array_layer_count: 1,
        });

        let child = texture.create_view(&TextureViewDescriptor {
            dimension: TextureViewDimension::D2,
            format: TextureFormat::Rgba8Uint,
            aspect: TextureAspect::All,
            base_mip_level: 1,
            level_count: 1,
            base_array_layer: 0,
            array_layer_count: 1,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &self.bind_group_layout,
            bindings: &[
                Binding {
                    binding: 0,
                    resource: BindingResource::TextureView(&parent),
                },
                Binding {
                    binding: 1,
                    resource: BindingResource::TextureView(&child),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { todo: 0 });
        let mut cpass = encoder.begin_compute_pass();

        cpass.set_pipeline(&self.pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch(child_size.x, child_size.y, 0);

        drop(cpass);

        encoder.finish()
    }
}
