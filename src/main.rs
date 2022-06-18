use bevy::prelude::*;
use rand::prelude::random;
use bevy::core::FixedTimestep;

const SCALE:          i32 = 40;
const ARENA_WIDTH:    i32 = SCALE;
const ARENA_HEIGHT:   i32 = SCALE;
const STARTING_POINT: i32 = SCALE/2;
const SEGMENT_SIZE:   f32 = 0.9;

const BRICK_COLOR:         Color = Color::hsla(333.0,0.8,0.6,0.6);
const FOOD_COLOR:          Color = Color::hsla(23.0,0.8,0.6,0.6);
const SNAKE_HEAD_COLOR:    Color = Color::hsla(183.0,0.3,0.7,0.9);
const SNAKE_SEGMENT_COLOR: Color = Color::hsla(183.0,0.3,0.7,0.5);
const BACKGROUND_COLOR:    Color = Color::hsl(183.0,0.3,0.1);


fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Component)]
struct Size {
    width: f32,
    height: f32,
}

impl Size {
    pub fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x,
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
enum Direction {
    Left,
    Up,
    Right,
    Down,
}

impl Direction {
    fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Right => Self::Left,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
        }
    }
}

#[derive(Component)]
struct SnakeHead {
    direction: Direction,
}

#[derive(Component)]
struct SnakeSegment;

#[derive(Default, Deref, DerefMut)]
struct SnakeSegments(Vec<Entity>);

#[derive(Default)]
struct LastTailPosition(Option<Position>);

struct GameOverEvent;

fn spawn_snake(mut commands: Commands, mut segments: ResMut<SnakeSegments>) {
    *segments = SnakeSegments(vec![
        commands
            .spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    color: SNAKE_HEAD_COLOR,
                    ..default()
                },
                ..default()
            })
            .insert(SnakeHead {
                direction: Direction::Up,
            })
            .insert(SnakeSegment)
            .insert(Position { x: STARTING_POINT, y: STARTING_POINT })
            .insert(Size::square(SEGMENT_SIZE))
            .id(),
        spawn_segment(commands, Position{ x: STARTING_POINT, y: STARTING_POINT-1 }),
    ])
}

fn spawn_segment(mut commands: Commands, position: Position)->Entity {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: SNAKE_SEGMENT_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(SnakeSegment)
        .insert(position)
        .insert(Size::square(SEGMENT_SIZE))
        .id()
}

fn size_scaling(windows: Res<Windows>, mut q: Query<(&Size, &mut Transform)>) {
    let window = windows.get_primary().unwrap();
    for (sprite_size, mut transform) in q.iter_mut() {
        transform.scale = Vec3::new(
            sprite_size.width / ARENA_WIDTH as f32 * window.width() as f32,
            sprite_size.height / ARENA_HEIGHT as f32 * window.height() as f32,
            1.0,
        )
    }
}

fn position_translation(windows: Res<Windows>, mut q: Query<(&Position, &mut Transform)>) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }
    let window = windows.get_primary().unwrap();
    for (pos, mut transform) in q.iter_mut() {
        transform.translation = Vec3::new(
            convert(pos.x as f32, window.width() as f32, ARENA_WIDTH as f32),
            convert(pos.y as f32, window.height() as f32, ARENA_HEIGHT as f32),
            0.0
        );
    }
}

fn snake_movement_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut heads: Query<&mut SnakeHead>
) {
   if let Some(mut head) = heads.iter_mut().next() {
        let dir: Direction = if keyboard_input.pressed(KeyCode::Up) {
            Direction::Up
        } else if keyboard_input.pressed(KeyCode::Right) {
            Direction::Right
        } else if keyboard_input.pressed(KeyCode::Down) {
            Direction::Down
        } else if keyboard_input.pressed(KeyCode::Left) {
            Direction::Left
        } else {
            head.direction
        };
        if dir != head.direction.opposite() {
            head.direction = dir;
        }
    }
}

fn snake_movement(
    segments: ResMut<SnakeSegments>,
    mut heads: Query<(Entity, &SnakeHead)>,
    mut positions: Query<&mut Position>,
    mut last_tail_position: ResMut<LastTailPosition>,
    mut game_over_writer: EventWriter<GameOverEvent>
) {
    if let Some((head_entity, head)) = heads.iter_mut().next() {
        let segment_positions = segments
            .iter()
            .map(|e| *positions.get_mut(*e).unwrap())
            .collect::<Vec<Position>>();
        let mut head_pos = positions.get_mut(head_entity).unwrap();
        match &head.direction {
            Direction::Up => {
                head_pos.y += 1;
            }
            Direction::Right => {
                head_pos.x += 1;
            }
            Direction::Down => {
                head_pos.y -= 1;
            }
            Direction::Left => {
                head_pos.x -= 1;
            }
        };
        if head_pos.x < 1
            || head_pos.y < 1
            || head_pos.x as i32 >= ARENA_WIDTH-1
            || head_pos.y as i32 >= ARENA_HEIGHT-1
        {
            game_over_writer.send(GameOverEvent);
        }
        if segment_positions.contains(&head_pos) {
            game_over_writer.send(GameOverEvent);
        }
        segment_positions
            .iter()
            .zip(segments.iter().skip(1))
            .for_each(|(pos, segment)| {
                *positions.get_mut(*segment).unwrap() = *pos;
            });
        *last_tail_position = LastTailPosition(Some(*segment_positions.last().unwrap()));
    }
}

struct GrowthEvent;

fn snake_eating(
    mut commands: Commands,
    mut growth_writer: EventWriter<GrowthEvent>,
    food_positions: Query<(Entity, &Position), With<Food>>,
    head_positions: Query<&Position, With<SnakeHead>>
) {
    for head_pos in head_positions.iter() {
        for (ent, food_pos) in food_positions.iter() {
            if food_pos == head_pos {
                commands.entity(ent).despawn();
                growth_writer.send(GrowthEvent);
            }
        }
    }
}

fn snake_growth(
    commands: Commands,
    last_tail_position: Res<LastTailPosition>,
    mut segments: ResMut<SnakeSegments>,
    mut growth_reader: EventReader<GrowthEvent>
) {
    if growth_reader.iter().next().is_some() {
        segments.push(spawn_segment(commands, last_tail_position.0.unwrap()));
    }
}

#[derive(Component)]
struct Brick;

fn brick_spawner(commands: &mut Commands, position: Position)->Entity {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: BRICK_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Brick)
        .insert(position)
        .insert(Size::square(SEGMENT_SIZE))
        .id()
}

#[derive(Default, Deref, DerefMut)]
struct Wall(Vec<Entity>);

fn wall_builder(mut commands: Commands, mut wall: ResMut<Wall>) {
    for coordinate in 0..ARENA_WIDTH {
        wall.push(brick_spawner(&mut commands, Position { x: coordinate as i32, y: 0 as i32 }));
        wall.push(brick_spawner(&mut commands, Position { x: ARENA_WIDTH-1 as i32, y: coordinate as i32 }));
        wall.push(brick_spawner(&mut commands, Position { x: coordinate as i32, y: ARENA_WIDTH-1 as i32 }));
        wall.push(brick_spawner(&mut commands, Position { x: 0 as i32, y: coordinate as i32 }));
    }
}

#[derive(Component)]
struct Food;

fn food_spawner(mut commands: Commands) {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: FOOD_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Food)
        .insert(Position {
            x: (random::<f32>() * ARENA_WIDTH as f32) as i32,
            y: (random::<f32>() * ARENA_HEIGHT as f32) as i32,
        })
        .insert(Size::square(SEGMENT_SIZE));
}

fn game_over(
    mut commands: Commands,
    mut reader: EventReader<GameOverEvent>,
    segments_res: ResMut<SnakeSegments>,
    food: Query<Entity, With<Food>>,
    segments: Query<Entity, With<SnakeSegment>>
) {
    if reader.iter().next().is_some() {
        for ent in food.iter().chain(segments.iter()) {
            commands.entity(ent).despawn();
        }
        spawn_snake(commands, segments_res);
    }
}

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Snake!".to_string(),
            width: 800.0,
            height: 800.0,
            ..default()
        })
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .insert_resource(SnakeSegments::default())
        .insert_resource(LastTailPosition::default())
        .add_startup_system(setup_camera)
        .insert_resource(Wall::default())
        .add_startup_system(wall_builder)
        .add_startup_system(spawn_snake)
        .add_system(snake_movement_input.before(snake_movement))
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .with_system(position_translation)
                .with_system(size_scaling),
        )
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(0.200))
                .with_system(snake_movement)
                .with_system(snake_eating.after(snake_movement))
                .with_system(snake_growth.after(snake_eating)),
        )
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(2.0))
                .with_system(food_spawner),
        )
        .add_system(game_over.after(snake_movement))
        .add_plugins(DefaultPlugins)
        .add_event::<GameOverEvent>()
        .add_event::<GrowthEvent>()
        .run();
}
