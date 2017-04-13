extern crate glium;

use game;
use std::io::prelude::*;
use std::fs::File;
use glium::Surface;

const VERT_PATH: &'static str = "shaders/vert.glsl";
const GEOM_PATH: &'static str = "shaders/geom.glsl";
const FRAG_PATH: &'static str = "shaders/frag.glsl";
const BATCH_SIZE: u32 = 100;
const BORDER_WIDTH: f32 = 2.;
const DIGIT_LINE_SIZE: f32 = 20.;
const DIGIT_LINE_THICKNESS: f32 = 5.;
const DIGIT_SPACING: f32 = 2. * DIGIT_LINE_THICKNESS;
const NET_WIDTH: f32 = 1.;
const SCORE_PADDING: f32 = 20.;
const NUM_NET_SEGMENTS: u32 = 20;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    dimension: [f32; 2],
}

implement_vertex!(Vertex, position, dimension);

pub struct BoardRenderer {
    program: glium::Program,
    vertex_buffer: glium::VertexBuffer<Vertex>,
    shape: Vec<Vertex>,
    projection: [f32; 2],
    batch_index: u32
}

impl BoardRenderer {
    pub fn new(dpy: &glium::backend::Facade, width: u32, height: u32) -> Result<Self, ()> {
        let shape: Vec<Vertex> = (0..BATCH_SIZE).map(|_| Vertex {
                position: [0., 0.],
                dimension: [0., 0.]
            })
            .collect();
        let vertex_buffer = glium::VertexBuffer::dynamic(dpy, &shape).map_err(|_| {})?;

        fn read_file(path: &str) -> Result<String, ()> {
            let mut file = File::open(path).map_err(|_| {})?;
            let mut contents = String::new();
            file.read_to_string(&mut contents).map_err(|_| {})?;
            Ok(contents)
        }

        let program = glium::Program::from_source(dpy,
            &read_file(VERT_PATH)?,
            &read_file(FRAG_PATH)?,
            Some(&read_file(GEOM_PATH)?))
                .map_err(|e| {
                    match e {
                        glium::CompilationError(log) |
                        glium::LinkingError(log) => println!("{}", log.trim()),
                        _ => {}
                    }
                })?;

        Ok(BoardRenderer {
            program: program,
            vertex_buffer: vertex_buffer,
            shape: shape,
            projection: [width as f32, height as f32],
            batch_index: 0
        })
    }

    pub fn handle_frame_resize(&mut self, width: u32, height: u32) {
        self.projection = [
            width as f32,
            height as f32
        ];
    }

    fn draw_rect(&mut self, frame: &mut glium::Frame, board: &game::Board, rect: game::Rect) {
        if self.batch_index == BATCH_SIZE {
            self.flush_draw_batch(frame, board);
        }
        self.shape[self.batch_index as usize] = Vertex {
            position: [rect.x, rect.y],
            dimension: [rect.width, rect.height]
        };
        self.batch_index += 1;
    }

    fn draw_digit(&mut self, frame: &mut glium::Frame, board: &game::Board, digit: u8, x: f32, y: f32) {
        let lines = [
            game::Rect {
                x: 0.,
                y: DIGIT_LINE_SIZE * 2. - DIGIT_LINE_THICKNESS,
                width: DIGIT_LINE_SIZE,
                height: DIGIT_LINE_THICKNESS
            },
            game::Rect {
                x: DIGIT_LINE_SIZE - DIGIT_LINE_THICKNESS,
                y: DIGIT_LINE_SIZE,
                width: DIGIT_LINE_THICKNESS,
                height: DIGIT_LINE_SIZE
            },
            game::Rect {
                x: DIGIT_LINE_SIZE - DIGIT_LINE_THICKNESS,
                y: 0.,
                width: DIGIT_LINE_THICKNESS,
                height: DIGIT_LINE_SIZE
            },
            game::Rect {
                x: 0.,
                y: 0.,
                width: DIGIT_LINE_SIZE,
                height: DIGIT_LINE_THICKNESS
            },
            game::Rect {
                x: 0.,
                y: 0.,
                width: DIGIT_LINE_THICKNESS,
                height: DIGIT_LINE_SIZE
            },
            game::Rect {
                x: 0.,
                y: DIGIT_LINE_SIZE,
                width: DIGIT_LINE_THICKNESS,
                height: DIGIT_LINE_SIZE
            },
            game::Rect {
                x: 0.,
                y: DIGIT_LINE_SIZE - DIGIT_LINE_THICKNESS / 2.,
                width: DIGIT_LINE_SIZE,
                height: DIGIT_LINE_THICKNESS
            }
        ];
        let digit_to_lines: [Vec<u8>; 10] = [
            vec![0, 1, 2, 3, 4, 5],
            vec![1, 2],
            vec![0, 1, 3, 4, 6],
            vec![0, 1, 2, 3, 6],
            vec![1, 2, 5, 6],
            vec![0, 2, 3, 5, 6],
            vec![0, 2, 3, 4, 5, 6],
            vec![0, 1, 2],
            vec![0, 1, 2, 3, 4, 5, 6],
            vec![0, 1, 2, 5, 6]
        ];
        for &i in digit_to_lines[digit as usize].iter() {
            self.draw_rect(frame, board, lines[i as usize].translate(x, y));
        }
    }

    fn draw_number(&mut self, frame: &mut glium::Frame, board: &game::Board, number: u32, x: f32, y: f32, align_left: bool) {
        let mut digits = Vec::new();
        if number == 0 {
            digits.push(0u8);
        } else {
            let mut t_number = number;
            while t_number != 0 {
                digits.push((t_number % 10) as u8);
                t_number /= 10;
            }
        }
        let mut cursor = if align_left {
            (x, y)
        } else {
            (x - (digits.len() as f32) * (DIGIT_LINE_SIZE + DIGIT_SPACING) + DIGIT_SPACING, y)
        };
        for &digit in digits.iter().rev() {
            let (x, y) = cursor;
            self.draw_digit(frame, board, digit, x, y);
            cursor = (x + DIGIT_LINE_SIZE + DIGIT_SPACING, y);
        }
    }

    fn flush_draw_batch(&mut self, frame: &mut glium::Frame, board: &game::Board) {
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::Points);
        self.vertex_buffer.write(&self.shape);
        frame.draw(
            self.vertex_buffer.slice(..self.batch_index as usize).unwrap(), 
            &indices, &self.program, &uniform!{
                projection: self.projection,
                offset: [
                    (self.projection[0] - board.width as f32) / 2.,
                    (self.projection[1] - board.height as f32) / 2.
                ]
            },
            &Default::default()).unwrap();
        self.batch_index = 0;
    }

    pub fn draw(&mut self, mut frame: glium::Frame, board: &game::Board) {
        self.vertex_buffer.write(&self.shape);
        frame.clear_color(0.0, 0.0, 1.0, 1.0);
        self.draw_rect(&mut frame, board, board.lhs_paddle.bound.clone());
        self.draw_rect(&mut frame, board, board.rhs_paddle.bound.clone());
        let lhs_goal_border_height = (board.height - board.lhs_goal_height) / 2.;
        let rhs_goal_border_height = (board.height - board.rhs_goal_height) / 2.;
        self.draw_rect(&mut frame, board, game::Rect {
            x: -BORDER_WIDTH, y: 0., 
            width: BORDER_WIDTH, height: lhs_goal_border_height
        });
        self.draw_rect(&mut frame, board, game::Rect {
            x: -BORDER_WIDTH, y: board.height - lhs_goal_border_height, 
            width: BORDER_WIDTH, height: lhs_goal_border_height
        });
        self.draw_rect(&mut frame, board, game::Rect {
            x: -BORDER_WIDTH, y: -BORDER_WIDTH,
            width: board.width + 2. * BORDER_WIDTH, height: BORDER_WIDTH
        });
        self.draw_rect(&mut frame, board, game::Rect {
            x: -BORDER_WIDTH, y: board.height,
            width: board.width + 2. * BORDER_WIDTH, height: BORDER_WIDTH
        });
        self.draw_rect(&mut frame, board, game::Rect {
            x: board.width, y: 0.,
            width: BORDER_WIDTH, height: rhs_goal_border_height
        });
        self.draw_rect(&mut frame, board, game::Rect {
            x: board.width, y: board.height - rhs_goal_border_height,
            width: BORDER_WIDTH, height: rhs_goal_border_height
        });
        self.draw_number(&mut frame, board, board.lhs_score, board.width / 2. - SCORE_PADDING, SCORE_PADDING, false);
        self.draw_number(&mut frame, board, board.rhs_score, board.width / 2. + SCORE_PADDING, SCORE_PADDING, true);
        for i in 0..NUM_NET_SEGMENTS {
            self.draw_rect(&mut frame, board, game::Rect {
                x: board.width / 2., y: (i as f32 + 0.25) * board.height / (NUM_NET_SEGMENTS as f32),
                width: NET_WIDTH, height: board.height / (2 * NUM_NET_SEGMENTS) as f32
            });
        }
        self.draw_rect(&mut frame, board, board.ball.bound.clone());
        self.flush_draw_batch(&mut frame, board);
        frame.finish().unwrap();
    }
}
