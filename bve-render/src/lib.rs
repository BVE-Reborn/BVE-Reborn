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
#![allow(clippy::else_if_without_else)]
#![allow(clippy::fallible_impl_from)] // This fails horribly when you try to panic in a macro inside a From impl
#![allow(clippy::float_arithmetic)]
#![allow(clippy::float_cmp)]
#![allow(clippy::float_cmp_const)]
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
#![allow(clippy::option_expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::result_expect_used)]
#![allow(clippy::similar_names)]
#![allow(clippy::shadow_reuse)]
#![allow(clippy::shadow_same)]
#![allow(clippy::string_add)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unreachable)]
#![allow(clippy::wildcard_enum_match_arm)]
#![allow(clippy::wildcard_imports)]

pub use crate::{
    lights::{ConeLight, LightDescriptor, LightHandle, PointLight},
    mesh::MeshHandle,
    object::ObjectHandle,
    render::{oit::OITNodeCount, DebugMode, MSAASetting, Vsync},
    statistics::RendererStatistics,
    texture::TextureHandle,
};
use crate::{object::perspective_matrix, render::Uniforms};
use bve::load::mesh::Vertex as MeshVertex;
use image::RgbaImage;
use indexmap::map::IndexMap;
use itertools::Itertools;
use log::{debug, error, info};
use nalgebra_glm::{inverse, make_vec2, make_vec3, Mat4, Vec3};
use num_traits::{ToPrimitive, Zero};
use std::{mem::size_of, sync::Arc, time::Instant};
use wgpu::*;
use winit::{dpi::PhysicalSize, window::Window};
use zerocopy::{AsBytes, FromBytes};

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
mod screenspace;
mod shader;
mod statistics;
mod texture;

fn create_timestamp(duration: &mut f32, prev: Instant) -> Instant {
    let now = Instant::now();
    *duration = (now - prev).as_secs_f32() * 1000.0;
    now
}

pub struct Renderer {
    objects: IndexMap<u64, object::Object>,
    object_handle_count: u64,

    mesh: IndexMap<u64, mesh::Mesh>,
    mesh_handle_count: u64,

    textures: IndexMap<u64, texture::Texture>,
    texture_handle_count: u64,

    lights: IndexMap<u64, lights::LightDescriptor>,
    light_handle_count: u64,

    camera: camera::Camera,
    resolution: PhysicalSize<u32>,
    oit_node_count: OITNodeCount,
    samples: MSAASetting,
    vsync: Vsync,
    debug_mode: DebugMode,

    projection_matrix: Mat4,

    surface: Surface,
    device: Device,
    queue: Queue,
    swapchain: SwapChain,
    framebuffer: TextureView,
    depth_buffer: TextureView,
    opaque_pipeline: RenderPipeline,
    pipeline_layout: PipelineLayout,
    texture_bind_group_layout: BindGroupLayout,
    sampler: Sampler,

    vert_shader: Arc<ShaderModule>,
    frag_shader: Arc<ShaderModule>,

    screenspace_triangle_verts: Buffer,

    transparency_processor: compute::CutoutTransparencyCompute,
    mip_creator: compute::MipmapCompute,
    cluster_renderer: render::cluster::Clustering,
    oit_renderer: render::oit::Oit,
    skybox_renderer: render::skybox::Skybox,
    imgui_renderer: imgui_wgpu::Renderer,

    command_buffers: Vec<CommandBuffer>,
    _renderdoc_capture: bool,
}

impl Renderer {
    pub async fn new(
        window: &Window,
        imgui_context: &mut imgui::Context,
        oit_node_count: OITNodeCount,
        samples: render::MSAASetting,
        vsync: render::Vsync,
    ) -> Self {
        let screen_size = window.inner_size();

        info!(
            "Creating renderer with: screen size = {}x{}, oit nodes = {}; samples = {}, vsync = {}",
            screen_size.width, screen_size.height, oit_node_count as u8, samples as u8, vsync
        );

        let surface = Surface::create(window);

        let adapter = Adapter::request(
            &RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
            },
            BackendBit::VULKAN | BackendBit::METAL,
        )
        .await
        .expect("Could not create Adapter");

        let (device, mut queue) = adapter
            .request_device(&DeviceDescriptor {
                extensions: Extensions {
                    anisotropic_filtering: true,
                },
                limits: Limits::default(),
            })
            .await;

        let mut startup_encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: Some("startup") });

        let swapchain_desc = render::create_swapchain_descriptor(screen_size, vsync);
        let swapchain = device.create_swap_chain(&surface, &swapchain_desc);

        let vs_module = shader!(&device; opaque - vert);

        let fs_module = shader!(&device; opaque - frag);

        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare: CompareFunction::Never,
        });

        let framebuffer = render::create_framebuffer(&device, screen_size, samples);
        let depth_buffer = render::create_depth_buffer(&device, screen_size, samples);

        let projection_matrix = perspective_matrix(
            45_f32.to_radians(),
            screen_size.width as f32 / screen_size.height as f32,
        );

        let cluster_renderer = render::cluster::Clustering::new(
            &device,
            &mut startup_encoder,
            inverse(&projection_matrix),
            frustum::Frustum::from_matrix(projection_matrix),
        );

        let texture_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            bindings: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::SampledTexture {
                        multisampled: false,
                        component_type: TextureComponentType::Uint,
                        dimension: TextureViewDimension::D2,
                    },
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Sampler { comparison: false },
                },
            ],
            label: Some("texture and sampler"),
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            bind_group_layouts: &[&texture_bind_group_layout, cluster_renderer.bind_group_layout()],
        });

        let opaque_pipeline = render::create_pipeline(&device, &pipeline_layout, &vs_module, &fs_module, samples);

        let transparency_processor = compute::CutoutTransparencyCompute::new(&device);
        let mip_creator = compute::MipmapCompute::new(&device);
        let oit_renderer = render::oit::Oit::new(
            &device,
            &mut startup_encoder,
            &vs_module,
            &texture_bind_group_layout,
            cluster_renderer.bind_group_layout(),
            &framebuffer,
            make_vec2(&[screen_size.width, screen_size.height]),
            oit_node_count,
            samples,
        );
        let skybox_renderer = render::skybox::Skybox::new(&device, &texture_bind_group_layout, samples);
        let imgui_renderer = imgui_wgpu::Renderer::new(imgui_context, &device, &mut queue, swapchain_desc.format, None);

        let screenspace_triangle_verts = screenspace::create_screen_space_verts(&device);

        // Create the Renderer object early so we can can call methods on it.
        let mut renderer = Self {
            objects: IndexMap::new(),
            object_handle_count: 0,

            mesh: IndexMap::new(),
            mesh_handle_count: 0,

            textures: IndexMap::new(),
            texture_handle_count: 0,

            lights: IndexMap::new(),
            light_handle_count: 0,

            camera: camera::Camera {
                location: make_vec3(&[-6.0, 0.0, 0.0]),
                pitch: 0.0,
                yaw: 0.0,
            },
            resolution: screen_size,
            samples,
            oit_node_count,
            projection_matrix,
            debug_mode: DebugMode::None,
            vsync,

            surface,
            device,
            queue,
            swapchain,
            framebuffer,
            depth_buffer,
            opaque_pipeline,
            pipeline_layout,
            texture_bind_group_layout,
            sampler,

            vert_shader: vs_module,
            frag_shader: fs_module,

            screenspace_triangle_verts,

            transparency_processor,
            mip_creator,
            cluster_renderer,
            oit_renderer,
            skybox_renderer,
            imgui_renderer,

            command_buffers: vec![startup_encoder.finish()],
            _renderdoc_capture: false,
        };

        // Default texture is texture handle zero, immediately discard the handle, never to be seen again
        renderer.add_texture(&RgbaImage::from_raw(1, 1, vec![0xff, 0xff, 0xff, 0xff]).expect("Invalid Image"));

        renderer
    }

    pub fn resize(&mut self, screen_size: PhysicalSize<u32>) {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: Some("resizer") });
        debug!("Resizing to {}x{}", screen_size.width, screen_size.height);
        self.framebuffer = render::create_framebuffer(&self.device, screen_size, self.samples);
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

        self.cluster_renderer.resize(
            &self.device,
            &mut encoder,
            inverse(&self.projection_matrix),
            frustum::Frustum::from_matrix(self.projection_matrix),
        );
        self.oit_renderer.resize(
            &self.device,
            make_vec2(&[screen_size.width, screen_size.height]),
            &self.framebuffer,
            self.samples,
        );
        self.command_buffers.push(encoder.finish());
    }

    pub fn set_debug(&mut self, mode: DebugMode) {
        match mode {
            DebugMode::None => {
                self.frag_shader = shader!(&self.device; opaque - fragment);
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
        );
    }

    pub fn set_samples(&mut self, samples: render::MSAASetting) {
        debug!("Setting sample count to {}", samples as u8);
        self.framebuffer = render::create_framebuffer(&self.device, self.resolution, samples);
        self.depth_buffer = render::create_depth_buffer(&self.device, self.resolution, samples);
        self.opaque_pipeline = render::create_pipeline(
            &self.device,
            &self.pipeline_layout,
            &self.vert_shader,
            &self.frag_shader,
            samples,
        );
        self.samples = samples;

        self.oit_renderer.set_samples(
            &self.device,
            &self.vert_shader,
            &self.framebuffer,
            make_vec2(&[self.resolution.width, self.resolution.height]),
            self.oit_node_count,
            samples,
        );
        self.skybox_renderer.set_samples(&self.device, samples);
    }

    pub fn set_oit_node_count(&mut self, oit_node_count: OITNodeCount) {
        debug!("Setting oit node count to {}", oit_node_count as u8);
        self.oit_renderer
            .set_node_count(&self.device, oit_node_count, self.samples);
        self.oit_node_count = oit_node_count;
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
