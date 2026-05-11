mod sprites;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rand::seq::SliceRandom;
use rand::thread_rng;

const SPRITE_SIZE: u32 = 128;

#[derive(Resource)]
struct SpriteHandles {
    path_straight: Handle<Image>,
    path_curve: Handle<Image>,
    key: Handle<Image>,
    snack: Handle<Image>,
    ogre: Handle<Image>,
    treasure: Handle<Image>,
    start_marker: Handle<Image>,
}

const GRID_W: i32 = 6;
const GRID_H: i32 = 6;
const CELL: f32 = 80.0;
const OGRE_TRACK_LEN: usize = 10;
const KEYS_NEEDED: u32 = 3;

const START_CELL: IVec2 = IVec2::new(0, 0);
const END_CELL: IVec2 = IVec2::new(GRID_W - 1, GRID_H - 1);

// Direction bits
const N: u8 = 1;
const E: u8 = 2;
const S: u8 = 4;
const W: u8 = 8;

fn dir_offset(d: u8) -> IVec2 {
    match d {
        N => IVec2::new(0, -1),
        E => IVec2::new(1, 0),
        S => IVec2::new(0, 1),
        W => IVec2::new(-1, 0),
        _ => IVec2::ZERO,
    }
}
fn opposite(d: u8) -> u8 {
    match d {
        N => S,
        S => N,
        E => W,
        W => E,
        _ => 0,
    }
}

fn rotate_dirs(d: u8, r: u8) -> u8 {
    // N=0, E=1, S=2, W=3 (clockwise)
    let mut out = 0u8;
    for (bit, idx) in [(N, 0u8), (E, 1), (S, 2), (W, 3)] {
        if d & bit != 0 {
            let new_idx = (idx + r) % 4;
            out |= match new_idx {
                0 => N,
                1 => E,
                2 => S,
                _ => W,
            };
        }
    }
    out
}

#[derive(Copy, Clone, Debug)]
enum Shape {
    Straight,
    Curve,
}

#[derive(Copy, Clone, Debug, Component)]
struct PathCard {
    shape: Shape,
    rotation: u8, // 0..4
}

impl PathCard {
    fn openings(&self) -> u8 {
        let base = match self.shape {
            Shape::Straight => N | S,
            Shape::Curve => N | E,
        };
        rotate_dirs(base, self.rotation)
    }
    fn rotated(mut self) -> Self {
        self.rotation = (self.rotation + 1) % 4;
        self
    }
}

#[derive(Copy, Clone, Debug)]
enum CardKind {
    Path(Shape),
    Ogre,
}

#[derive(Resource)]
struct Board {
    cells: Vec<Vec<Option<PathCard>>>, // [y][x]
    keys: Vec<IVec2>,
    snack: Option<IVec2>,
    keys_collected: u32,
    snacks_available: u32,
}
impl Board {
    fn new() -> Self {
        Self {
            cells: vec![vec![None; GRID_W as usize]; GRID_H as usize],
            keys: Vec::new(),
            snack: None,
            keys_collected: 0,
            snacks_available: 0,
        }
    }
    fn get(&self, c: IVec2) -> Option<PathCard> {
        if c.x < 0 || c.x >= GRID_W || c.y < 0 || c.y >= GRID_H {
            return None;
        }
        self.cells[c.y as usize][c.x as usize]
    }
    fn set(&mut self, c: IVec2, card: PathCard) {
        self.cells[c.y as usize][c.x as usize] = Some(card);
    }
    fn any_card(&self) -> bool {
        self.cells.iter().flatten().any(|c| c.is_some())
    }
}

#[derive(Resource, Default)]
struct Deck {
    cards: Vec<CardKind>,
}

#[derive(Resource, Default)]
struct OgreTrack {
    placed: usize,
}

#[derive(Resource, Default, PartialEq, Eq, Clone)]
enum Phase {
    #[default]
    WaitingDraw,
    PlacingPath,
    Won,
    LostOgre,
}

#[derive(Resource, Default)]
struct CurrentDraw {
    card: Option<PathCard>,
}

// Visual-only marker components
#[derive(Component)]
struct BoardViz;
#[derive(Component)]
struct GhostViz;
#[derive(Component)]
struct HudText;

fn cell_to_world(c: IVec2) -> Vec2 {
    let ox = -CELL * GRID_W as f32 / 2.0 + CELL / 2.0;
    let oy = CELL * GRID_H as f32 / 2.0 - CELL / 2.0;
    Vec2::new(ox + c.x as f32 * CELL, oy - c.y as f32 * CELL)
}

fn world_to_cell(world: Vec2) -> Option<IVec2> {
    let ox = -CELL * GRID_W as f32 / 2.0;
    let oy = CELL * GRID_H as f32 / 2.0;
    let rel = world - Vec2::new(ox, oy);
    let cx = (rel.x / CELL).floor() as i32;
    let cy = (-rel.y / CELL).floor() as i32;
    if cx >= 0 && cx < GRID_W && cy >= 0 && cy < GRID_H {
        Some(IVec2::new(cx, cy))
    } else {
        None
    }
}

fn is_legal(board: &Board, cell: IVec2, card: PathCard) -> bool {
    if cell.x < 0 || cell.x >= GRID_W || cell.y < 0 || cell.y >= GRID_H {
        return false;
    }
    if board.get(cell).is_some() {
        return false;
    }
    if !board.any_card() {
        return cell == START_CELL;
    }
    let op = card.openings();
    for d in [N, E, S, W] {
        if op & d != 0 {
            let n = cell + dir_offset(d);
            if let Some(nc) = board.get(n) {
                if nc.openings() & opposite(d) != 0 {
                    return true;
                }
            }
        }
    }
    false
}

fn path_connects(board: &Board) -> bool {
    if board.get(START_CELL).is_none() || board.get(END_CELL).is_none() {
        return false;
    }
    let mut seen = vec![vec![false; GRID_W as usize]; GRID_H as usize];
    let mut stack = vec![START_CELL];
    seen[START_CELL.y as usize][START_CELL.x as usize] = true;
    while let Some(cur) = stack.pop() {
        if cur == END_CELL {
            return true;
        }
        let Some(card) = board.get(cur) else {
            continue;
        };
        for d in [N, E, S, W] {
            if card.openings() & d != 0 {
                let next = cur + dir_offset(d);
                if next.x < 0 || next.x >= GRID_W || next.y < 0 || next.y >= GRID_H {
                    continue;
                }
                if seen[next.y as usize][next.x as usize] {
                    continue;
                }
                if let Some(nc) = board.get(next) {
                    if nc.openings() & opposite(d) != 0 {
                        seen[next.y as usize][next.x as usize] = true;
                        stack.push(next);
                    }
                }
            }
        }
    }
    false
}

fn build_deck() -> Vec<CardKind> {
    let mut v = Vec::with_capacity(37);
    // 27 path cards: split roughly evenly between straight and curve
    for _ in 0..13 {
        v.push(CardKind::Path(Shape::Straight));
    }
    for _ in 0..14 {
        v.push(CardKind::Path(Shape::Curve));
    }
    for _ in 0..OGRE_TRACK_LEN {
        v.push(CardKind::Ogre);
    }
    v.shuffle(&mut thread_rng());
    v
}

fn random_free_cells(count: usize, exclude: &[IVec2]) -> Vec<IVec2> {
    let mut pool: Vec<IVec2> = (0..GRID_H)
        .flat_map(|y| (0..GRID_W).map(move |x| IVec2::new(x, y)))
        .filter(|c| *c != START_CELL && *c != END_CELL && !exclude.contains(c))
        .collect();
    pool.shuffle(&mut thread_rng());
    pool.into_iter().take(count).collect()
}

fn setup(
    mut commands: Commands,
    mut board: ResMut<Board>,
    mut deck: ResMut<Deck>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.spawn(Camera2d);

    let handles = SpriteHandles {
        path_straight: images.add(sprites::gen_path_straight(SPRITE_SIZE)),
        path_curve: images.add(sprites::gen_path_curve(SPRITE_SIZE)),
        key: images.add(sprites::gen_key(SPRITE_SIZE)),
        snack: images.add(sprites::gen_snack(SPRITE_SIZE)),
        ogre: images.add(sprites::gen_ogre(SPRITE_SIZE)),
        treasure: images.add(sprites::gen_treasure(SPRITE_SIZE)),
        start_marker: images.add(sprites::gen_start_marker(SPRITE_SIZE)),
    };
    commands.insert_resource(handles);

    // Setup keys and snack positions
    let keys = random_free_cells(4, &[]);
    let snack_pool = random_free_cells(1, &keys);
    board.keys = keys;
    board.snack = snack_pool.first().copied();

    deck.cards = build_deck();

    // HUD text (Text2d at top of screen)
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_xyz(0.0, CELL * GRID_H as f32 / 2.0 + 50.0, 10.0),
        HudText,
    ));

    // Bottom help
    commands.spawn((
        Text2d::new(
            "SPACE: draw   R: rotate   LMB: place   1: use snack   ESC: discard unplayable card",
        ),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        Transform::from_xyz(0.0, -CELL * GRID_H as f32 / 2.0 - 90.0, 10.0),
    ));
}

fn color_water() -> Color {
    Color::srgb(0.10, 0.40, 0.50)
}
fn color_start() -> Color {
    Color::srgb(0.9, 0.75, 0.25)
}
fn color_end() -> Color {
    Color::srgb(0.85, 0.3, 0.2)
}
fn color_ogre_cell() -> Color {
    Color::srgb(0.75, 0.15, 0.15)
}

fn spawn_card_on(
    commands: &mut Commands,
    handles: &SpriteHandles,
    world_pos: Vec2,
    card: PathCard,
    alpha: f32,
    marker_ghost: bool,
    z: f32,
) {
    let handle = match card.shape {
        Shape::Straight => handles.path_straight.clone(),
        Shape::Curve => handles.path_curve.clone(),
    };
    // rotation: each step = 90° clockwise. Bevy +Z rot is CCW in screen space, so negate.
    let angle = -std::f32::consts::FRAC_PI_2 * card.rotation as f32;
    let mut e = commands.spawn((
        Sprite {
            image: handle,
            color: Color::srgba(1.0, 1.0, 1.0, alpha),
            custom_size: Some(Vec2::splat(CELL - 2.0)),
            ..default()
        },
        Transform {
            translation: world_pos.extend(z),
            rotation: Quat::from_rotation_z(angle),
            ..default()
        },
    ));
    if marker_ghost {
        e.insert(GhostViz);
    } else {
        e.insert(BoardViz);
    }
}

fn redraw_board(
    mut commands: Commands,
    existing: Query<Entity, With<BoardViz>>,
    board: Res<Board>,
    ogre: Res<OgreTrack>,
    handles: Option<Res<SpriteHandles>>,
) {
    if !board.is_changed() && !ogre.is_changed() {
        return;
    }
    let Some(handles) = handles else {
        return;
    };
    for e in &existing {
        commands.entity(e).despawn();
    }

    // Grid cells
    for y in 0..GRID_H {
        for x in 0..GRID_W {
            let cell = IVec2::new(x, y);
            let base = if cell == START_CELL {
                color_start()
            } else if cell == END_CELL {
                color_end()
            } else {
                color_water()
            };
            let pos = cell_to_world(cell);
            commands.spawn((
                Sprite {
                    color: base,
                    custom_size: Some(Vec2::splat(CELL - 2.0)),
                    ..default()
                },
                Transform::from_translation(pos.extend(0.0)),
                BoardViz,
            ));
        }
    }

    // Row/column labels
    for x in 0..GRID_W {
        let letter = (b'A' + x as u8) as char;
        let pos = cell_to_world(IVec2::new(x, 0)) + Vec2::new(0.0, CELL / 2.0 + 12.0);
        commands.spawn((
            Text2d::new(letter.to_string()),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.85, 0.2)),
            Transform::from_translation(pos.extend(5.0)),
            BoardViz,
        ));
    }
    for y in 0..GRID_H {
        let pos = cell_to_world(IVec2::new(0, y)) + Vec2::new(-CELL / 2.0 - 14.0, 0.0);
        commands.spawn((
            Text2d::new((y + 1).to_string()),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.85, 0.2)),
            Transform::from_translation(pos.extend(5.0)),
            BoardViz,
        ));
    }

    // Start marker
    {
        let pos = cell_to_world(START_CELL);
        commands.spawn((
            Sprite {
                image: handles.start_marker.clone(),
                custom_size: Some(Vec2::splat(CELL * 0.75)),
                ..default()
            },
            Transform::from_translation(pos.extend(0.3)),
            BoardViz,
        ));
    }
    // Treasure chest on END cell
    {
        let pos = cell_to_world(END_CELL);
        commands.spawn((
            Sprite {
                image: handles.treasure.clone(),
                custom_size: Some(Vec2::splat(CELL * 0.85)),
                ..default()
            },
            Transform::from_translation(pos.extend(0.3)),
            BoardViz,
        ));
    }
    // Keys on board — draw only if not yet covered
    for k in &board.keys {
        if board.get(*k).is_some() {
            continue;
        }
        let pos = cell_to_world(*k);
        commands.spawn((
            Sprite {
                image: handles.key.clone(),
                custom_size: Some(Vec2::splat(CELL * 0.7)),
                ..default()
            },
            Transform::from_translation(pos.extend(0.5)),
            BoardViz,
        ));
    }
    if let Some(s) = board.snack {
        if board.get(s).is_none() {
            let pos = cell_to_world(s);
            commands.spawn((
                Sprite {
                    image: handles.snack.clone(),
                    custom_size: Some(Vec2::splat(CELL * 0.7)),
                    ..default()
                },
                Transform::from_translation(pos.extend(0.5)),
                BoardViz,
            ));
        }
    }

    // Placed path cards
    for y in 0..GRID_H {
        for x in 0..GRID_W {
            if let Some(card) = board.cells[y as usize][x as usize] {
                let pos = cell_to_world(IVec2::new(x, y));
                spawn_card_on(&mut commands, &handles, pos, card, 1.0, false, 1.0);
            }
        }
    }

    // Ogre track along the right side of the board
    let track_x = CELL * GRID_W as f32 / 2.0 + 40.0;
    let track_top = CELL * GRID_H as f32 / 2.0;
    for i in 0..OGRE_TRACK_LEN {
        let y = track_top - i as f32 * 38.0 - 18.0;
        // empty slot: red square
        commands.spawn((
            Sprite {
                color: color_ogre_cell(),
                custom_size: Some(Vec2::new(36.0, 36.0)),
                ..default()
            },
            Transform::from_xyz(track_x, y, 0.0),
            BoardViz,
        ));
        if i < ogre.placed {
            commands.spawn((
                Sprite {
                    image: handles.ogre.clone(),
                    custom_size: Some(Vec2::new(34.0, 34.0)),
                    ..default()
                },
                Transform::from_xyz(track_x, y, 1.0),
                BoardViz,
            ));
        }
    }
    commands.spawn((
        Text2d::new("OGRE"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_xyz(track_x, track_top + 10.0, 1.0),
        BoardViz,
    ));
}

fn input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut board: ResMut<Board>,
    mut deck: ResMut<Deck>,
    mut ogre: ResMut<OgreTrack>,
    mut phase: ResMut<Phase>,
    mut current: ResMut<CurrentDraw>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) {
    if *phase == Phase::Won || *phase == Phase::LostOgre {
        return;
    }

    // Draw a card
    if *phase == Phase::WaitingDraw && keys.just_pressed(KeyCode::Space) {
        if let Some(k) = deck.cards.pop() {
            match k {
                CardKind::Ogre => {
                    ogre.placed += 1;
                    if ogre.placed >= OGRE_TRACK_LEN {
                        *phase = Phase::LostOgre;
                    }
                }
                CardKind::Path(shape) => {
                    current.card = Some(PathCard { shape, rotation: 0 });
                    *phase = Phase::PlacingPath;
                }
            }
        }
    }

    if *phase != Phase::PlacingPath {
        return;
    }
    let Some(card) = current.card else {
        return;
    };

    if keys.just_pressed(KeyCode::KeyR) {
        current.card = Some(card.rotated());
    }

    // Use snack
    if keys.just_pressed(KeyCode::Digit1) && board.snacks_available > 0 && ogre.placed > 0 {
        board.snacks_available -= 1;
        ogre.placed -= 1;
    }

    // Escape discards this card (players give up on placing)
    if keys.just_pressed(KeyCode::Escape) {
        current.card = None;
        *phase = Phase::WaitingDraw;
        return;
    }

    // Place
    if mouse.just_pressed(MouseButton::Left) {
        let Ok(window) = windows.get_single() else {
            return;
        };
        let Ok((cam, cam_tf)) = cameras.get_single() else {
            return;
        };
        let Some(cursor) = window.cursor_position() else {
            return;
        };
        let Ok(world) = cam.viewport_to_world_2d(cam_tf, cursor) else {
            return;
        };
        let Some(cell) = world_to_cell(world) else {
            return;
        };

        let card_now = current.card.unwrap();
        if is_legal(&board, cell, card_now) {
            board.set(cell, card_now);
            // collect key or snack
            if board.keys.contains(&cell) {
                board.keys.retain(|k| *k != cell);
                if board.keys_collected < KEYS_NEEDED + 1 {
                    board.keys_collected += 1;
                }
            }
            if board.snack == Some(cell) {
                board.snack = None;
                board.snacks_available += 1;
            }

            current.card = None;
            *phase = Phase::WaitingDraw;

            if cell == END_CELL && path_connects(&board) && board.keys_collected >= KEYS_NEEDED {
                *phase = Phase::Won;
            }
        }
    }
}

fn ghost_system(
    mut commands: Commands,
    ghosts: Query<Entity, With<GhostViz>>,
    phase: Res<Phase>,
    current: Res<CurrentDraw>,
    board: Res<Board>,
    handles: Option<Res<SpriteHandles>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) {
    for e in &ghosts {
        commands.entity(e).despawn();
    }
    if *phase != Phase::PlacingPath {
        return;
    }
    let Some(handles) = handles else {
        return;
    };
    let Some(card) = current.card else {
        return;
    };
    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok((cam, cam_tf)) = cameras.get_single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(world) = cam.viewport_to_world_2d(cam_tf, cursor) else {
        return;
    };
    let Some(cell) = world_to_cell(world) else {
        return;
    };
    let pos = cell_to_world(cell);
    let alpha = if is_legal(&board, cell, card) {
        0.75
    } else {
        0.3
    };
    spawn_card_on(&mut commands, &handles, pos, card, alpha, true, 3.0);
}

fn hud_system(
    phase: Res<Phase>,
    current: Res<CurrentDraw>,
    deck: Res<Deck>,
    board: Res<Board>,
    ogre: Res<OgreTrack>,
    mut q: Query<&mut Text2d, With<HudText>>,
) {
    let Ok(mut text) = q.get_single_mut() else {
        return;
    };
    let status = match *phase {
        Phase::WaitingDraw => "Press SPACE to draw a card".to_string(),
        Phase::PlacingPath => {
            let shape = match current.card.unwrap().shape {
                Shape::Straight => "STRAIGHT",
                Shape::Curve => "CURVE",
            };
            format!(
                "Placing {} (rot {}). R to rotate, click to place, ESC to discard.",
                shape,
                current.card.unwrap().rotation
            )
        }
        Phase::Won => "YOU WIN! Treasure secured!".to_string(),
        Phase::LostOgre => "THE OGRE WINS! Better luck next time.".to_string(),
    };
    text.0 = format!(
        "{}\nKeys: {}/{}   Snacks: {}   Ogre: {}/{}   Deck: {}",
        status,
        board.keys_collected,
        KEYS_NEEDED,
        board.snacks_available,
        ogre.placed,
        OGRE_TRACK_LEN,
        deck.cards.len()
    );
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Race to the Treasure!".into(),
                resolution: (900.0, 800.0).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.12, 0.15, 0.18)))
        .insert_resource(Board::new())
        .insert_resource(Deck::default())
        .insert_resource(OgreTrack::default())
        .insert_resource(Phase::default())
        .insert_resource(CurrentDraw::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (input_system, redraw_board, ghost_system, hud_system),
        )
        .run();
}
