use std::collections::VecDeque;

use crate::game::DrawMode;

#[derive(Debug, Clone, Copy)]
pub struct RobotSolverAnchor {
    state_hash: u64,
    seed: u64,
    draw_mode: DrawMode,
    move_count: u32,
}

#[derive(Debug, Clone)]
pub struct RobotPlayback<M> {
    anchor: Option<RobotSolverAnchor>,
    scripted_line: VecDeque<M>,
    use_scripted_line: bool,
}

impl<M> Default for RobotPlayback<M> {
    fn default() -> Self {
        Self {
            anchor: None,
            scripted_line: VecDeque::new(),
            use_scripted_line: false,
        }
    }
}

impl<M> RobotPlayback<M> {
    pub fn arm(
        &mut self,
        seed: u64,
        draw_mode: DrawMode,
        move_count: u32,
        state_hash: u64,
        line: Vec<M>,
    ) {
        self.anchor = Some(RobotSolverAnchor {
            state_hash,
            seed,
            draw_mode,
            move_count,
        });
        self.scripted_line = VecDeque::from(line);
        self.use_scripted_line = false;
    }

    pub fn clear(&mut self) {
        self.anchor = None;
        self.scripted_line.clear();
        self.use_scripted_line = false;
    }

    pub fn matches_current(
        &self,
        seed: u64,
        draw_mode: DrawMode,
        move_count: u32,
        state_hash: u64,
    ) -> bool {
        let Some(anchor) = self.anchor else {
            return false;
        };
        anchor.seed == seed
            && anchor.draw_mode == draw_mode
            && anchor.move_count == move_count
            && anchor.state_hash == state_hash
    }

    pub fn has_scripted_line(&self) -> bool {
        !self.scripted_line.is_empty()
    }

    pub fn set_use_scripted_line(&mut self, enabled: bool) {
        self.use_scripted_line = enabled;
    }

    pub fn use_scripted_line(&self) -> bool {
        self.use_scripted_line
    }

    pub fn pop_scripted_move(&mut self) -> Option<M> {
        self.scripted_line.pop_front()
    }

    pub fn clear_scripted_line(&mut self) {
        self.scripted_line.clear();
        self.use_scripted_line = false;
    }
}
