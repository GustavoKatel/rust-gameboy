use sdl2;
use sdl2::Sdl;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::Renderer as SDLRenderer;
use sdl2::render::Texture as SDLTexture;

pub enum SDLDisplayEvent {
    Quit,
}

pub struct SDLDisplay {

    context: Sdl,
    renderer: SDLRenderer<'static>,
    texture: SDLTexture,
    height: u32,
    width: u32,
    events: Vec<SDLDisplayEvent>,

}

impl SDLDisplay {

    pub fn new(height: u32, width: u32, window_name: String) -> SDLDisplay {

        let context = sdl2::init().unwrap();
        let video_subsystem = context.video().unwrap();

        let window = video_subsystem.window(&window_name, width, height)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

        let renderer = window.renderer().build().unwrap();

        // creates a texture that matches the size of the GB display
        let texture = renderer.create_texture_streaming(
        PixelFormatEnum::RGB24, 160, 144).unwrap();

        SDLDisplay{
            context: context,
            renderer: renderer,
            texture: texture,
            width: width,
            height: height,
            events: vec!(),
        }

    }

    pub fn step(&mut self) {

        self.renderer.clear();
        self.renderer.copy(&self.texture, None, Some(Rect::new(0, 0, self.width, self.height))).unwrap();
        self.renderer.present();

        self.events.clear();
        let mut event_pump = self.context.event_pump().unwrap();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..}
                | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    self.events.push(SDLDisplayEvent::Quit);
                },
                // TODO: joystick
                _ => {}
            }
        }

    }

    pub fn get_events<'a>(&'a self) -> &'a Vec<SDLDisplayEvent> {
        &self.events
    }

}
