extern crate glium;
extern crate ears;
use ears::{Sound, AudioController};

const BOARD_WIDTH: f32 = 600.;
const BOARD_HEIGHT: f32 = 300.;
const PADDLE_X_OFFSET: f32 = 10.;
const PADDLE_WIDTH: f32 = 10.;
const PADDLE_HEIGHT: f32 = 60.;
const GOAL_HEIGHT: f32 = 240.;
const BALL_RADIUS: f32 = 5.;
const BALL_MAX_SPEED: f32 = 800.;
const BALL_MAX_SLOPE: f32 = 1.;
const BALL_SPEEDUP: f32 = 1.05;
const BALL_START_SPEED: f32 = 300.;
const PADDLE_MAX_SPEED: f32 = 500.;
const PADDLE_FRICTION: f32 = 0.005;
const PADDLE_BALL_INFLUENCE: f32 = 0.3;
const PADDLE_CURVE: f32 = 0.5;
const PLAYER_PADDLE_ACCEL: f32 = 2000.;
const HIT_DELAY: f32 = 0.05;
const AI_PADDLE_P_FACTOR: f32 = 40.;
const AI_PADDLE_I_FACTOR: f32 = 0.1;
const AI_PADDLE_D_FACTOR: f32 = 2.;
const AI_PADDLE_MAX_ACCEL: f32 = 1800.;
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
    pub fn set_speed(&mut self, dx: f32, dy: f32) {
        let mut mag = (dx * dx + dy * dy).sqrt();
        let mut slope = dy / dx;
        if mag > BALL_MAX_SPEED {
            mag = BALL_MAX_SPEED;
        }
        if slope.abs() > BALL_MAX_SLOPE {
            slope = slope.signum() * BALL_MAX_SLOPE;
        }
        self.dx = dx.signum() * mag / (1. + slope * slope).sqrt();
        self.dy = dx.signum() * mag * slope / (1. + slope * slope).sqrt();
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
    delay: f32,
    override_ball_sim: bool,
    ai_last_offset: f32,
    ai_accum_offset: f32,
    beep_snd: Sound,
    tick_snd: Sound,
    error_snd: Sound
}

fn collides(sx: f32, sy: f32, sdx: f32, sdy: f32, tx: f32, ty: f32, tdx: f32, tdy: f32) -> (f32, f32) {
    let (stdx, stdy) = (sx - tx, sy - ty);
    let cn = (tdx * stdx + tdy * stdy) / (tdx * tdx + tdy * tdy);
    let (nx, ny) = (tx + cn * tdx - sx, ty + cn * tdy - sy);
    let cs = (nx * nx + ny * ny) / (nx * sdx + ny * sdy);
    let (px, py) = (sx + cs * sdx - tx, sy + cs * sdy - ty);
    let ct = (px * tdx + py * tdy) / (tdx * tdx + tdy * tdy);
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
            delay: 0.,
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

    pub fn update(&mut self, mut dt: f32) {
        if self.delay >= dt {
            self.delay -= dt;
            return;
        }
        dt -= self.delay;
        self.delay = 0.;
        // ai sim
        let target = if self.ball.dx > 0. {
            self.ball.bound.y + self.ball.bound.height / 2.
        } else {
            self.height / 2.
        };
        let target_offset = target - (self.rhs_paddle.bound.y + self.rhs_paddle.bound.height / 2.);
        if self.ai_accum_offset.signum() != target_offset.signum() {
            self.ai_accum_offset = 0.;
        } else {
            self.ai_accum_offset += target_offset;
        }
        let p = target_offset;
        let i = self.ai_accum_offset;
        let d = (p - self.ai_last_offset) / dt;
        self.ai_last_offset = target_offset;
        let ddy_diff = AI_PADDLE_P_FACTOR * p + AI_PADDLE_I_FACTOR * i + AI_PADDLE_D_FACTOR * d - self.rhs_paddle.ddy;
        if ddy_diff.abs() > AI_PADDLE_MAX_ACCEL {
            self.rhs_paddle.ddy += ddy_diff.signum() * AI_PADDLE_MAX_ACCEL;
        } else {
            self.rhs_paddle.ddy += ddy_diff;
        }
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
           let (dx, dy) = reflect(ball.dx.signum() * (ball.dx * ball.dx + ball.dy * ball.dy).sqrt(), paddle_dy * PADDLE_BALL_INFLUENCE, nx, ny);
           (BALL_SPEEDUP * dx, BALL_SPEEDUP * dy)
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
            let mut colliders: [(f32, f32, f32, f32, Normal, Option<&mut FnMut()>, bool); 8] = [
                (0., BALL_RADIUS, self.width, 0., Normal::Static(0., 1.), None, false),
                (0., self.height - BALL_RADIUS, self.width, 0., Normal::Static(0., -1.), None, false),
                (BALL_RADIUS, 0., 0., lhs_goal_border_height, Normal::Static(1., 0.), None, false),
                (BALL_RADIUS, self.height, 0., -lhs_goal_border_height, Normal::Static(1., 0.), None, false),
                (self.width - BALL_RADIUS, 0., 0., lhs_goal_border_height, Normal::Static(-1., 0.), None, false),
                (self.width - BALL_RADIUS, self.height, 0., -rhs_goal_border_height, Normal::Static(-1., 0.), None, false),
                (self.lhs_paddle.bound.x + self.lhs_paddle.bound.width + BALL_RADIUS, self.lhs_paddle.bound.y - BALL_RADIUS, 0., self.lhs_paddle.bound.height + 2. * BALL_RADIUS, 
                    Normal::Dynamic(1., 0., &lhs_reflect_fn), Some(&mut lhs_callback_fn), true),
                (self.rhs_paddle.bound.x - BALL_RADIUS, self.rhs_paddle.bound.y - BALL_RADIUS, 0., self.rhs_paddle.bound.height + 2. * BALL_RADIUS, 
                    Normal::Dynamic(-1., 0., &rhs_reflect_fn), Some(&mut rhs_callback_fn), true),
            ];
            let mut kill_early = false;
            while dt_left > 0. && has_collide && iterations < MAX_ITERATIONS && !kill_early {
                iterations += 1;
                has_collide = false;
                let iter_dx = self.ball.dx * dt_left;
                let iter_dy = self.ball.dy * dt_left;
                for &mut (tx, ty, tdx, tdy, ref normal, ref mut callback, early) in colliders.iter_mut() {
                    let (nx, ny) = match normal {
                        &Normal::Static(x, y) => (x, y),
                        &Normal::Dynamic(x, y, _) => (x, y)
                    };
                    if nx * self.ball.dx + ny * self.ball.dy >= 0. { continue; }
                    let (cs, ct) = collides(
                        self.ball.bound.x + BALL_RADIUS, self.ball.bound.y + BALL_RADIUS, iter_dx, iter_dy,
                        tx, ty, tdx, tdy);
                    if 0. < cs && cs <= 1. && 0. < ct && ct <= 1. {
                        self.ball.bound.x += iter_dx * cs;
                        self.ball.bound.y += iter_dy * cs;
                        has_collide = true;
                        hit_any = true;
                        if early { kill_early = true; }
                        let (dx, dy) = if let &Normal::Dynamic(_, _, reflect_fn) = normal {
                            reflect_fn(ct, &self.ball)
                        } else {
                            reflect(self.ball.dx, self.ball.dy, nx, ny)
                        };
                        self.ball.set_speed(dx, dy);
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
                self.delay = HIT_DELAY;
            }
            if dt_left < self.delay {
                self.delay = HIT_DELAY - dt_left;
            } else {
                self.ball.bound.x += self.ball.dx * (dt_left - self.delay);
                self.ball.bound.y += self.ball.dy * (dt_left - self.delay);
                self.delay = 0.;
            }
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
        self.delay = 0.;
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
