use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

use glam::{Vec2, Vec3};

use super::doodle::Path;
use super::{ARRIVED, Activity, EventKind, Nature, ROOM_MAX, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Cross,
    Nought,
}

impl Mark {
    fn other(self) -> Self {
        match self {
            Self::Cross => Self::Nought,
            Self::Nought => Self::Cross,
        }
    }

    fn draw_time(self) -> f64 {
        match self {
            Self::Cross => 0.5,
            Self::Nought => 1.0,
        }
    }

    fn path(self) -> &'static Path {
        &paths()[match self {
            Self::Cross => 1,
            Self::Nought => 2,
        }]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    GhostWon { line: [usize; 3] },
    ViewerWon { line: [usize; 3] },
    Draw,
    Abandoned,
}

impl Outcome {
    pub fn finished(self) -> bool {
        self != Self::Abandoned
    }
}

/// A game in the fog, kept only until its strokes fade. Cells read from
/// left to right, then top to bottom, as the viewer sees the glass.
#[derive(Clone, Debug, PartialEq)]
pub struct Game {
    pub center: Vec2,
    pub cell_size: f32,
    pub cells: [Option<Mark>; 9],
    pub drawn_at: [Option<f64>; 9],
    pub turn: Mark,
    pub first: Mark,
    pub grid_started: Option<f64>,
    pub turn_started: f64,
    pub outcome: Option<Outcome>,
    pub ended: Option<f64>,
    drawing: Option<usize>,
    chosen: Option<usize>,
    think_until: Option<f64>,
}

impl Game {
    pub const GRID_TIME: f64 = 2.5;
    pub const LINE_TIME: f64 = 0.6;
    pub const VIEWER_TIMEOUT: f64 = 45.0;
    pub const LINES: [[usize; 3]; 8] = [
        [0, 1, 2],
        [3, 4, 5],
        [6, 7, 8],
        [0, 3, 6],
        [1, 4, 7],
        [2, 5, 8],
        [0, 4, 8],
        [2, 4, 6],
    ];

    pub fn new(center: Vec2, first: Mark, time: f64) -> Self {
        Self {
            center,
            cell_size: 0.3,
            cells: [None; 9],
            drawn_at: [None; 9],
            turn: first,
            first,
            grid_started: None,
            turn_started: time,
            outcome: None,
            ended: None,
            drawing: None,
            chosen: None,
            think_until: None,
        }
    }

    pub fn winner(cells: &[Option<Mark>; 9]) -> Option<(Mark, [usize; 3])> {
        Self::LINES.into_iter().find_map(|line| {
            let mark = cells[line[0]]?;
            line.iter()
                .all(|&cell| cells[cell] == Some(mark))
                .then_some((mark, line))
        })
    }

    pub fn free_cells(cells: &[Option<Mark>; 9]) -> Vec<usize> {
        cells
            .iter()
            .enumerate()
            .filter_map(|(index, cell)| cell.is_none().then_some(index))
            .collect()
    }

    pub fn cell_at(&self, point: Vec2) -> Option<usize> {
        let offset = (point - self.center) / self.cell_size;
        if !offset.is_finite() || offset.abs().max_element() >= 1.5 {
            return None;
        }
        let column = (offset.x + 1.5).floor() as usize;
        let row = (1.5 - offset.y).floor() as usize;
        Some(row * 3 + column)
    }

    pub fn cell_center(&self, index: usize) -> Vec2 {
        self.center + Vec2::new((index % 3) as f32 - 1.0, 1.0 - (index / 3) as f32) * self.cell_size
    }

    pub fn grid_ready(&self, time: f64) -> bool {
        self.grid_started
            .is_some_and(|started| time - started >= Self::GRID_TIME)
    }

    pub fn could_win(&self, mark: Mark) -> bool {
        if Self::winner(&self.cells).is_some() {
            return false;
        }
        Self::free_cells(&self.cells).into_iter().any(|cell| {
            let mut cells = self.cells;
            cells[cell] = Some(mark);
            Self::winner(&cells).is_some_and(|(winner, _)| winner == mark)
        })
    }

    pub fn opacity(&self, time: f64) -> f32 {
        self.ended.map_or(1.0, |ended| {
            (1.0 - (time - ended) / 40.0).clamp(0.0, 1.0) as f32
        })
    }

    pub fn distance(&self, point: Vec2, time: f64) -> f32 {
        // An interrupted pen stops where it was; only the winning line
        // continues drawing after a result.
        let drawing_time = time.min(self.ended.unwrap_or(time));
        let offset = (point - self.center) / self.cell_size;
        let progress = self.grid_started.map_or(0.0, |started| {
            ((drawing_time - started) / Self::GRID_TIME) as f32
        });
        let mut nearest = paths()[0].distance(offset, progress) * self.cell_size;
        for (cell, mark) in self.cells.iter().enumerate() {
            if let (Some(mark), Some(started)) = (mark, self.drawn_at[cell]) {
                let progress = ((drawing_time - started) / mark.draw_time()) as f32;
                nearest = nearest.min(
                    mark.path()
                        .distance((point - self.cell_center(cell)) / self.cell_size, progress)
                        * self.cell_size,
                );
            }
        }
        nearest.min(self.winning_distance(point, time))
    }

    pub fn winning_distance(&self, point: Vec2, time: f64) -> f32 {
        let line = match self.outcome {
            Some(Outcome::GhostWon { line } | Outcome::ViewerWon { line }) => line,
            _ => return f32::INFINITY,
        };
        let progress = ((time - self.ended.unwrap()) / Self::LINE_TIME).clamp(0.0, 1.0) as f32;
        if progress <= 0.0 {
            return f32::INFINITY;
        }
        let first = self.cell_center(line[0]);
        let last = self.cell_center(line[2]);
        let extension = (last - first).normalize() * self.cell_size * 0.35;
        crate::sdf::capsule(
            point,
            first - extension,
            (first - extension).lerp(last + extension, progress),
            0.0,
        )
    }

    fn grid_pen(&self, time: f64) -> Vec2 {
        let progress = self
            .grid_started
            .map_or(0.0, |started| ((time - started) / Self::GRID_TIME) as f32);
        self.center + paths()[0].point_at(progress) * self.cell_size
    }

    fn place(&mut self, cell: usize, mark: Mark, time: f64) -> bool {
        if self.outcome.is_some()
            || self.turn != mark
            || self.drawing.is_some()
            || !self.grid_ready(time)
            || self.cells.get(cell) != Some(&None)
        {
            return false;
        }
        self.cells[cell] = Some(mark);
        self.drawn_at[cell] = Some(time);
        self.drawing = Some(cell);
        true
    }

    fn finish(&mut self, outcome: Outcome, time: f64) {
        self.outcome = Some(outcome);
        self.ended = Some(time);
    }
}

// All strokes are measured once; ray casting only measures the drawn part.
fn paths() -> &'static [Path; 3] {
    static PATHS: OnceLock<[Path; 3]> = OnceLock::new();
    PATHS.get_or_init(|| {
        [
            Path::new(vec![
                vec![Vec2::new(-0.5, 1.5), Vec2::new(-0.5, -1.5)],
                vec![Vec2::new(0.5, 1.5), Vec2::new(0.5, -1.5)],
                vec![Vec2::new(-1.5, 0.5), Vec2::new(1.5, 0.5)],
                vec![Vec2::new(-1.5, -0.5), Vec2::new(1.5, -0.5)],
            ]),
            Path::new(vec![
                vec![Vec2::new(-0.32, 0.32), Vec2::new(0.32, -0.32)],
                vec![Vec2::new(0.32, 0.32), Vec2::new(-0.32, -0.32)],
            ]),
            Path::new(vec![
                (0..=32)
                    .map(|index| Vec2::from_angle(PI * 0.5 + TAU * index as f32 / 32.0) * 0.34)
                    .collect(),
            ]),
        ]
    })
}

/// Depth favors an early win and puts off a loss. Equal scores are all
/// kept, so the world's random source can choose among equally good moves.
pub fn best_cells(cells: &[Option<Mark>; 9], turn: Mark) -> Vec<usize> {
    if Game::winner(cells).is_some() {
        return Vec::new();
    }
    let mut best = Vec::new();
    let mut highest = i32::MIN;
    let mut board = *cells;
    for cell in Game::free_cells(cells) {
        board[cell] = Some(turn);
        let score = -search(&mut board, turn.other(), 1);
        board[cell] = None;
        if score > highest {
            best.clear();
            highest = score;
        }
        if score == highest {
            best.push(cell);
        }
    }
    best
}

fn search(cells: &mut [Option<Mark>; 9], turn: Mark, depth: i32) -> i32 {
    if let Some((winner, _)) = Game::winner(cells) {
        return if winner == turn {
            10 - depth
        } else {
            depth - 10
        };
    }
    let mut best = None;
    for cell in 0..9 {
        if cells[cell].is_some() {
            continue;
        }
        cells[cell] = Some(turn);
        let score = -search(cells, turn.other(), depth + 1);
        cells[cell] = None;
        best = Some(best.map_or(score, |previous: i32| previous.max(score)));
    }
    best.unwrap_or(0)
}

pub fn mistake_chance(nature: Nature, energy: f32, bond: f32, level: f32) -> f32 {
    let gentle = if bond >= 0.55 {
        0.25 * nature.tender * bond
    } else {
        0.0
    };
    let chance =
        (0.22 - 0.12 * nature.curious + 0.10 * nature.playful + 0.20 * (1.0 - energy) + gentle)
            .clamp(0.05, 0.6);
    (chance - 0.1 * level).clamp(0.05, 0.6)
}

impl World {
    /// Invites an awake ghost to a game; an invitation gives the viewer
    /// the opening move, even if it is too tired to suggest a game itself.
    pub fn challenge(&mut self) -> bool {
        if self.ghost.asleep >= 0.3 {
            return false;
        }
        self.notice_viewer();
        self.begin(Activity::Game);
        let game = self.game.as_mut().unwrap();
        game.turn = Mark::Cross;
        game.first = Mark::Cross;
        self.shown(Activity::Game);
        true
    }

    pub fn mark(&mut self, cell: usize) -> bool {
        if self.ghost.plan.activity != Activity::Game || self.ghost.plan.done {
            return false;
        }
        let placed = self
            .game
            .as_mut()
            .is_some_and(|game| game.place(cell, Mark::Cross, self.time));
        if placed {
            self.notice_viewer();
        }
        placed
    }

    pub fn good_sport(&self) -> bool {
        self.life.nature.unwrap_or_default().tender > 0.4 || self.life.bond >= 0.55
    }

    pub(super) fn abandon_game(&mut self) {
        if let Some(game) = &mut self.game
            && game.outcome.is_none()
        {
            game.finish(Outcome::Abandoned, self.time);
        }
    }

    pub(super) fn begin_game(&mut self) -> Vec3 {
        let center = Vec2::new(self.ghost.position.x.clamp(-1.0, 1.0), 1.25);
        let game = Game::new(center, Mark::Nought, self.time);
        // It wipes the glass first: every picture whose strokes reach the
        // board.
        let board = Vec2::splat(1.5 * game.cell_size);
        self.doodles.retain(|doodle| {
            let reach = doodle.picture.extent() * doodle.size;
            ((doodle.center - game.center).abs() - reach - board).max_element() >= 0.0
        });
        let target = inside(game.grid_pen(self.time), 0.3);
        self.game = Some(game);
        target
    }

    pub(super) fn play_game(&mut self) -> Option<Vec3> {
        let mut game = self.game.take()?;
        let time = self.time;
        let viewer = self
            .viewer
            .pointer
            .unwrap_or(game.center.extend(ROOM_MAX.z))
            + Vec3::Z * 3.0;
        let mut look = Some(viewer);
        if let Some(ended) = game.ended {
            self.ghost.plan.target = inside(game.center, 0.7);
            self.ghost.plan.done = time - ended > 4.0;
        } else if let Some(arrived) = self.ghost.plan.arrived {
            game.grid_started.get_or_insert(arrived);
            if !game.grid_ready(time) {
                let pen = game.grid_pen(time);
                self.ghost.plan.target = inside(pen, 0.3);
                look = Some(pen.extend(ROOM_MAX.z));
                game.turn_started = time;
            } else if let Some(cell) = game.drawing {
                let mark = game.cells[cell].unwrap();
                let progress = ((time - game.drawn_at[cell].unwrap()) / mark.draw_time()) as f32;
                if mark == Mark::Nought {
                    let pen =
                        game.cell_center(cell) + mark.path().point_at(progress) * game.cell_size;
                    self.ghost.plan.target = inside(pen, 0.3);
                    look = Some(pen.extend(ROOM_MAX.z));
                }
                if progress >= 1.0 {
                    game.drawing = None;
                    let outcome = if let Some((winner, line)) = Game::winner(&game.cells) {
                        Some(match winner {
                            Mark::Nought => Outcome::GhostWon { line },
                            Mark::Cross => Outcome::ViewerWon { line },
                        })
                    } else if Game::free_cells(&game.cells).is_empty() {
                        Some(Outcome::Draw)
                    } else {
                        None
                    };
                    if let Some(outcome) = outcome {
                        self.end_game(&mut game, outcome);
                    } else {
                        game.turn = mark.other();
                        game.turn_started = time;
                        game.chosen = None;
                        game.think_until = None;
                    }
                }
            } else if game.turn == Mark::Cross {
                self.ghost.plan.target = inside(game.center, 0.7);
                if time - game.turn_started >= Game::VIEWER_TIMEOUT {
                    self.end_game(&mut game, Outcome::Abandoned);
                }
            } else {
                let think_until = *game
                    .think_until
                    .get_or_insert_with(|| time + f64::from(self.random.range(1.0, 2.5)));
                let free = Game::free_cells(&game.cells);
                let glance = ((time - game.turn_started) / 0.4) as usize % free.len();
                look = Some(game.cell_center(free[glance]).extend(ROOM_MAX.z));
                if time >= think_until {
                    let cell = *game.chosen.get_or_insert_with(|| {
                        let best = best_cells(&game.cells, Mark::Nought);
                        let other: Vec<usize> = free
                            .iter()
                            .copied()
                            .filter(|cell| !best.contains(cell))
                            .collect();
                        let choices = if !other.is_empty()
                            && self.random.unit()
                                < mistake_chance(
                                    self.life.nature.unwrap_or_default(),
                                    self.ghost.needs.energy,
                                    self.life.bond,
                                    self.life.skills.level(super::Skill::Games),
                                ) {
                            &other
                        } else {
                            &best
                        };
                        choices[self.random.range(0.0, choices.len() as f32) as usize]
                    });
                    let target = inside(game.cell_center(cell), 0.3);
                    self.ghost.plan.target = target;
                    look = Some(game.cell_center(cell).extend(ROOM_MAX.z));
                    if self.ghost.position.distance(target) < ARRIVED {
                        game.place(cell, Mark::Nought, time);
                    }
                }
                // A blocked approach still ends, even after the generic
                // plan timeout has been lifted for the viewer's turns.
                if time - game.turn_started > 15.0 {
                    game.finish(Outcome::Abandoned, time);
                    self.ghost.plan.done = true;
                }
            }
        } else {
            look = Some(self.ghost.plan.target + Vec3::Z * 0.3);
        }
        self.game = Some(game);
        look
    }

    fn end_game(&mut self, game: &mut Game, outcome: Outcome) {
        game.finish(outcome, self.time);
        self.record(EventKind::GameOver(outcome));
        let lines: &[(&str, &str)] = match outcome {
            Outcome::GhostWon { .. } if self.life.nature.unwrap_or_default().playful > 0.6 => {
                &[("Got you!", "잡았다!"), ("I win!", "내가 이겼다!")]
            }
            Outcome::GhostWon { .. } => {
                &[("I win!", "내가 이겼다!"), ("That was fun.", "재밌었어.")]
            }
            Outcome::ViewerWon { .. } if !self.good_sport() => &[
                ("Hmph. Again?", "흥. 한 번 더?"),
                ("You win. Rematch?", "네가 이겼어. 또 할래?"),
            ],
            Outcome::ViewerWon { .. } => {
                &[("You win!", "네가 이겼어!"), ("Well played!", "잘했어!")]
            }
            Outcome::Draw => &[("A draw!", "비겼다!"), ("Nobody wins.", "우리 비겼네.")],
            Outcome::Abandoned => &[("Maybe later.", "다음에 하자.")],
        };
        let line = lines[self.random.range(0.0, lines.len() as f32) as usize];
        self.say(self.spoken(line.0, line.1));
        if outcome.finished() {
            self.ghost.needs.change(0.0, 0.15, 0.08);
        }
    }
}

fn inside(point: Vec2, setback: f32) -> Vec3 {
    point.extend(ROOM_MAX.z - setback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_results_are_spoken_in_the_viewers_language() {
        for korean in [false, true] {
            for outcome in [
                Outcome::GhostWon { line: [0, 1, 2] },
                Outcome::ViewerWon { line: [0, 1, 2] },
                Outcome::Draw,
                Outcome::Abandoned,
            ] {
                let mut world = World::new(1, 12.0);
                world.life.korean = if korean { 1.0 } else { 0.0 };
                let mut game = Game::new(Vec2::ZERO, Mark::Cross, world.time);
                world.end_game(&mut game, outcome);
                let spoken = &world.ghost.speech.as_ref().unwrap().text;
                assert_eq!(
                    spoken
                        .chars()
                        .any(|character| ('가'..='힣').contains(&character)),
                    korean
                );
            }
        }
    }
    use crate::world::{Doodle, Mood, Picture, Sketch};

    fn run_until(world: &mut World, condition: impl Fn(&World) -> bool) {
        for _ in 0..1400 {
            if condition(world) {
                return;
            }
            world.step(0.05);
        }
        panic!("game did not reach its next turn: {:?}", world.game);
    }

    fn viewer_can_move(world: &World) -> bool {
        world.game.as_ref().is_some_and(|game| {
            game.grid_ready(world.time)
                && game.turn == Mark::Cross
                && game.drawing.is_none()
                && game.outcome.is_none()
        })
    }

    #[test]
    fn practice_reduces_mistakes_while_preserving_gentleness_and_the_limits() {
        let nature = Nature::default();
        let beginner = mistake_chance(nature, 1.0, 0.0, 0.0);
        let practiced = mistake_chance(nature, 1.0, 0.0, 1.0);
        assert!((beginner - practiced - 0.1).abs() < 1e-6);
        assert!(mistake_chance(nature, 1.0, 1.0, 1.0) > practiced);
        let gentle = Nature {
            curious: 0.0,
            playful: 1.0,
            tender: 1.0,
            ..nature
        };
        assert_eq!(mistake_chance(gentle, 0.0, 1.0, 1.0), 0.5);
        for level in [0.0, 0.5, 1.0] {
            for energy in [0.0, 0.5, 1.0] {
                for bond in [0.0, 0.55, 1.0] {
                    assert!((0.05..=0.6).contains(&mistake_chance(nature, energy, bond, level)));
                }
            }
        }
    }

    #[test]
    fn every_line_has_a_winner_and_empty_or_incomplete_lines_do_not() {
        assert_eq!(Game::winner(&[None; 9]), None);
        for mark in [Mark::Cross, Mark::Nought] {
            for line in Game::LINES {
                let mut cells = [None; 9];
                cells[line[0]] = Some(mark);
                cells[line[1]] = Some(mark);
                assert_eq!(Game::winner(&cells), None);
                cells[line[2]] = Some(mark);
                assert_eq!(Game::winner(&cells), Some((mark, line)));
            }
        }
    }

    #[test]
    fn glass_points_map_to_cells_in_reading_order_and_reject_the_outside() {
        let game = Game::new(Vec2::new(0.4, 1.25), Mark::Cross, 0.0);
        for cell in 0..9 {
            assert_eq!(game.cell_at(game.cell_center(cell)), Some(cell));
        }
        for offset in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
            assert_eq!(game.cell_at(game.center + offset * 0.46), None);
        }
        assert_eq!(game.cell_at(Vec2::splat(f32::NAN)), None);
        assert_eq!(Game::free_cells(&[None; 9]), (0..9).collect::<Vec<_>>());
    }

    #[test]
    fn minimax_takes_a_win_and_blocks_an_immediate_loss() {
        let mut cells = [None; 9];
        cells[0] = Some(Mark::Nought);
        cells[1] = Some(Mark::Nought);
        cells[3] = Some(Mark::Cross);
        cells[4] = Some(Mark::Cross);
        assert_eq!(best_cells(&cells, Mark::Nought), [2]);
        cells[1] = None;
        assert_eq!(best_cells(&cells, Mark::Nought), [5]);
        cells[5] = Some(Mark::Cross);
        assert!(best_cells(&cells, Mark::Nought).is_empty());
    }

    #[test]
    fn perfect_play_against_perfect_play_always_draws() {
        fn play(cells: [Option<Mark>; 9], turn: Mark) {
            assert_eq!(Game::winner(&cells), None);
            for cell in best_cells(&cells, turn) {
                let mut next = cells;
                next[cell] = Some(turn);
                play(next, turn.other());
            }
        }
        play([None; 9], Mark::Cross);
        play([None; 9], Mark::Nought);
    }

    #[test]
    fn mistakes_follow_curiosity_playfulness_tiredness_and_a_close_tender_bond() {
        let nature = Nature::default();
        let usual = mistake_chance(nature, 1.0, 0.0, 0.0);
        assert!((usual - 0.21).abs() < 1e-5);
        assert!(
            mistake_chance(
                Nature {
                    curious: 1.0,
                    ..nature
                },
                1.0,
                0.0,
                0.0
            ) < usual
        );
        assert!(
            mistake_chance(
                Nature {
                    playful: 1.0,
                    ..nature
                },
                1.0,
                0.0,
                0.0
            ) > usual
        );
        assert!(mistake_chance(nature, 0.0, 0.0, 0.0) > usual);
        assert_eq!(mistake_chance(nature, 1.0, 0.54, 0.0), usual);
        assert!(mistake_chance(nature, 1.0, 0.55, 0.0) > usual);
        let independent = Nature {
            tender: 0.0,
            ..nature
        };
        assert_eq!(mistake_chance(independent, 1.0, 1.0, 0.0), usual);
        for curious in [0.0, 0.5, 1.0] {
            for playful in [0.0, 0.5, 1.0] {
                for tender in [0.0, 0.5, 1.0] {
                    for energy in [0.0, 0.5, 1.0] {
                        for bond in [0.0, 0.54, 0.55, 1.0] {
                            let chance = mistake_chance(
                                Nature {
                                    curious,
                                    playful,
                                    tender,
                                    ..nature
                                },
                                energy,
                                bond,
                                0.0,
                            );
                            assert!((0.05..=0.6).contains(&chance));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_challenge_leaves_sleepers_alone_and_gives_an_awake_viewer_the_opening() {
        let mut world = World::new(3, 12.0);
        assert!(!world.options().contains(&Activity::Game));
        world.ghost.asleep = 0.3;
        let plan = world.ghost.plan.clone();
        assert!(!world.challenge());
        assert_eq!(world.ghost.plan, plan);
        assert!(world.game.is_none());
        assert!(!world.viewer_present());
        world.ghost.asleep = 0.29;
        assert!(world.challenge());
        assert!(world.viewer_present());
        assert_eq!(world.game.as_ref().unwrap().first, Mark::Cross);
        assert!(!world.mark(0));
        run_until(&mut world, viewer_can_move);
        assert!(world.mark(0));
        assert!(!world.mark(1));
        world.step(0.6);
        assert!(!world.mark(1));
        assert!(!world.mark(9));
        world.ghost.needs.energy = 0.25;
        assert!(!world.options().contains(&Activity::Game));
        world.ghost.needs.energy = 0.8;
        assert!(world.options().contains(&Activity::Game));
        world.viewer.active_at = 0.0;
        assert!(!world.options().contains(&Activity::Game));
    }

    #[test]
    fn a_whole_game_counts_its_result_and_remembers_its_firsts_once() {
        let mut world = World::new(9, 12.0);
        world.begin(Activity::Game);
        assert_eq!(world.game.as_ref().unwrap().first, Mark::Nought);
        run_until(&mut world, viewer_can_move);
        assert_eq!(
            world.game.as_ref().unwrap().cells.iter().flatten().count(),
            1
        );
        let occupied = world
            .game
            .as_ref()
            .unwrap()
            .cells
            .iter()
            .position(Option::is_some)
            .unwrap();
        assert!(!world.mark(occupied));
        assert!(!world.mark(9));
        loop {
            let game = world.game.as_ref().unwrap();
            let cell = best_cells(&game.cells, Mark::Cross)[0];
            assert!(world.mark(cell));
            assert!(!world.mark(cell));
            run_until(&mut world, |world| {
                viewer_can_move(world) || world.game.as_ref().unwrap().outcome.is_some()
            });
            if world.game.as_ref().unwrap().outcome.is_some() {
                break;
            }
        }
        let outcome = world.game.as_ref().unwrap().outcome.unwrap();
        assert!(outcome.finished());
        assert_eq!(world.life.total.games, 1);
        assert_eq!(world.life.today.games, 1);
        assert_eq!(
            world.life.total.wins + world.life.total.losses + world.life.total.draws,
            1
        );
        assert!(
            world
                .life
                .milestones
                .iter()
                .any(|entry| entry.text == "We played our first game of noughts and crosses.")
        );
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::GameOver(outcome))
                .count(),
            1
        );
        assert!(!world.mark(0));
        world.step(4.1);
        assert!(world.needs_decision());
        assert_eq!(world.life.total.games, 1);
        world.step(40.0);
        assert!(world.game.is_none());
    }

    #[test]
    fn the_viewers_turn_times_out_once_and_the_plan_finishes_within_a_minute() {
        let mut world = World::new(9, 12.0);
        world.begin(Activity::Game);
        run_until(&mut world, viewer_can_move);
        let waiting_at = world.time;
        world.step(44.8);
        assert_eq!(world.game.as_ref().unwrap().outcome, None);
        // Other input cannot prolong a turn without placing a cross.
        world.notice_viewer();
        world.step(0.3);
        assert_eq!(
            world.game.as_ref().unwrap().outcome,
            Some(Outcome::Abandoned)
        );
        assert!(world.time - waiting_at >= 45.0);
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "Maybe later.");
        world.step(4.1);
        assert!(world.needs_decision());
        assert!(world.time < 70.0);
        assert_eq!(world.life.total.games, 0);
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::GameOver(Outcome::Abandoned))
                .count(),
            1
        );
    }

    #[test]
    fn long_games_wait_past_the_generic_plan_timeout() {
        let mut world = World::new(3, 12.0);
        assert!(world.challenge());
        run_until(&mut world, viewer_can_move);
        world.step(40.0);
        assert!(world.mark(0));
        run_until(&mut world, viewer_can_move);
        world.step(20.0);
        assert!(world.time > 60.0);
        assert!(!world.needs_decision());
        assert!(world.game.as_ref().unwrap().outcome.is_none());
        world.step(26.0);
        assert_eq!(
            world.game.as_ref().unwrap().outcome,
            Some(Outcome::Abandoned)
        );
    }

    #[test]
    fn interruptions_abandon_the_board_without_an_outcome_event() {
        for interruption in 0..4 {
            let mut world = World::new(3, 12.0);
            world.challenge();
            run_until(&mut world, viewer_can_move);
            match interruption {
                0 => world.begin(Activity::Float),
                1 => {
                    world.hear("no");
                }
                2 => {
                    world.hour = 23.0;
                    world.tuck_in();
                }
                _ => {
                    world.ghost.plan.done = true;
                    world.step(0.1);
                }
            }
            assert_eq!(
                world.game.as_ref().unwrap().outcome,
                Some(Outcome::Abandoned)
            );
            assert!(
                !world
                    .events
                    .iter()
                    .any(|event| matches!(event.kind, EventKind::GameOver(_)))
            );
            world.step(40.1);
            assert!(world.game.is_none());
        }
    }

    #[test]
    fn beginning_a_board_wipes_only_pictures_whose_strokes_reach_it() {
        use crate::world::Writing;

        let mut world = World::new(3, 12.0);
        let doodle = Doodle {
            picture: Picture::Sketch(Sketch::Heart),
            center: Vec2::new(world.ghost.position.x, 1.25),
            size: 0.3,
            started: 0.0,
        };
        let away = Doodle {
            center: Vec2::new(-1.5, 1.25),
            ..doodle.clone()
        };
        // A long word centered off the board still writes across it.
        let word = Doodle {
            picture: Picture::Word(Writing::new(
                "GOODNESS".to_owned(),
                false,
                1.0,
                &mut super::super::Random::new(1),
            )),
            center: Vec2::new(-0.5, 1.25),
            size: 1.0,
            started: 0.0,
        };
        assert!(word.picture.extent().x > 0.5);
        world.doodles.extend([doodle, away.clone(), word]);
        world.begin(Activity::Game);
        assert_eq!(world.doodles, [away]);
    }

    #[test]
    fn game_results_give_delight_good_sports_and_sore_losers_their_feelings() {
        let mut world = World::new(3, 12.0);
        world.life.nature = Some(Nature {
            tender: 0.0,
            ..Nature::default()
        });
        world.record(EventKind::GameOver(Outcome::GhostWon { line: [0, 1, 2] }));
        assert_eq!(world.feeling(), Mood::Excited);
        world.events.clear();
        world.record(EventKind::GameOver(Outcome::ViewerWon { line: [0, 1, 2] }));
        assert_eq!(world.feeling(), Mood::Grumpy);
        world.life.nature.as_mut().unwrap().tender = 0.41;
        assert_eq!(world.feeling(), Mood::Cheerful);
        world.life.nature.as_mut().unwrap().tender = 0.0;
        world.life.bond = 0.55;
        assert_eq!(world.feeling(), Mood::Cheerful);
        world.time += 8.1;
        assert_ne!(world.feeling(), Mood::Grumpy);
        world.events.clear();
        world.record(EventKind::GameOver(Outcome::Draw));
        assert_eq!(world.feeling(), Mood::Curious);
    }

    #[test]
    fn games_and_their_random_choices_repeat_for_a_seed() {
        let mut first = World::new(17, 12.0);
        let mut second = World::new(17, 12.0);
        for world in [&mut first, &mut second] {
            world.begin(Activity::Game);
        }
        for _ in 0..1400 {
            for world in [&mut first, &mut second] {
                if viewer_can_move(world) {
                    let cell = Game::free_cells(&world.game.as_ref().unwrap().cells)[0];
                    world.mark(cell);
                }
                world.step(0.05);
            }
            assert_eq!(first.game, second.game);
            assert_eq!(first.ghost.position, second.ghost.position);
        }
        assert_eq!(first.events, second.events);
        assert_eq!(first.life, second.life);
    }
}
