//! Пиксельный кот: рисуется квадратными спрайтами 1x1, растянутыми пиксель-перфект
//! масштабированием. Никаких внешних ассетов и шейдеров — только код.

use bevy::prelude::*;

/// Размер "холста" кота в пикселях (сетка ART ниже)
pub const CAT_W: usize = 34;
pub const CAT_H: usize = 24;

/// Пиксель-перфект масштаб: одна "клетка" ART = N экранных пикселей
pub const PIXEL_SCALE: f32 = 8.0;

/// Палитра: символ -> цвет, None = прозрачный пиксель
fn color_of(c: char) -> Option<Color> {
    Some(match c {
        'o' => Color::srgb(0.95, 0.72, 0.30), // оранжевый мех
        'd' => Color::srgb(0.80, 0.55, 0.20), // тёмные полосы
        'w' => Color::srgb(1.00, 0.98, 0.92), // белые лапки/мордочка
        'p' => Color::srgb(1.00, 0.65, 0.75), // розовый нос/ушки
        'e' => Color::srgb(0.15, 0.15, 0.20), // глаза
        'k' => Color::srgb(0.10, 0.08, 0.14), // зрачок/контур
        'm' => Color::srgb(0.90, 0.40, 0.55), // рот
        _ => return None,                     // '.' - прозрачный
    })
}

/// Пиксельная сетка кота (сидит, морда вправо). 34x24.
const ART: [&str; CAT_H] = [
    "..kk..........kk............",
    ".kook........koook..........",
    ".kopok......koppok..........",
    "..koo.k....koo..k...........",
    "...oooooo.oooooo..kkkkk.....",
    "..ooooooooooooooooopppppk...",
    ".odoooooooooooooooooppoopk..",
    ".ooodoooooooeekooooopppppk..",
    "oodoooooooooeekooooo.kkkk...",
    "odooooooooooommmooooo.......",
    "odoooooooooooowwooooo.......",
    ".odoooooooooooosooooooo.....",
    ".ooodooooooooooooooooo.o....",
    "..oooodddddddooooooo...o....",
    "...oooooooooooooo....oo.....",
    "....oooooooooooo...oo.......",
    ".....wwwww.wwwww.oo.........",
    ".....wkkw..wkkw..ok.........",
    ".....wwww..wwww.kk..........",
    "......kkk...kkk.............",
    "............................",
    "............................",
    "......................kkkk..",
    ".....................koo..k.",
];

/// Спавн кота: один квадратный спрайт на каждый непустой пиксель ART.
/// `parent` — родительский узел (можно UI-Node), корень двигают/масштабируют целиком.
pub fn spawn_pixel_cat(commands: &mut Commands, images: &mut Assets<Image>, parent: Entity) {
    // Один белый пиксель-Image переиспользуется; цвет задаёт Sprite::color
    let white_px = images.add(Image::from_fill(1, [255, 255, 255, 255]));

    let ox = CAT_W as f32 / 2.0;
    let oy = CAT_H as f32 / 2.0;

    for (y, row) in ART.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if let Some(color) = color_of(ch) {
                commands.spawn((
                    Sprite {
                        image: white_px.clone(),
                        color,
                        custom_size: Some(Vec2::new(PIXEL_SCALE, PIXEL_SCALE)),
                        ..default()
                    },
                    // локально относительно центра холста, Y инвертирован (ART читается сверху вниз)
                    Transform::from_xyz(
                        (x as f32 - ox + 0.5) * PIXEL_SCALE,
                        (oy - y as f32 - 0.5) * PIXEL_SCALE,
                        0.0,
                    ),
                    ChildOf(parent),
                ));
            }
        }
    }
}
