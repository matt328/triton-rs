use std::sync::Arc;

use vulkano::{
    command_buffer::allocator::StandardCommandBufferAllocator,
    descriptor_set::allocator::StandardDescriptorSetAllocator, device::{Features, DeviceExtensions}, instance::{InstanceCreateInfo, InstanceExtensions, InstanceCreateFlags}, Version,
};
use vulkano_util::{
    context::{VulkanoConfig, VulkanoContext},
    window::{VulkanoWindows, WindowDescriptor},
};
use winit::event_loop::EventLoop;

pub const WINDOW_WIDTH: f32 = 1280.0;
pub const WINDOW_HEIGHT: f32 = 720.0;

pub struct App {
    pub context: VulkanoContext,
    pub windows: VulkanoWindows,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    pub descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
}

impl App {
    pub fn open(&mut self, event_loop: &EventLoop<()>) {
        // Create window
        let _id1 = self.windows.create_window(
            event_loop,
            &self.context,
            &WindowDescriptor {
                width: WINDOW_WIDTH,
                height: WINDOW_HEIGHT,
                title: "Triton Application".to_string(),
                ..Default::default()
            },
            |_| {},
        );
    }
}

impl Default for App {
    fn default() -> Self {
        let config = VulkanoConfig {
            instance_create_info: InstanceCreateInfo {
                #[cfg(target_os = "macos")]
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                max_api_version: Some(Version::V1_2),
                enabled_extensions: InstanceExtensions {
                    #[cfg(target_os = "macos")]
                    khr_portability_enumeration: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            device_extensions: DeviceExtensions {
                khr_dynamic_rendering: true,
                #[cfg(target_os = "macos")]
                khr_portability_subset: true,
                khr_swapchain: true,
                ..Default::default()
            },
            device_features: Features {
                dynamic_rendering: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let context = VulkanoContext::new(config);
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            context.device().clone(),
            Default::default(),
        ));
        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            context.device().clone(),
        ));

        App {
            context,
            windows: VulkanoWindows::default(),
            command_buffer_allocator,
            descriptor_set_allocator,
        }
    }
}
