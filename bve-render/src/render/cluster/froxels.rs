use crate::{
    frustum::Frustum,
    render::cluster::{FrustumBytes, FROXELS_X, FROXELS_Y},
    *,
};
use bve::UVec2;
use wgpu::util::{BufferInitDescriptor, DeviceExt};

#[repr(C)]
#[derive(Clone, Copy)]
struct FroxelUniforms {
    _inv_proj: shader_types::Mat4,
    _frustum: FrustumBytes,
    _frustum_count: shader_types::UVec2,
}

unsafe impl bytemuck::Zeroable for FroxelUniforms {}
unsafe impl bytemuck::Pod for FroxelUniforms {}

pub struct FrustumCreation {
    uniform_buffer: Buffer,
    bind_group: BindGroup,
    pipeline: ComputePipeline,
}
impl FrustumCreation {
    pub fn new(
        device: &Device,
        _encoder: &mut CommandEncoder,
        frustum_buffer: &Buffer,
        mx_inv_proj: Mat4,
        frustum: Frustum,
        frustum_count: UVec2,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::UniformBuffer {
                        dynamic: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::StorageBuffer {
                        readonly: false,
                        dynamic: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("frustum creation bind group layout"),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("frustum creation pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = shader!(device; froxels - compute);

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("frustum creation pipeline"),
            layout: Some(&pipeline_layout),
            compute_stage: ProgrammableStageDescriptor {
                entry_point: "main",
                module: &*shader,
            },
        });

        let uniforms = FroxelUniforms {
            _frustum: frustum.into(),
            _frustum_count: shader_types::UVec2::from(frustum_count.into_array()),
            _inv_proj: shader_types::Mat4::from(*mx_inv_proj.as_ref()),
        };

        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("froxel uniform buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(uniform_buffer.slice(..)),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(frustum_buffer.slice(..)),
                },
            ],
            label: Some("frustum creation bind group"),
        });

        Self {
            uniform_buffer,
            bind_group,
            pipeline,
        }
    }

    pub fn resize(
        &mut self,
        device: &Device,
        encoder: &mut CommandEncoder,
        mx_inv_proj: Mat4,
        frustum: Frustum,
        frustum_count: UVec2,
    ) {
        let uniforms = FroxelUniforms {
            _frustum: frustum.into(),
            _frustum_count: shader_types::UVec2::from(frustum_count.into_array()),
            _inv_proj: shader_types::Mat4::from(*mx_inv_proj.as_ref()),
        };

        // TODO: use a belt
        let uniform_staging_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("froxel resize temp uniform buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_SRC,
        });

        encoder.copy_buffer_to_buffer(
            &uniform_staging_buffer,
            0,
            &self.uniform_buffer,
            0,
            size_of::<FroxelUniforms>() as BufferAddress,
        );
    }

    pub fn execute<'a>(&'a self, pass: &mut ComputePass<'a>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch(FROXELS_X / 8, FROXELS_Y / 8, 1);
    }
}
