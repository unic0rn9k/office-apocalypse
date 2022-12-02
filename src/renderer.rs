use sdl2::video::{GLContext, Window};

pub struct Renderer<'a> {
    window: &'a Window,
    _context: GLContext,
}

impl<'a> Renderer<'a> {
    pub fn new(window: &'a Window, video_subsystem: sdl2::VideoSubsystem) -> Result<Self, String> {
        gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const _);
        let _context = window.gl_create_context()?;
        Ok(Self { window, _context })
    }

    pub fn render(&mut self) -> Result<(), String> {
        unsafe { gl::ClearColor(1.0, 0.0, 0.0, 1.0) };
        unsafe { gl::Clear(gl::COLOR_BUFFER_BIT) };

        self.window.gl_swap_window();
        Ok(())
    }
}
