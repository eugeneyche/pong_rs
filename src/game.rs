extern crate glium;
extern crate ears;
use ears::{Sound, AudioController};

const BOARD_WIDTH: f32 = 600.;
const BOARD_HEIGHT: f32 = 300.;
const PADDLE_X_OFFSET: f32 = 10.;
const PADDLE_WIDTH: f32 = 10.;
const PADDLE_HEIGHT: f32 = 75.;
const GOAL_HEIGHT: f32 = 240.;
const BALL_RADIUS: f32 = 5.;
const BALL_SPEEDUP: f32 = 1.1;
const BALL_START_SPEED: f32 = 250.;
const PADDLE_MAX_SPEED: f32 = 500.;
const PADDLE_FRICTION: f32 = 0.005;
const PADDLE_BALL_INFLUENCE: f32 = 0.25;
const PADDLE_CURVE: f32 = 0.2;
const PLAYER_PADDLE_ACCEL: f32 = 2000.;
const AI_PADDLE_P_FACTOR: f32 = 30.;
const AI_PADDLE_I_FACTOR: f32 = 0.1;
const AI_PADDLE_D_FACTOR: f32 = 10.;
const BEEP_PATH: &'static str = "sounds/beep.wav";
const TICK_PATH: &'static str = "sounds/tick.wav";
const ERROR_PATH: &'static str = "sounds/error.wav";

const WIN_SCORE: u32 = 10;

#[derive(Copy, Clone)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn translate(&self, dx: f32, dy: f32) -> Rect {
        Rect {
            x: self.x + dx,
            y: self.y + dy,
            width: self.width,
            height: self.height
        }
    }
}

pub struct Paddle {
    pub bound: Rect,
    pub dy: f32,
    pub ddy: f32
}

pub struct Ball {
    pub bound: Rect,
    pub dx: f32,
    pub dy: f32
}

impl Ball {
    pub fn speedup(&mut self, amount: f32) {
        self.dx *= amount; 
        self.dy *= amount; 
    }
}

pub struct Board {
    pub lhs_score: u32,
    pub rhs_score: u32,
    pub width: f32,
    pub height: f32,
    pub lhs_goal_height: f32,
    pub rhs_goal_height: f32,
    pub lhs_paddle: Paddle,
    pub rhs_paddle: Paddle,
    pub ball: Ball,
    override_ball_sim: bool,
    ai_last_offset: f32,
    ai_accum_offset: f32,
    beep_snd: Sound,
    tick_snd: Sound,
    error_snd: Sound
}

fn collides(sx: f32, sy: f32, sdx: f32, sdy: f32, tx: f32, ty: f32, tdx: f32, tdy: f32) -> (f32, f32) {
    let (stdx, stdy) = (sx - tx, sy - ty);
    let ct = (tdx * stdx + tdy * stdy) / (tdx * tdx + tdy * tdy);
    let nx = (tx + ct * tdx) - sx;
    let ny = (ty + ct * tdy) - sy;
    let cs = (nx * nx + ny * ny) /  (nx * sdx + ny * sdy);
    (cs, ct)
}

impl Board {
    pub fn new() -> Self {
        Board {
            lhs_score: 0,
            rhs_score: 0,
            width: BOARD_WIDTH,
            height: BOARD_HEIGHT,
            lhs_paddle: Paddle {
                bound: Rect {
                    x: PADDLE_X_OFFSET - PADDLE_WIDTH / 2.,
                    y: BOARD_HEIGHT / 2. - PADDLE_HEIGHT / 2.,
                    width: PADDLE_WIDTH,
                    height: PADDLE_HEIGHT
                },
                dy: 0.,
                ddy: 0. 
            },
            rhs_paddle: Paddle {
                bound: Rect {
                    x: BOARD_WIDTH - PADDLE_X_OFFSET - PADDLE_WIDTH / 2.,
                    y: BOARD_HEIGHT / 2. - PADDLE_HEIGHT / 2.,
                    width: PADDLE_WIDTH,
                    height: PADDLE_HEIGHT
                },
                dy: 0.,
                ddy: 0. 
            },
            lhs_goal_height: GOAL_HEIGHT,
            rhs_goal_height: GOAL_HEIGHT,
            ball: Ball {
                bound: Rect {
                    x: BOARD_WIDTH / 2. - BALL_RADIUS,
                    y: BOARD_HEIGHT / 2. - BALL_RADIUS,
                    width: 2. * BALL_RADIUS,
                    height: 2. * BALL_RADIUS
                },
                dx: 0.,
                dy: 0.
            },
            override_ball_sim: false,
            ai_last_offset: 0.,
            ai_accum_offset: 0.,
            beep_snd: Sound::new(BEEP_PATH).expect("Failed to load beep sound."),
            tick_snd: Sound::new(TICK_PATH).expect("Failed to load tick sound."),
            error_snd: Sound::new(ERROR_PATH).expect("Failed to load error sound.")
        }
    }

    pub fn winner(&self) -> Option<bool> {
        if self.lhs_score == WIN_SCORE { Some(true) }
        else if self.rhs_score == WIN_SCORE { Some(false) }
        else { None }
    }

    pub fn update(&mut self, dt: f32) {
        // ai sim
        let paddle_offset = (self.ball.bound.y + self.ball.bound.height / 2.) - (self.rhs_paddle.bound.y + self.rhs_paddle.bound.height / 2.);
        if self.ai_accum_offset.signum() != paddle_offset.signum() {
            self.ai_accum_offset = 0.;
        } else {
            self.ai_accum_offset += paddle_offset;
        }
        let p = paddle_offset;
        let i = self.ai_accum_offset;
        let d = (p - self.ai_last_offset) / dt;
        self.ai_last_offset = p;
        self.rhs_paddle.ddy = AI_PADDLE_P_FACTOR * p + AI_PADDLE_I_FACTOR * i + AI_PADDLE_D_FACTOR * d;
        // paddle sim
        for ref mut paddle in [&mut self.lhs_paddle, &mut self.rhs_paddle].iter_mut() {
            paddle.dy += dt * paddle.ddy;
            if paddle.dy.abs() > PADDLE_MAX_SPEED {
                paddle.dy = paddle.dy.signum() * PADDLE_MAX_SPEED;
            }
            paddle.dy *= PADDLE_FRICTION.powf(dt);
            let mut y = paddle.bound.y + paddle.dy * dt;
            if y < 0. {
                y = 0.;
                paddle.dy = paddle.dy.abs();
            } else if y + paddle.bound.height > BOARD_HEIGHT {
                y = BOARD_HEIGHT - paddle.bound.height;
                paddle.dy = -paddle.dy.abs();
            }
            paddle.bound.y = y;
        }
        const MAX_ITERATIONS: u32 = 10;
        let mut iterations = 0;
        let mut has_collide = true;
        let mut dt_left = dt;

        let lhs_goal_border_height = (self.height - self.lhs_goal_height) / 2.;
        let rhs_goal_border_height = (self.height - self.rhs_goal_height) / 2.;
        enum Normal<'a> {
            Static(f32, f32),
            Dynamic(f32, f32, &'a Fn(f32, &Ball) -> (f32, f32))
        }
        fn reflect(vx: f32, vy: f32, nx: f32, ny: f32) -> (f32, f32) {
            let dot = -2. * (nx * vx + ny * vy) / (nx * nx + ny * ny);
            (vx + dot * nx, vy + dot * ny)
        }
        fn paddle_reflect(nx: f32, paddle_dy: f32, ct: f32, ball: &Ball) -> (f32, f32) {
           let ny = (2. * ct - 1.) * PADDLE_CURVE;
           let (dx, dy) = reflect(ball.dx, ball.dy, nx, ny);
           (dx, dy + paddle_dy * PADDLE_BALL_INFLUENCE)
        }
        let lhs_paddle_dy = self.lhs_paddle.dy;
        let rhs_paddle_dy = self.rhs_paddle.dy;
        let lhs_reflect_fn = |ct: f32, ball: &Ball| { paddle_reflect(1., lhs_paddle_dy, ct, ball) };
        let rhs_reflect_fn = |ct: f32, ball: &Ball| { paddle_reflect(-1., rhs_paddle_dy, ct, ball) };
        let mut hit_paddle_lhs = false;
        let mut hit_paddle_rhs = false;
        let mut hit_any = false;
        {
            let mut lhs_callback_fn = || { hit_paddle_lhs = true; };
            let mut rhs_callback_fn = || { hit_paddle_rhs = true; };
            let mut basic_colliders: [(f32, f32, f32, f32, Normal, Option<&mut FnMut()>); 8] = [
                (0., BALL_RADIUS, self.width, 0., Normal::Static(0., 1.), None),
                (0., self.height - BALL_RADIUS, self.width, 0., Normal::Static(0., -1.), None),
                (BALL_RADIUS, 0., 0., lhs_goal_border_height, Normal::Static(1., 0.), None),
                (BALL_RADIUS, self.height, 0., -lhs_goal_border_height, Normal::Static(1., 0.), None),
                (self.width - BALL_RADIUS, 0., 0., lhs_goal_border_height, Normal::Static(-1., 0.), None),
                (self.width - BALL_RADIUS, self.height, 0., -rhs_goal_border_height, Normal::Static(-1., 0.), None),
                (self.lhs_paddle.bound.x + self.lhs_paddle.bound.width + BALL_RADIUS, self.lhs_paddle.bound.y, 0., self.lhs_paddle.bound.height, 
                    Normal::Dynamic(1., 0., &lhs_reflect_fn), Some(&mut lhs_callback_fn)),
                (self.rhs_paddle.bound.x - BALL_RADIUS, self.rhs_paddle.bound.y, 0., self.rhs_paddle.bound.height, 
                    Normal::Dynamic(-1., 0., &rhs_reflect_fn), Some(&mut rhs_callback_fn)),
            ];
            while dt_left > 0. && has_collide && iterations < MAX_ITERATIONS {
                iterations += 1;
                has_collide = false;
                let iter_dx = self.ball.dx * dt_left;
                let iter_dy = self.ball.dy * dt_left;
                for &mut (tx, ty, tdx, tdy, ref normal, ref mut callback) in basic_colliders.iter_mut() {
                    let (nx, ny) = match normal {
                        &Normal::Static(x, y) => (x, y),
                        &Normal::Dynamic(x, y, _) => (x, y)
                    };
                    if nx * self.ball.dx + ny * self.ball.dy >= 0. { continue; }
                    let (cs, ct) = collides(self.ball.bound.x + BALL_RADIUS, self.ball.bound.y + BALL_RADIUS, iter_dx, iter_dy, tx, ty, tdx, tdy);
                    if 0. < cs && cs <= 1. && 0. < ct && ct <= 1. {
                        has_collide = true;
                        hit_any = true;
                        self.ball.bound.x += iter_dx * (cs - 0.001);
                        self.ball.bound.y += iter_dy * (cs - 0.001);
                        let (dx, dy) = if let &Normal::Dynamic(_, _, reflect_fn) = normal {
                            reflect_fn(ct, &self.ball)
                        } else {
                            reflect(self.ball.dx, self.ball.dy, nx, ny)
                        };
                        self.ball.dx = dx;
                        self.ball.dy = dy;
                        dt_left *= 1. - cs;
                        if let &mut Some(ref mut callback_fn) = callback {
                            callback_fn();
                        }
                        break;
                    }
                }
            }
        }
        if !self.override_ball_sim {
            if hit_paddle_lhs || hit_paddle_rhs {
                self.ball.speedup(BALL_SPEEDUP);
            }
            self.ball.bound.x += self.ball.dx * dt_left;
            self.ball.bound.y += self.ball.dy * dt_left;
        }
        if hit_paddle_lhs || hit_paddle_rhs {
            self.beep_snd.play();
        } else if hit_any {
            self.tick_snd.play();
        }
        if self.ball.bound.x < 0. {
            self.rhs_score += 1;
            self.error_snd.play();
            self.start_game(true);
        } else if self.ball.bound.x > self.width {
            self.lhs_score += 1;
            self.error_snd.play();
            self.start_game(false);
        }
    }

    pub fn start_game(&mut self, lhs_start: bool) {
        self.lhs_paddle.bound.x = PADDLE_X_OFFSET - PADDLE_WIDTH / 2.;
        self.lhs_paddle.bound.y = BOARD_HEIGHT / 2. - PADDLE_HEIGHT / 2.;
        self.lhs_paddle.dy = 0.;
        self.lhs_paddle.ddy = 0.;
        self.rhs_paddle.bound.x = BOARD_WIDTH - PADDLE_X_OFFSET - PADDLE_WIDTH / 2.;
        self.rhs_paddle.bound.y = BOARD_HEIGHT / 2. - PADDLE_HEIGHT / 2.;
        self.rhs_paddle.dy = 0.;
        self.rhs_paddle.ddy = 0.;
        self.ball.bound.x = BOARD_WIDTH / 2. - BALL_RADIUS;
        self.ball.bound.y = BOARD_HEIGHT / 2. - BALL_RADIUS;
        self.ball.dx = if lhs_start { -BALL_START_SPEED } else { BALL_START_SPEED };
        self.ball.dy = 0.;
        self.ai_last_offset = 0.;
        self.ai_accum_offset = 0.;
    }

    pub fn handle_input(&mut self, key: glium::glutin::VirtualKeyCode, is_pressed: bool) {
        // player input
        match (key, is_pressed) {
            (glium::glutin::VirtualKeyCode::Up, true) => {
                self.lhs_paddle.ddy = PLAYER_PADDLE_ACCEL;
            },
            (glium::glutin::VirtualKeyCode::Up, false) => {
                if self.lhs_paddle.ddy > 0. {
                    self.lhs_paddle.ddy = 0.;
                }
            },
            (glium::glutin::VirtualKeyCode::Down, true) => {
                self.lhs_paddle.ddy = -PLAYER_PADDLE_ACCEL;
            },
            (glium::glutin::VirtualKeyCode::Down, false) => {
                if self.lhs_paddle.ddy < 0. {
                    self.lhs_paddle.ddy = 0.;
                }
            },
            (glium::glutin::VirtualKeyCode::B, true) => {
                self.override_ball_sim = !self.override_ball_sim;
            },
            (glium::glutin::VirtualKeyCode::W, true) => {
                if self.override_ball_sim {
                    self.ball.bound.y += 100.;
                }
            },
            (glium::glutin::VirtualKeyCode::S, true) => {
                if self.override_ball_sim {
                    self.ball.bound.y -= 100.;
                }
            },
            (glium::glutin::VirtualKeyCode::Return, true) => {
                self.start_game(true);
            },
            _ => ()
        }
    }
}
