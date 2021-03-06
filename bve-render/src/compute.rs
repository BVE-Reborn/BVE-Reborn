use crate::*;
use log::debug;
use std::{
    mem::size_of,
    num::{NonZeroU32, NonZeroU64},
};
use wgpu::util::{BufferInitDescriptor, DeviceExt};

#[repr(C)]
#[derive(Copy, Clone)]
struct Uniforms {
    _max_size: shader_types::UVec2,
}

unsafe impl bytemuck::Zeroable for Uniforms {}
unsafe impl bytemuck::Pod for Uniforms {}

fn create_texture_compute_pipeline(
    device: &Device,
    shader_module: &ShaderModule,
) -> (ComputePipeline, BindGroupLayout) {
    debug!("Creating texture compute pipeline");
    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStage::COMPUTE,
                ty: BindingType::StorageTexture {
                    dimension: TextureViewDimension::D2,
                    format: TextureFormat::Rgba8Uint,
                    readonly: true,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStage::COMPUTE,
                ty: BindingType::StorageTexture {
                    dimension: TextureViewDimension::D2,
                    format: TextureFormat::Rgba8Uint,
                    readonly: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStage::COMPUTE,
                ty: BindingType::UniformBuffer {
                    dynamic: false,
                    min_binding_size: NonZeroU64::new(size_of::<Uniforms>() as _),
                },
                count: None,
            },
        ],
        label: Some("compute"),
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("texture compute layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: Some("texture compute pipeline"),
        layout: Some(&pipeline_layout),
        compute_stage: ProgrammableStageDescriptor {
            module: shader_module,
            entry_point: "main",
        },
    });

    (pipeline, bind_group_layout)
}

fn create_uniform_buffer(device: &Device, _encoder: &mut CommandEncoder, max_size: UVec2) -> Buffer {
    let uniforms = Uniforms {
        _max_size: shader_types::UVec2::from(max_size.into_array()),
    };
    device.create_buffer_init(&BufferInitDescriptor {
        label: Some("compute uniform buffer"),
        usage: BufferUsage::UNIFORM,
        contents: bytemuck::bytes_of(&uniforms),
    })
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
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(source),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::TextureView(dest),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::Buffer(uniform_buffer.slice(..)),
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
                dimension: Some(TextureViewDimension::D2),
                format: Some(TextureFormat::Rgba8Unorm),
                aspect: TextureAspect::All,
                base_mip_level: level - 1,
                level_count: NonZeroU32::new(1),
                base_array_layer: 0,
                array_layer_count: NonZeroU32::new(1),
                label: Some("mipmap creation parent"),
            });

            let child = texture.create_view(&TextureViewDescriptor {
                dimension: Some(TextureViewDimension::D2),
                format: Some(TextureFormat::Rgba8Unorm),
                aspect: TextureAspect::All,
                base_mip_level: level,
                level_count: NonZeroU32::new(1),
                base_array_layer: 0,
                array_layer_count: NonZeroU32::new(1),
                label: Some("mipmap creation child"),
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
        let mut view = TextureViewDescriptor {
            dimension: Some(TextureViewDimension::D2),
            format: Some(TextureFormat::Rgba8Unorm),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            level_count: NonZeroU32::new(1),
            base_array_layer: 0,
            array_layer_count: NonZeroU32::new(1),
            label: Some("transparency source"),
        };
        let source = texture.create_view(&view);
        view.label = Some("transparency dest");
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
