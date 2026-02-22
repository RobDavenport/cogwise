use cogwise::utility::ResponseCurve;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
struct Agent {
    x: i32,
    y: i32,
    preset: u8,
    stamina: f32,
}

#[wasm_bindgen]
pub struct Simulation {
    tick: u64,
    waypoint_x: i32,
    waypoint_y: i32,
    agents: Vec<Agent>,
    signal_curve: ResponseCurve<f32>,
}

#[wasm_bindgen]
impl Simulation {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            tick: 0,
            waypoint_x: 10,
            waypoint_y: 10,
            agents: vec![
                Agent {
                    x: 2,
                    y: 2,
                    preset: 0,
                    stamina: 1.0,
                },
                Agent {
                    x: 17,
                    y: 16,
                    preset: 1,
                    stamina: 1.0,
                },
            ],
            signal_curve: ResponseCurve::Logistic {
                midpoint: 0.5,
                steepness: 10.0,
            },
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.saturating_add(1);
        let signal = self.signal();
        for agent in &mut self.agents {
            update_agent(agent, self.waypoint_x, self.waypoint_y, signal, self.tick);
        }
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("TICK:{}\n", self.tick));
        out.push_str(&format!("WAYPOINT:{},{}\n", self.waypoint_x, self.waypoint_y));
        out.push_str(&format!("SIGNAL:{:.4}\n", self.signal()));
        out.push_str(&format!("AGENTS:{}\n", self.agents.len()));
        for agent in &self.agents {
            out.push_str(&format!(
                "A:{},{},{},{:.3}\n",
                agent.x, agent.y, agent.preset, agent.stamina
            ));
        }
        out
    }

    pub fn add_agent(&mut self, x: i32, y: i32, preset: u8) {
        self.agents.push(Agent {
            x: x.clamp(0, 19),
            y: y.clamp(0, 19),
            preset,
            stamina: 1.0,
        });
    }

    pub fn set_waypoint(&mut self, x: i32, y: i32) {
        self.waypoint_x = x.clamp(0, 19);
        self.waypoint_y = y.clamp(0, 19);
    }

    pub fn tick_count(&self) -> u64 {
        self.tick
    }

    fn signal(&self) -> f32 {
        let phase = (self.tick % 240) as f32 / 240.0;
        self.signal_curve.evaluate(phase)
    }
}

fn update_agent(agent: &mut Agent, waypoint_x: i32, waypoint_y: i32, signal: f32, tick: u64) {
    let mut step = if signal > 0.45 { 1 } else { 0 };
    if agent.stamina < 0.2 {
        step = 0;
    }

    match agent.preset % 4 {
        0 => {
            let dx = (waypoint_x - agent.x).signum();
            let dy = (waypoint_y - agent.y).signum();
            agent.x += dx * step;
            agent.y += dy * step;
        }
        1 => {
            let target_x = if tick % 120 < 60 { 2 } else { 17 };
            let target_y = if tick % 120 < 60 { 16 } else { 2 };
            agent.x += (target_x - agent.x).signum() * step;
            agent.y += (target_y - agent.y).signum() * step;
        }
        2 => {
            let wave = ((tick / 10) % 4) as i32;
            let (dx, dy) = match wave {
                0 => (1, 0),
                1 => (0, 1),
                2 => (-1, 0),
                _ => (0, -1),
            };
            agent.x += dx * step;
            agent.y += dy * step;
        }
        _ => {
            let orbit_x = 10 + (((tick / 8) % 6) as i32 - 3);
            let orbit_y = 10 + (((tick / 6) % 6) as i32 - 3);
            agent.x += (orbit_x - agent.x).signum() * step;
            agent.y += (orbit_y - agent.y).signum() * step;
        }
    }

    agent.x = agent.x.clamp(0, 19);
    agent.y = agent.y.clamp(0, 19);

    if step == 0 {
        agent.stamina = (agent.stamina + 0.02).min(1.0);
    } else {
        agent.stamina = (agent.stamina - 0.03).max(0.0);
    }
}
