// +x right
// +y up
// +z away from camera

// Rust warnings
#![warn(unused)]
#![deny(future_incompatible)]
#![deny(nonstandard_style)]
#![deny(rust_2018_idioms)]
// Rustdoc Warnings
#![deny(intra_doc_link_resolution_failure)]
// Clippy warnings
#![warn(clippy::cargo)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::restriction)]
// Annoying regular clippy warnings
#![allow(clippy::cast_lossless)] // Annoying
#![allow(clippy::cast_sign_loss)] // Annoying
#![allow(clippy::cast_precision_loss)] // Annoying
#![allow(clippy::cast_possible_truncation)] // Annoying
#![allow(clippy::cognitive_complexity)] // This is dumb
#![allow(clippy::too_many_lines)] // This is also dumb
// Annoying/irrelevant clippy Restrictions
#![allow(clippy::as_conversions)]
#![allow(clippy::decimal_literal_representation)]
#![allow(clippy::default_trait_access)]
#![allow(clippy::else_if_without_else)]
#![allow(clippy::expect_used)]
#![allow(clippy::fallible_impl_from)] // This fails horribly when you try to panic in a macro inside a From impl
#![allow(clippy::float_arithmetic)]
#![allow(clippy::float_cmp)]
#![allow(clippy::float_cmp_const)]
#![allow(clippy::future_not_send)]
#![allow(clippy::if_not_else)]
#![allow(clippy::implicit_return)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::integer_arithmetic)]
#![allow(clippy::integer_division)]
#![allow(clippy::let_underscore_must_use)]
#![allow(clippy::match_bool)] // prettier
#![allow(clippy::missing_docs_in_private_items)]
#![allow(clippy::missing_inline_in_public_items)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::multiple_crate_versions)] // Cargo deny's job
#![allow(clippy::multiple_inherent_impl)]
#![allow(clippy::non_ascii_literal)]
#![allow(clippy::panic)]
#![allow(clippy::similar_names)]
#![allow(clippy::shadow_reuse)]
#![allow(clippy::shadow_same)]
#![allow(clippy::string_add)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnested_or_patterns)] // CLion no loves me
#![allow(clippy::unreachable)]
#![allow(clippy::wildcard_enum_match_arm)]
#![allow(clippy::wildcard_imports)]

pub use crate::{
    lights::LightHandle,
    mesh::MeshHandle,
    object::ObjectHandle,
    render::{DebugMode, MSAASetting, Vsync},
    statistics::RendererStatistics,
    texture::TextureHandle,
};
use crate::{
    object::perspective_matrix,
    render::{create_texture_bind_group, create_texture_bind_group_layout, UniformVerts},
};
use bve::{load::mesh::Vertex as MeshVertex, runtime::RenderLightDescriptor, UVec2};
use bve_conveyor::{AutomatedBuffer, AutomatedBufferManager, UploadStyle};
use glam::{Mat4, Vec3A};
use image::RgbaImage;
use itertools::Itertools;
use log::{debug, error, info};
use num_traits::{ToPrimitive, Zero};
use slotmap::{DefaultKey, SlotMap};
use std::{mem::size_of, num::NonZeroU8, sync::Arc, time::Instant};
use wgpu::*;
use winit::{dpi::PhysicalSize, window::Window};

#[cfg(feature = "renderdoc")]
macro_rules! renderdoc {
    ($($tokens:tt)*) => {
        $($tokens)*
    };
}

#[cfg(not(feature = "renderdoc"))]
macro_rules! renderdoc {
    ($($tokens:tt)*) => {};
}

mod camera;
mod compute;
mod frustum;
mod lights;
mod mesh;
mod object;
mod render;
mod shader;
mod statistics;
mod texture;

fn create_timestamp(duration: &mut f32, prev: Instant) -> Instant {
    let now = Instant::now();
    *duration = (now - prev).as_secs_f32() * 1000.0;
    now
}

pub struct Renderer {
    objects: SlotMap<DefaultKey, object::Object>,
    mesh: SlotMap<DefaultKey, mesh::Mesh>,
    textures: SlotMap<DefaultKey, texture::Texture>,
    null_texture: DefaultKey,
    lights: SlotMap<DefaultKey, RenderLightDescriptor>,

    camera: camera::Camera,
    resolution: PhysicalSize<u32>,
    samples: MSAASetting,
    vsync: Vsync,
    debug_mode: DebugMode,

    projection_matrix: Mat4,

    surface: Surface,
    device: Device,
    queue: Queue,
    swapchain: SwapChain,
    framebuffer: TextureView,
    unsampled_framebuffer: Option<TextureView>,
    depth_buffer: TextureView,
    opaque_pipeline: RenderPipeline,
    transparent_pipeline: RenderPipeline,
    pipeline_layout: PipelineLayout,
    texture_bind_group_layout: BindGroupLayout,
    sampler: Sampler,
    nearest_sampler: Sampler,

    buffer_manager: AutomatedBufferManager,
    matrix_buffer: AutomatedBuffer,

    vert_shader: Arc<ShaderModule>,
    frag_shader: Arc<ShaderModule>,
    transparent_shader: Arc<ShaderModule>,

    transparency_processor: compute::CutoutTransparencyCompute,
    mip_creator: compute::MipmapCompute,
    framebuffer_blitter: render::blit::FramebufferBlitter,
    cluster_renderer: render::cluster::Clustering,
    skybox_renderer: render::skybox::Skybox,
    imgui_renderer: bve_imgui_wgpu::Renderer,

    command_buffers: Vec<CommandBuffer>,
    _renderdoc_capture: bool,
}

impl Renderer {
    pub async fn new(
        window: &Window,
        imgui_context: &mut imgui::Context,
        samples: render::MSAASetting,
        vsync: render::Vsync,
    ) -> Self {
        let screen_size = window.inner_size();

        info!(
            "Creating renderer with: screen size = {}x{}, samples = {}, vsync = {}",
            screen_size.width, screen_size.height, samples as u8, vsync
        );

        let instance = Instance::new(BackendBit::VULKAN | BackendBit::METAL);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Could not create Adapter");

        let adapter_info = adapter.get_info();
        let adapter_features = adapter.features();
        let needed_features = Features::MAPPABLE_PRIMARY_BUFFERS
            | Features::SAMPLED_TEXTURE_BINDING_ARRAY
            | Features::SAMPLED_TEXTURE_ARRAY_NON_UNIFORM_INDEXING
            | Features::MULTI_DRAW_INDIRECT
            | Features::MULTI_DRAW_INDIRECT_COUNT;

        assert!(
            adapter_features.contains(needed_features),
            "Adapter must support all features needed. Missing features: {:?}",
            adapter_features - needed_features
        );

        let (device, mut queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    features: needed_features,
                    limits: Limits::default(),
                    shader_validation: false,
                },
                None,
            )
            .await
            .expect("Could not create device");

        let mut startup_encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: Some("startup") });

        let swapchain_desc = render::create_swapchain_descriptor(screen_size, vsync);
        let swapchain = device.create_swap_chain(&surface, &swapchain_desc);

        let vs_module = shader!(&device; opaque - vert);
        let fs_module = shader!(&device; opaque - frag);
        let transparent_fs_module = shader!(&device; transparent - frag);

        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare: None,
            anisotropy_clamp: NonZeroU8::new(16),
            label: Some("primary texture sampler"),
        });
        let nearest_sampler = device.create_sampler(&SamplerDescriptor::default());

        let (framebuffer, unsampled_framebuffer) = render::create_framebuffers(&device, screen_size, samples);
        let depth_buffer = render::create_depth_buffer(&device, screen_size, samples);

        let projection_matrix = perspective_matrix(
            45_f32.to_radians(),
            screen_size.width as f32 / screen_size.height as f32,
        );

        let mut buffer_manager = AutomatedBufferManager::new(UploadStyle::from_device_type(&adapter_info.device_type));

        let cluster_renderer = render::cluster::Clustering::new(
            &device,
            &mut buffer_manager,
            &mut startup_encoder,
            projection_matrix.inverse(),
            frustum::Frustum::from_matrix(projection_matrix),
        );

        let texture_bind_group_layout = create_texture_bind_group_layout(&device, TextureComponentType::Uint);
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("standard layout"),
            bind_group_layouts: &[&texture_bind_group_layout, cluster_renderer.bind_group_layout()],
            push_constant_ranges: &[],
        });

        let opaque_pipeline =
            render::create_pipeline(&device, &pipeline_layout, &vs_module, &fs_module, samples, false);
        let transparent_pipeline = render::create_pipeline(
            &device,
            &pipeline_layout,
            &vs_module,
            &transparent_fs_module,
            samples,
            true,
        );

        let matrix_buffer = buffer_manager.create_new_buffer(&device, 0, BufferUsage::VERTEX, Some("uniform buffer"));

        let transparency_processor = compute::CutoutTransparencyCompute::new(&device);
        let mip_creator = compute::MipmapCompute::new(&device);
        let framebuffer_blitter = render::blit::FramebufferBlitter::new(
            &device,
            unsampled_framebuffer.as_ref().unwrap_or(&framebuffer),
            &nearest_sampler,
        );
        let skybox_renderer =
            render::skybox::Skybox::new(&mut buffer_manager, &device, &texture_bind_group_layout, samples);
        let imgui_renderer = bve_imgui_wgpu::Renderer::new(
            imgui_context,
            &device,
            &mut queue,
            &mut buffer_manager,
            swapchain_desc.format,
            None,
        );

        // Create the Renderer object early so we can can call methods on it.
        let mut renderer = Self {
            objects: SlotMap::new(),
            mesh: SlotMap::new(),
            textures: SlotMap::new(),
            null_texture: DefaultKey::default(),
            lights: SlotMap::new(),

            camera: camera::Camera {
                location: Vec3A::new(-6.0, 0.0, 0.0),
                pitch: 0.0,
                yaw: 0.0,
            },
            resolution: screen_size,
            samples,
            projection_matrix,
            debug_mode: DebugMode::None,
            vsync,

            surface,
            device,
            queue,
            swapchain,
            framebuffer,
            unsampled_framebuffer,
            depth_buffer,
            opaque_pipeline,
            transparent_pipeline,
            pipeline_layout,
            texture_bind_group_layout,
            sampler,
            nearest_sampler,

            buffer_manager,
            matrix_buffer,

            vert_shader: vs_module,
            frag_shader: fs_module,
            transparent_shader: transparent_fs_module,

            transparency_processor,
            mip_creator,
            framebuffer_blitter,
            cluster_renderer,
            skybox_renderer,
            imgui_renderer,

            command_buffers: vec![startup_encoder.finish()],
            _renderdoc_capture: false,
        };

        // Default texture is texture handle zero, immediately discard the handle, never to be seen again
        renderer.null_texture = renderer
            .add_texture(&RgbaImage::from_raw(1, 1, vec![0xff, 0xff, 0xff, 0xff]).expect("Invalid Image"))
            .0;

        renderer
    }

    pub fn resize(&mut self, screen_size: PhysicalSize<u32>) {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: Some("resizer") });
        debug!("Resizing to {}x{}", screen_size.width, screen_size.height);
        let (framebuffer, unsampled_framebuffer) = render::create_framebuffers(&self.device, screen_size, self.samples);
        self.framebuffer = framebuffer;
        self.unsampled_framebuffer = unsampled_framebuffer;
        self.depth_buffer = render::create_depth_buffer(&self.device, screen_size, self.samples);
        self.resolution = screen_size;

        self.swapchain = self.device.create_swap_chain(
            &self.surface,
            &render::create_swapchain_descriptor(screen_size, self.vsync),
        );
        self.projection_matrix = perspective_matrix(
            45_f32.to_radians(),
            screen_size.width as f32 / screen_size.height as f32,
        );

        self.framebuffer_blitter.resize(
            &self.device,
            self.unsampled_framebuffer.as_ref().unwrap_or(&self.framebuffer),
            &self.nearest_sampler,
        );
        self.cluster_renderer.resize(
            &self.device,
            &mut encoder,
            self.projection_matrix.inverse(),
            frustum::Frustum::from_matrix(self.projection_matrix),
        );
        self.command_buffers.push(encoder.finish());
    }

    pub fn set_debug(&mut self, mode: DebugMode) {
        match mode {
            DebugMode::None => {
                self.frag_shader = shader!(&self.device; opaque - fragment);
            }
            DebugMode::Normals => {
                self.frag_shader = shader!(&self.device; debug_normals - fragment);
            }
            DebugMode::Frustums => {
                self.frag_shader = shader!(&self.device; debug_frustums - fragment);
            }
            DebugMode::FrustumAddressing => {
                self.frag_shader = shader!(&self.device; debug_frustum_addressing - fragment);
            }
            DebugMode::LightCount => {
                self.frag_shader = shader!(&self.device; debug_light_count - fragment);
            }
        };
        self.debug_mode = mode;
        self.opaque_pipeline = render::create_pipeline(
            &self.device,
            &self.pipeline_layout,
            &self.vert_shader,
            &self.frag_shader,
            self.samples,
            false,
        );
        self.transparent_pipeline = render::create_pipeline(
            &self.device,
            &self.pipeline_layout,
            &self.vert_shader,
            &self.transparent_shader,
            self.samples,
            true,
        );
    }

    pub fn set_samples(&mut self, samples: render::MSAASetting) {
        debug!("Setting sample count to {}", samples as u8);
        let (framebuffer, unsampled_framebuffer) = render::create_framebuffers(&self.device, self.resolution, samples);
        self.framebuffer = framebuffer;
        self.unsampled_framebuffer = unsampled_framebuffer;
        self.depth_buffer = render::create_depth_buffer(&self.device, self.resolution, samples);
        self.opaque_pipeline = render::create_pipeline(
            &self.device,
            &self.pipeline_layout,
            &self.vert_shader,
            &self.frag_shader,
            samples,
            false,
        );
        self.transparent_pipeline = render::create_pipeline(
            &self.device,
            &self.pipeline_layout,
            &self.vert_shader,
            &self.transparent_shader,
            samples,
            true,
        );
        self.samples = samples;

        self.framebuffer_blitter.set_samples(
            &self.device,
            self.unsampled_framebuffer.as_ref().unwrap_or(&self.framebuffer),
            &self.nearest_sampler,
        );
        self.skybox_renderer.set_samples(&self.device, samples);
    }

    pub fn set_vsync(&mut self, vsync: Vsync) {
        debug!("Setting vsync to {}", vsync);
        self.swapchain = self.device.create_swap_chain(
            &self.surface,
            &render::create_swapchain_descriptor(self.resolution, vsync),
        );
        self.vsync = vsync;
    }
}
