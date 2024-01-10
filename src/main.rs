#![allow(clippy::eq_op)]

use std::sync::Arc;

use egui_winit_vulkano::{Gui, GuiConfig};
use triton::{renderer, GuiState, RenderPipeline, TimeInfo};
use vulkano::{
    command_buffer::allocator::{
        StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
    },
    format::Format,
    image::{view::ImageView, Image, ImageCreateInfo, ImageType, ImageUsage},
    memory::allocator::AllocationCreateInfo,
    sync::{self, GpuFuture},
};
use vulkano_util::{
    context::{VulkanoConfig, VulkanoContext},
    renderer::DEFAULT_IMAGE_FORMAT,
    window::{VulkanoWindows, WindowDescriptor},
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

pub fn main() -> anyhow::Result<()> {
    // Winit event loop & our time tracking initialization
    let event_loop = EventLoop::new();
    let mut time = TimeInfo::new();
    // Create renderer for our scene & ui
    let scene_view_size = [256, 256];
    // Vulkano context
    let context = VulkanoContext::new(VulkanoConfig::default());
    // Vulkano windows (create one)
    let mut windows = VulkanoWindows::default();
    windows.create_window(&event_loop, &context, &WindowDescriptor::default(), |ci| {
        ci.image_format = Format::B8G8R8A8_UNORM;
        ci.min_image_count = ci.min_image_count.max(2);
    });
    let renderer = windows.get_primary_renderer_mut().unwrap();
    // Create gui as main render pass (no overlay means it clears the image each frame)
    let mut gui = {
        Gui::new(
            &event_loop,
            renderer.surface(),
            renderer.graphics_queue(),
            renderer.swapchain_format(),
            GuiConfig {
                is_overlay: true,
                ..Default::default()
            },
        )
    };
    // Create a simple image to which we'll draw the triangle scene
    let scene_image = ImageView::new_default(
        Image::new(
            context.memory_allocator().clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: DEFAULT_IMAGE_FORMAT,
                extent: [scene_view_size[0], scene_view_size[1], 1],
                array_layers: 1,
                usage: ImageUsage::SAMPLED | ImageUsage::COLOR_ATTACHMENT,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap(),
    )
    .unwrap();

    // Create our render pipeline
    let mut scene_render_pipeline = RenderPipeline::new(
        context.graphics_queue().clone(),
        renderer.swapchain_format(),
        &renderer::Allocators {
            command_buffers: Arc::new(StandardCommandBufferAllocator::new(
                context.device().clone(),
                StandardCommandBufferAllocatorCreateInfo {
                    secondary_buffer_count: 32,
                    ..Default::default()
                },
            )),
            memory: context.memory_allocator().clone(),
        },
    );
    // Create gui state (pass anything your state requires)
    let mut gui_state = GuiState::new(&mut gui, scene_image.clone(), scene_view_size);
    // Event loop run
    event_loop.run(move |event, _, control_flow| {
        let renderer = windows.get_primary_renderer_mut().unwrap();
        // Update Egui integration so the UI works!
        match event {
            Event::WindowEvent { event, window_id } if window_id == renderer.window().id() => {
                let _pass_events_to_game = !gui.update(&event);
                match event {
                    WindowEvent::Resized(_) => {
                        renderer.resize();
                    }
                    WindowEvent::ScaleFactorChanged { .. } => {
                        renderer.resize();
                    }
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => (),
                }
            }
            Event::RedrawRequested(window_id) if window_id == window_id => {
                // Set immediate UI in redraw here
                // It's a closure giving access to egui context inside which you can call anything.
                // Here we're calling the layout of our `gui_state`.
                gui.immediate_ui(|gui| {
                    let ctx = gui.context();
                    gui_state.layout(ctx, renderer.window_size(), time.fps())
                });
                // Render UI
                // Acquire swapchain future
                let before_future = match renderer.acquire() {
                    Ok(future) => future,
                    Err(vulkano::VulkanError::OutOfDate) => {
                        renderer.resize();
                        sync::now(context.device().clone()).boxed()
                    }
                    Err(e) => panic!("Failed to acquire swapchain future: {}", e),
                };
                // Draw scene
                // let after_scene_draw =
                //     scene_render_pipeline.render(before_future, scene_image.clone());
                // Render gui

                let after_scene_draw =
                    scene_render_pipeline.render(before_future, renderer.swapchain_image_view());

                let after_future =
                    gui.draw_on_image(after_scene_draw, renderer.swapchain_image_view());

                // Present swapchain
                renderer.present(after_future, true);

                // Update fps & dt
                time.update();
            }
            Event::MainEventsCleared => {
                renderer.window().request_redraw();
            }
            _ => (),
        }
    })
}
