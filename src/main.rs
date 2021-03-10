mod camera;
mod model;
mod texture;
mod bsp;
mod bsp_look_up;

use winit::{
    event::*,
    event_loop::{EventLoop, ControlFlow},
    window::{Window, WindowBuilder},
};

use futures::executor::block_on;
use wgpu::util::DeviceExt;
use cgmath::SquareMatrix;
use cgmath::InnerSpace;
use cgmath::Rotation3;
use cgmath::Zero;
use std::mem;
use std::time::{Instant, Duration};

use model::{DrawModel, Vertex};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
}

impl Uniforms {
    fn new() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
            model: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &camera::Camera, projection: &camera::Projection) {
        self.view_proj = (projection.calc_matrix() * camera.view).into();
    }

    //fn update_model(&mut self, model: cgmath::Matrix4<f32>) {
     //   self.model = model.into();
    //}
}

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    bsp_model_render_pipeline: wgpu::RenderPipeline,
    camera: camera::Camera,
    projection: camera::Projection,
    camera_controller: camera::CameraController,
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    //obj_model: model::Model,
    depth_texture: texture::Texture,
    bsp: bsp::Bsp,
}

impl State {

    async fn new(window: &Window) -> Self {

        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                shader_validation: true,
            },
            None,
        ).await.unwrap();

        //Fifo or Immediate (vsync on and off)
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let camera = camera::Camera::new();
        let projection = camera::Projection::new(sc_desc.width, sc_desc.height, cgmath::Deg(90.0), 0.1, 4000.0);
        let camera_controller = camera::CameraController::new(3.0, 3.0);

        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera, &projection);

        let uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Uniform Buffer"),
                contents: bytemuck::cast_slice(&[uniforms]),
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            }
        );

        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::UniformBuffer {
                        dynamic: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("uniform_bind_group_layout"),
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(uniform_buffer.slice(..)),
                }
            ],
            label: Some("uniform_bind_group"),
        });

        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Uint,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler { comparison: false },
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        let lightmap_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Uint,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler { comparison: false },
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        let res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");
        //let obj_model = model::Model::load(&device, &queue, &texture_bind_group_layout, res_dir.join("cube.obj"),).unwrap();

        let vs_bsp_model_module = device.create_shader_module(wgpu::include_spirv!("bsp_model.vert.spv"));
        let fs_bsp_model_module = device.create_shader_module(wgpu::include_spirv!("bsp_model.frag.spv"));
        let vs_module = device.create_shader_module(wgpu::include_spirv!("bsp.vert.spv"));
        let fs_module = device.create_shader_module(wgpu::include_spirv!("bsp.frag.spv"));

        let mut bsp = bsp::Bsp::new(&device, &queue, &texture_bind_group_layout, &lightmap_bind_group_layout);

        let depth_texture = texture::Texture::create_depth_texture(&device, &sc_desc, "depth_texture");

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &lightmap_bind_group_layout, &uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(
                wgpu::RasterizationStateDescriptor {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: wgpu::CullMode::Front,
                    depth_bias: 0,
                    depth_bias_slope_scale: 0.0,
                    depth_bias_clamp: 0.0,
                    clamp_depth: false,
                }
            ),
            color_states: &[
                wgpu::ColorStateDescriptor {
                    format: sc_desc.format,
                    color_blend: wgpu::BlendDescriptor::REPLACE, //color_blend: wgpu::BlendDescriptor::REPLACE, //alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                },
            ],
            primitive_topology: wgpu::PrimitiveTopology::TriangleList, //LineList TriangleList
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilStateDescriptor::default(),
            }),
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint32,
                vertex_buffers: &[bsp::Vertex::desc()],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let bsp_model_render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let bsp_model_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Bsp model Pipeline"),
            layout: Some(&bsp_model_render_pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_bsp_model_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_bsp_model_module,
                entry_point: "main",
            }),
            rasterization_state: Some(
                wgpu::RasterizationStateDescriptor {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: wgpu::CullMode::Front,
                    depth_bias: 0,
                    depth_bias_slope_scale: 0.0,
                    depth_bias_clamp: 0.0,
                    clamp_depth: false,
                }
            ),
            color_states: &[
                wgpu::ColorStateDescriptor {
                    format: sc_desc.format,
                    color_blend: wgpu::BlendDescriptor {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add
                    },
                    alpha_blend: wgpu::BlendDescriptor {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add
                    },
                    write_mask: wgpu::ColorWrite::ALL,
                },
            ],
            primitive_topology: wgpu::PrimitiveTopology::TriangleList, //LineList TriangleList
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilStateDescriptor::default(),
            }),
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint32,
                vertex_buffers: &[bsp::Vertex::desc()],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            render_pipeline,
            bsp_model_render_pipeline,
            camera,
            projection,
            camera_controller,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            depth_texture,
            bsp,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {

        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.projection.resize(new_size.width, new_size.height);
        self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.sc_desc, "depth_texture");
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    fn input(&mut self, event: &WindowEvent, window: &Window) -> bool {

        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
                self.camera_controller.process_keyboard(*keycode, *state);
                true
            }
            WindowEvent::CursorMoved  { position, .. } => {
                self.camera_controller.process_mouse((position.x as f32 / self.sc_desc.width as f32) - 0.5, 0.5 - (position.y as f32 / self.sc_desc.height as f32), &mut self.camera);
                window.set_cursor_position(winit::dpi::PhysicalPosition::new(self.sc_desc.width as f32 / 2.0, self.sc_desc.height as f32 / 2.0));
                //self.camera_controller.process_scroll(delta);
                true
            }/*
            WindowEvent::Button {
                button: 1,
                state,
            } => {
                //left mosue press
                true
            }
            WindowEvent::MouseMotion { delta } => {
                self.camera_controller.process_mouse(delta.0, delta.1);
                true
            }*/
            _ => false,
        }
    }

    fn update(&mut self) {

        //println!("{:?}", self.camera.position);
        let start = cgmath::Vector3::new(self.camera.position[0], self.camera.position[1], self.camera.position[2]);
        self.camera_controller.update_camera(&mut self.camera);
        self.uniforms.update_view_proj(&self.camera, &self.projection);
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniforms]));

        self.bsp.trace_ray(start, cgmath::Vector3::new(self.camera.position[0], self.camera.position[1], self.camera.position[2]));
    }

    fn render(&mut self) -> Result<(), wgpu::SwapChainError> {

        let frame = self.swap_chain.get_current_frame()?.output;
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        //let mut now = Instant::now();
        let mut offset = 0usize;
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[
                    wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &frame.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: true,
                        }
                    }
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            //Draw bsp
            let model_index = (self.bsp.indices_per_texture.len() - 1);
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.bsp.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.bsp.index_buffer.slice(..));
            render_pass.set_bind_group(2, &self.uniform_bind_group, &[]);
            for j in 0..(self.bsp.indices_per_texture.len() - 1) {
                for i in 0..self.bsp.indices_per_texture[j].len() {
                    render_pass.set_bind_group(0, &self.bsp.materials[i].bind_group, &[]);
                    render_pass.set_bind_group(1, &self.bsp.materials_light[j].bind_group, &[]);
                    render_pass.draw_indexed((offset as u32)..((offset + self.bsp.indices_per_texture[j][i].len()) as u32), 0, 0..1);
                    offset += self.bsp.indices_per_texture[j][i].len();
                }
            }

            //Draw models
            /*render_pass.set_pipeline(&self.bsp_model_render_pipeline);
            render_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
            for i in 0..self.bsp.indices_per_texture[model_index].len() {
                if self.bsp.indices_per_texture[model_index][i].len() != 0 {
                    render_pass.set_bind_group(0, &self.bsp.materials[i].bind_group, &[]);
                    render_pass.draw_indexed((offset as u32)..((offset + self.bsp.indices_per_texture[model_index][i].len()) as u32), 0, 0..1);
                    offset += self.bsp.indices_per_texture[model_index][i].len();
                }
            }*/
        }
        //println!("Frame time {}", (now.elapsed().as_nanos() as f32) / 1000000.0);
        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    window.set_cursor_grab(true);
    window.set_cursor_visible(false);
    let mut state = block_on(State::new(&window));
    let mut fps: i32 = 0;
    let mut run_time = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                state.input(event, &window);
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::KeyboardInput {
                        input,
                        ..
                    } => {
                        match input {
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            } => *control_flow = ControlFlow::Exit,
                            _ => {}
                        }
                    },
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(_) => {

                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SwapChainError::Lost) => state.resize(state.size),
                    Err(wgpu::SwapChainError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                fps += 1;
                if run_time.elapsed().as_millis() >= 1000 {
                    println!("fps {}", fps);
                    fps = 0;
                    run_time = Instant::now();
                }
                window.request_redraw();
            }
            _ => {}
        }
    });
}
