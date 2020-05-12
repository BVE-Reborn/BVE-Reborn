use crate::*;
use log::debug;
use std::mem::size_of;
use zerocopy::AsBytes;

#[repr(C)]
#[derive(AsBytes)]
struct Uniforms {
    _max_size: [u32; 2],
}

fn create_texture_compute_pipeline(
    device: &Device,
    shader_module: &ShaderModule,
) -> (ComputePipeline, BindGroupLayout) {
    debug!("Creating texture compute pipeline");
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
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStage::COMPUTE,
                ty: BindingType::UniformBuffer { dynamic: false },
            },
        ],
        label: Some("compute"),
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        bind_group_layouts: &[&bind_group_layout],
    });

    let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        layout: &pipeline_layout,
        compute_stage: ProgrammableStageDescriptor {
            module: shader_module,
            entry_point: "main",
        },
    });

    (pipeline, bind_group_layout)
}

fn create_uniform_buffer(device: &Device, encoder: &mut CommandEncoder, max_size: UVec2) -> Buffer {
    let uniforms = Uniforms {
        _max_size: max_size.into_array(),
    };
    let bytes = uniforms.as_bytes();
    let tmp_buffer = device.create_buffer_with_data(bytes, BufferUsage::COPY_SRC);
    let buffer = device.create_buffer(&BufferDescriptor {
        size: size_of::<Uniforms>() as u64,
        usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
        label: Some("Image Size"),
    });
    encoder.copy_buffer_to_buffer(&tmp_buffer, 0, &buffer, 0, size_of::<Uniforms>() as u64);

    buffer
}

fn create_texture_compute_bind_group(
    device: &Device,
    layout: &BindGroupLayout,
    source: &TextureView,
    dest: &TextureView,
    uniform_buffer: &Buffer,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        layout,
        bindings: &[
            Binding {
                binding: 0,
                resource: BindingResource::TextureView(source),
            },
            Binding {
                binding: 1,
                resource: BindingResource::TextureView(dest),
            },
            Binding {
                binding: 2,
                resource: BindingResource::Buffer {
                    buffer: uniform_buffer,
                    range: 0..(size_of::<Uniforms>() as u64),
                },
            },
        ],
        label: None,
    })
}

pub struct MipmapCompute {
    pipeline: ComputePipeline,
    bind_group_layout: BindGroupLayout,
}

impl MipmapCompute {
    pub fn new(device: &Device) -> Self {
        let (pipeline, bind_group_layout) = create_texture_compute_pipeline(device, &*shader!(device; mipmap - comp));

        Self {
            pipeline,
            bind_group_layout,
        }
    }

    pub fn compute_mipmaps(&self, device: &Device, encoder: &mut CommandEncoder, texture: &Texture, dimensions: UVec2) {
        for (level, dimensions) in render::enumerate_mip_levels(dimensions) {
            let parent = texture.create_view(&TextureViewDescriptor {
                dimension: TextureViewDimension::D2,
                format: TextureFormat::Rgba8Uint,
                aspect: TextureAspect::All,
                base_mip_level: level - 1,
                level_count: 1,
                base_array_layer: 0,
                array_layer_count: 1,
            });

            let child = texture.create_view(&TextureViewDescriptor {
                dimension: TextureViewDimension::D2,
                format: TextureFormat::Rgba8Uint,
                aspect: TextureAspect::All,
                base_mip_level: level,
                level_count: 1,
                base_array_layer: 0,
                array_layer_count: 1,
            });

            let uniform_buffer = create_uniform_buffer(device, encoder, dimensions);
            let bind_group =
                create_texture_compute_bind_group(device, &self.bind_group_layout, &parent, &child, &uniform_buffer);

            let mut cpass = encoder.begin_compute_pass();

            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch((dimensions.x + 7) / 8, (dimensions.y + 7) / 8, 1);

            drop(cpass);
        }
    }
}

pub struct CutoutTransparencyCompute {
    pipeline: ComputePipeline,
    bind_group_layout: BindGroupLayout,
}

impl CutoutTransparencyCompute {
    pub fn new(device: &Device) -> Self {
        let (pipeline, bind_group_layout) =
            create_texture_compute_pipeline(device, &*shader!(device; transparency - comp));

        Self {
            pipeline,
            bind_group_layout,
        }
    }

    pub fn compute_transparency(
        &self,
        device: &Device,
        encoder: &mut CommandEncoder,
        texture: &Texture,
        texture_dst: &Texture,
        dimensions: UVec2,
    ) {
        let view = TextureViewDescriptor {
            dimension: TextureViewDimension::D2,
            format: TextureFormat::Rgba8Uint,
            aspect: TextureAspect::All,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            array_layer_count: 1,
        };
        let source = texture.create_view(&view);
        let dest = texture_dst.create_view(&view);

        let uniform_buffer = create_uniform_buffer(device, encoder, dimensions);
        let bind_group =
            create_texture_compute_bind_group(device, &self.bind_group_layout, &source, &dest, &uniform_buffer);

        let mut cpass = encoder.begin_compute_pass();

        cpass.set_pipeline(&self.pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch((dimensions.x + 7) / 8, (dimensions.y + 7) / 8, 1);

        drop(cpass);
    }
}
