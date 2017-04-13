#[macro_use]
extern crate glium;
mod game;
mod graphics;

use std::time::SystemTime;
use glium::DisplayBuild;

fn main() {
    const BOARD_PADDING: u32 = 10;

    let mut board = game::Board::new();
    let width = board.width as u32 + 2 * BOARD_PADDING;
    let height = board.height as u32 + 2 * BOARD_PADDING;
    let dpy = glium::glutin::WindowBuilder::new()
        .with_title("Pong")
        .with_gl(
            glium::glutin::GlRequest::Specific(
                glium::glutin::Api::OpenGl,
                (3, 3)))
        .with_dimensions(width, height)
        .build_glium()
        .unwrap();
    let mut renderer = graphics::BoardRenderer::new(&dpy, width, height)
        .expect("Can't init board renderer.");
    let mut last_update = SystemTime::now();
    board.start_game(true);
    while board.winner().is_none() {
        renderer.draw(dpy.draw(), &board);
        for ev in dpy.poll_events() {
            match ev {
                glium::glutin::Event::Closed => return,
                glium::glutin::Event::KeyboardInput(state, _, Some(key)) => {
                    board.handle_input(key, state != glium::glutin::ElementState::Released);
                }
                glium::glutin::Event::Resized(width, height) => {
                    renderer.handle_frame_resize(width, height);
                }
                _ => ()
            }
        }
        let dt: f32 = last_update.elapsed().unwrap().subsec_nanos() as f32 / 1000000000.;
        last_update = SystemTime::now();
        board.update(dt);
    }
    println!("{} won!", if board.winner().unwrap() { "Player" } else { "AI" });
}
