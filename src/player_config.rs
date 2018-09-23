extern crate ggez;
use ggez::error::GameResult;
use ggez::graphics;
use notefield::Judgement;
use std::fs::File;
use std::io::Read;
use timingdata::GameplayInfo;
use toml;

#[derive(PartialEq)]
pub struct NoteLayout {
    pub arrows_sprite: graphics::Image,
    pub receptor_sprite: graphics::Image,
    pub judgment_sprite: graphics::Image,
    pub column_positions: [i64; 4],
    pub column_rotations: [f32; 4],
    pub receptor_height: i64,
    pub judgment_position: graphics::Point2,
    pub scroll_speed: f32,
}

#[derive(PartialEq, Clone)]
pub struct NoteSkin {
    arrows_sprite: graphics::Image,
    receptor_sprite: graphics::Image,
    judgment_sprite: graphics::Image,
    column_positions: [i64; 4],
    column_rotations: [f32; 4],
}

#[derive(PartialEq, Copy, Clone)]
pub struct PlayerOptions {
    notefield_position: i64,
    receptor_height: i64,
    scroll_speed: f32,
    is_reverse: bool,
    judgment_position: (f32, f32),
}

impl NoteLayout {
    pub fn new(skin: &NoteSkin, screen_height: i64, player_options: PlayerOptions) -> NoteLayout {
        let NoteSkin {
            arrows_sprite,
            receptor_sprite,
            judgment_sprite,
            mut column_positions,
            mut column_rotations,
        } = skin.clone();
        let PlayerOptions {
            notefield_position,
            mut receptor_height,
            mut scroll_speed,
            is_reverse,
            mut judgment_position,
        } = player_options;
        column_positions
            .iter_mut()
            .for_each(|x| *x += notefield_position);
        column_rotations.iter_mut().for_each(|x| *x *= 6.28 / 360.0);
        judgment_position.0 += notefield_position as f32;
        if is_reverse {
            receptor_height = screen_height - receptor_height;
            judgment_position.1 = screen_height as f32 - judgment_position.1;
            scroll_speed *= -1.0;
        }
        let judgment_position = graphics::Point2::new(judgment_position.0, judgment_position.1);
        NoteLayout {
            column_positions,
            column_rotations,
            arrows_sprite,
            receptor_sprite,
            judgment_sprite,
            receptor_height,
            judgment_position,
            scroll_speed,
        }
    }
    pub fn delta_to_position(&self, delta: i64) -> i64 {
        (delta as f32 * self.scroll_speed) as i64 + self.receptor_height
    }
    pub fn delta_to_offset(&self, delta: i64) -> f32 {
        (delta as f32 * self.scroll_speed)
    }
    pub fn add_note(
        &self,
        column: usize,
        position: i64,
        coords: graphics::Rect,
        batch: &mut graphics::spritebatch::SpriteBatch,
    ) {
        batch.add(graphics::DrawParam {
            src: coords,
            dest: graphics::Point2::new(self.column_positions[column] as f32, position as f32),
            rotation: self.column_rotations[column],
            offset: graphics::Point2::new(0.5, 0.5),
            ..Default::default()
        });
    }
    pub fn add_column_of_notes(
        &self,
        column: impl Iterator<Item = GameplayInfo>,
        column_index: usize,
        batch: &mut graphics::spritebatch::SpriteBatch,
    ) {
        for GameplayInfo(note, coords) in column {
            self.add_note(column_index, self.delta_to_position(note), coords, batch);
        }
    }
    pub fn draw_receptors(&self, ctx: &mut ggez::Context) -> Result<(), ggez::GameError> {
        for (index, &column_position) in self.column_positions.iter().enumerate() {
            graphics::draw_ex(
                ctx,
                &self.receptor_sprite,
                graphics::DrawParam {
                    dest: graphics::Point2::new(
                        column_position as f32,
                        self.receptor_height as f32,
                    ),
                    rotation: self.column_rotations[index],
                    offset: graphics::Point2::new(0.5, 0.5),
                    ..Default::default()
                },
            )?;
        }
        Ok(())
    }
    //this will likely be the method to draw receptors in the future, but it is not currently in use
    pub fn _add_receptors(
        &self,
        batch: &mut graphics::spritebatch::SpriteBatch,
    ) -> Result<(), ggez::GameError> {
        for &column_position in &self.column_positions {
            batch.add(graphics::DrawParam {
                dest: graphics::Point2::new(column_position as f32, self.receptor_height as f32),
                ..Default::default()
            });
        }
        Ok(())
    }
    fn select_judgment(&self, judge: Judgement) -> graphics::DrawParam {
        let src = match judge {
            Judgement::Hit(0) => graphics::Rect::new(0.0, 0.0, 1.0, 0.1666),
            Judgement::Hit(1) => graphics::Rect::new(0.0, 0.1666, 1.0, 0.1666),
            Judgement::Hit(2) => graphics::Rect::new(0.0, 0.3333, 1.0, 0.1666),
            Judgement::Hit(3) => graphics::Rect::new(0.0, 0.5, 1.0, 0.1666),
            Judgement::Hit(_) => graphics::Rect::new(0.0, 0.6666, 1.0, 0.1666),
            Judgement::Miss => graphics::Rect::new(0.0, 0.8333, 1.0, 1.666),
        };
        graphics::DrawParam {
            src,
            dest: self.judgment_position,
            ..Default::default()
        }
    }
    pub fn draw_judgment(
        &self,
        ctx: &mut ggez::Context,
        judge: Judgement,
    ) -> Result<(), ggez::GameError> {
        graphics::draw_ex(ctx, &self.judgment_sprite, self.select_judgment(judge))?;
        Ok(())
    }
}

#[derive(Deserialize)]
struct NoteSkinInfo {
    arrows: String,
    receptor: String,
    judgment: String,
    column_positions: [i64; 4],
    column_rotations: [f32; 4],
}

impl NoteSkin {
    pub fn from_path(path: &str, context: &mut ggez::Context) -> Option<Self> {
        let mut config_file = match File::open(format!("{}/config.toml", path)) {
            Ok(file) => file,
            Err(_) => return None,
        };
        let mut config_string = String::new();
        match config_file.read_to_string(&mut config_string) {
            Ok(_) => {}
            Err(_) => return None,
        };
        let NoteSkinInfo {
            arrows,
            receptor,
            judgment,
            column_positions,
            column_rotations,
        } = match toml::from_str(&config_string) {
            Ok(skin) => skin,
            Err(_) => return None,
        };
        if let (Ok(arrows_sprite), Ok(receptor_sprite), Ok(judgment_sprite)) = (
            image_from_subdirectory(context, path, arrows),
            image_from_subdirectory(context, path, receptor),
            image_from_subdirectory(context, path, judgment),
        ) {
            Some(NoteSkin {
                arrows_sprite,
                receptor_sprite,
                judgment_sprite,
                column_positions,
                column_rotations,
            })
        } else {
            None
        }
    }
}

fn image_from_subdirectory(
    context: &mut ggez::Context,
    path: &str,
    extension: String,
) -> GameResult<graphics::Image> {
    graphics::Image::new(context, format!("/{}/{}", path, extension))
}

impl PlayerOptions {
    pub fn new(
        notefield_position: i64,
        receptor_height: i64,
        scroll_speed: f32,
        is_reverse: bool,
        judgment_position: (f32, f32),
    ) -> Self {
        PlayerOptions {
            notefield_position,
            receptor_height,
            scroll_speed,
            is_reverse,
            judgment_position,
        }
    }
}
