extern crate glium;

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
const PADDLE_CURVE: f32 = 0.1;
const PLAYER_PADDLE_ACCEL: f32 = 2000.;
const AI_PADDLE_ACCEL_FACTOR: f32 = 30.;

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
    pub lhs_paddle: Paddle,
    pub rhs_paddle: Paddle,
    pub lhs_goal_height: f32,
    pub rhs_goal_height: f32,
    pub ball: Ball,
}

fn collides(sx: f32, sy: f32, sdx: f32, sdy: f32, tx: f32, ty: f32, tdx: f32, tdy: f32) -> (f32, f32) {
    let (stdx, stdy) = (sx - tx, sy - ty);
    let ct = (tdx * stdx + tdy * stdy) / (tdx * tdx + tdy * tdy);
    let nx = (tx + ct * tdx) - sx;
    let ny = (ty + ct * tdy) - sy;
    let cs = (nx * nx + ny * ny) /  (nx * sdx + ny * sdy);
    (ct, cs)
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
            }
        }
    }

    pub fn winner(&self) -> Option<bool> {
        if self.lhs_score == WIN_SCORE { Some(true) }
        else if self.rhs_score == WIN_SCORE { Some(false) }
        else { None }
    }

    pub fn update(&mut self, dt: f32) {
        // ai sim
        self.rhs_paddle.ddy = AI_PADDLE_ACCEL_FACTOR * ((self.ball.bound.y + self.ball.bound.height / 2.) - (self.rhs_paddle.bound.y + self.rhs_paddle.bound.height / 2.));
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
            Dynamic(f32, f32, &'a Fn(f32) -> (f32, f32))
        }
        fn paddle_sway_normal(is_left: bool, pdy: f32, ct: f32) -> (f32, f32) {
           let dx = if is_left {1.} else {-1.};
           let dy = ((2. * ct - 1.) + pdy / PADDLE_MAX_SPEED) * PADDLE_CURVE;
           let mag = (dx * dx + dy * dy).sqrt();
           (dx / mag, dy / mag)
        }
        let lhs_pdy = self.lhs_paddle.dy;
        let rhs_pdy = self.rhs_paddle.dy;
        let lhs_normal_fn = |ct: f32| { paddle_sway_normal(true, lhs_pdy, ct) };
        let rhs_normal_fn = |ct: f32| { paddle_sway_normal(true, rhs_pdy, ct) };
        let mut hit_lhs = false;
        let mut hit_rhs = false;
        {
            let mut lhs_callback_fn = || { hit_lhs = true; };
            let mut rhs_callback_fn = || { hit_rhs = true; };
            let mut basic_colliders: [(f32, f32, f32, f32, Normal, Option<&mut FnMut()>); 8] = [
                (0., BALL_RADIUS, self.width, 0., Normal::Static(0., 1.), None),
                (0., self.height - BALL_RADIUS, self.width, 0., Normal::Static(0., -1.), None),
                (BALL_RADIUS, 0., 0., lhs_goal_border_height, Normal::Static(1., 0.), None),
                (BALL_RADIUS, self.height, 0., -lhs_goal_border_height, Normal::Static(1., 0.), None),
                (self.width - BALL_RADIUS, 0., 0., lhs_goal_border_height, Normal::Static(-1., 0.), None),
                (self.width - BALL_RADIUS, self.height, 0., -rhs_goal_border_height, Normal::Static(-1., 0.), None),
                (self.lhs_paddle.bound.x + self.lhs_paddle.bound.width + BALL_RADIUS, self.lhs_paddle.bound.y, 0., self.lhs_paddle.bound.height, 
                    Normal::Dynamic(1., 0., &lhs_normal_fn), Some(&mut lhs_callback_fn)),
                (self.rhs_paddle.bound.x - BALL_RADIUS, self.rhs_paddle.bound.y, 0., self.rhs_paddle.bound.height, 
                    Normal::Dynamic(-1., 0., &rhs_normal_fn), Some(&mut rhs_callback_fn)),
            ];
            while dt_left > 0. && has_collide && iterations < MAX_ITERATIONS {
                iterations += 1;
                has_collide = false;
                let iter_dx = self.ball.dx * dt_left;
                let iter_dy = self.ball.dy * dt_left;
                for &mut (tx, ty, tdx, tdy, ref normal, ref mut callback) in basic_colliders.iter_mut() {
                    let (mut nx, mut ny) = match normal {
                        &Normal::Static(x, y) => (x, y),
                        &Normal::Dynamic(x, y, _) => (x, y)
                    };
                    if nx * self.ball.dx + ny * self.ball.dy >= 0. { continue; }
                    let (cs, ct) = collides(self.ball.bound.x + BALL_RADIUS, self.ball.bound.y + BALL_RADIUS, iter_dx, iter_dy, tx, ty, tdx, tdy);
                    if 0. < cs && cs <= 1. && 0. < ct && ct <= 1. {
                        if let &Normal::Dynamic(_, _, f) = normal {
                            let (nnx, nny) = f(ct);
                            nx = nnx;
                            ny = nny;
                        }
                        has_collide = true;
                        self.ball.bound.x += iter_dx * (cs - 0.001);
                        self.ball.bound.y += iter_dy * (cs - 0.001);
                        let dot = -2. * (nx * self.ball.dx + ny * self.ball.dy);
                        self.ball.dx += dot * nx;
                        self.ball.dy += dot * ny;
                        dt_left *= 1. - cs;
                        if let &mut Some(ref mut f) = callback {
                            f();
                        }
                        break;
                    }
                }
            }
        }
        if hit_lhs || hit_rhs {
            self.ball.speedup(BALL_SPEEDUP);
        }
        self.ball.bound.x += self.ball.dx * dt_left;
        self.ball.bound.y += self.ball.dy * dt_left;
        if self.ball.bound.x < 0. {
            self.rhs_score += 1;
            self.start_game(true);
        } else if self.ball.bound.x > self.width {
            self.lhs_score += 1;
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
        self.ball.dy = 0.;
        self.ball.dx = if lhs_start { -BALL_START_SPEED } else { BALL_START_SPEED };
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
            _ => ()
        }
    }
}
