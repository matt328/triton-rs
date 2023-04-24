use crate::*;

pub use helper::*;

mod helper {
    use super::*;
    use std::sync::Arc;
    use winit::event::Event;
    use winit::event_loop::{ControlFlow, EventLoop};
    use winit::window::Window;

    pub use winit;

    pub fn game_loop<G, U, R, H, T>(
        event_loop: EventLoop<T>,
        window: Arc<Window>,
        game: G,
        updates_per_second: u32,
        max_frame_time: f64,
        mut update: U,
        mut render: R,
        mut handler: H,
    ) -> !
    where
        G: 'static,
        U: FnMut(&mut GameLoop<G, Time, Arc<Window>>) + 'static,
        R: FnMut(&mut GameLoop<G, Time, Arc<Window>>) + 'static,
        H: FnMut(&mut GameLoop<G, Time, Arc<Window>>, &Event<'_, T>) + 'static,
        T: 'static,
    {
        let mut game_loop = GameLoop::new(game, updates_per_second, max_frame_time, window);

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            // Forward events to existing handlers.
            handler(&mut game_loop, &event);

            match event {
                Event::RedrawRequested(_) => {
                    if !game_loop.next_frame(&mut update, &mut render) {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                Event::MainEventsCleared => {
                    game_loop.window.request_redraw();
                }
                _ => {}
            }
        })
    }
}
