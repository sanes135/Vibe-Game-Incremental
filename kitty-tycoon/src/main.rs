//! «КОТОТАЙКУН: Лапки и Мемы» — инкрементальный кликер на Bevy 0.16.
//!
//! Гладь кота → получай лапки → покупай улучшения → копи 1B лапок за забег →
//! перерождайся за кошек → покупай кошачьи улучшения → числа улетают в 1e1000+.
//!
//! Ядро игры — система BigNum (src/big_num.rs) с буквенным форматированием
//! (K, M, B, T, Qa... Dc, дальше научная запись "1.23e45").

mod big_num;
mod game_state;
mod pixel_cat;

use bevy::prelude::*;
use std::fs;

use game_state::{Effect, CAT_UPGRADES, PRESTIGE_THRESHOLD, UPGRADES};
use pixel_cat::{spawn_pixel_cat, CAT_H, CAT_W, PIXEL_SCALE};

const SAVE_PATH: &str = "kitty_save.json";

// ---------------------------------------------------------------------------
// Компоненты / ресурсы
// ---------------------------------------------------------------------------

#[derive(Component)]
struct PawsText;
#[derive(Component)]
struct RateText;
#[derive(Component)]
struct ClickPowerText;
#[derive(Component)]
struct CatsText;
#[derive(Component)]
struct PrestigeInfoText;

#[derive(Component)]
struct BuyButton {
    idx: usize,
}
#[derive(Component)]
struct BuyLabel {
    idx: usize,
}
#[derive(Component)]
struct CatBuyButton {
    idx: usize,
}
#[derive(Component)]
struct CatBuyLabel {
    idx: usize,
}

#[derive(Component)]
struct PrestigeButton;
#[derive(Component)]
struct ResetButton;

/// Невидимая UI-зона клика поверх кота (Interaction обновляет CursorOverCat)
#[derive(Component)]
struct CatClickZone;

/// Маркер корня кота в UI-слое (взаимодействие с ним не нужно)
#[derive(Component)]
struct CatRootMarker;

/// Флаг: курсор сейчас над котом (клик засчитывается только тогда)
#[derive(Resource, Default)]
struct CursorOverCat(bool);

/// Анимация «поглаживания»: корень кота слегка сжимается при клике
#[derive(Component)]
struct CatSquish {
    t: f32,
}

/// Всплывающий текст "+X" после гладки
#[derive(Component)]
struct FloatingText {
    life: f32,
    vel: Vec2,
}

/// Сколько штук покупать за клик кнопки: 1 | 10 | 1000 ("MAX"-кап)
#[derive(Resource)]
struct BuyAmount(pub u32);

#[derive(Component)]
struct AmountBtn(pub u32);

// ---------------------------------------------------------------------------
// Палитра
// ---------------------------------------------------------------------------

const COL_BG: Color = Color::srgb(0.11, 0.09, 0.15);
const COL_PANEL: Color = Color::srgb(0.17, 0.14, 0.22);
const COL_BTN: Color = Color::srgb(0.32, 0.24, 0.45);
const COL_BTN_OK: Color = Color::srgb(0.25, 0.55, 0.32);
const COL_BTN_OFF: Color = Color::srgb(0.22, 0.18, 0.26);
const COL_GOLD: Color = Color::srgb(1.0, 0.85, 0.35);
const COL_TEXT: Color = Color::srgb(0.95, 0.93, 0.88);
const COL_DIM: Color = Color::srgb(0.65, 0.6, 0.72);
const COL_PINK: Color = Color::srgb(1.0, 0.6, 0.8);

// ---------------------------------------------------------------------------
// Настройка сцены
// ---------------------------------------------------------------------------

fn startup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    // Камера: пиксель-перфект рендер (спрайты рисуются чёткими квадратами)
    commands.spawn((Camera2d, Aa2d::Centered(2.0)));

    // Тёмный фон
    commands.spawn((
        Sprite {
            color: COL_BG,
            custom_size: Some(Vec2::new(10000.0, 10000.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -100.0),
    ));

    // Пиксельный кот слева от центра. Родителем делаем UI-узел: тогда он лежит
    // в общем Z-порядке с интерфейсом (Node+BackgroundColor) и рисуется поверх
    // спрайта фона. Корень держит анимацию сжатия.
    let cat_pos = Vec2::new(-320.0, 40.0);
    let cat_root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(640.0 + cat_pos.x),
                top: Val::Px(360.0 - cat_pos.y),
                ..default()
            },
            BackgroundColor(Color::NONE),
            CatRootMarker,
            CatSquish { t: 10.0 },
        ))
        .id();
    spawn_pixel_cat(&mut commands, &mut images, ChildOf(cat_root));
    commands.insert_resource(CursorOverCat(false));

    let font: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");
    let txt = move |s: String, size: f32, c: Color| Text {
        sections: vec![TextSection {
            value: s.into(),
            style: TextStyle {
                font: font.clone(),
                font_size: size,
                color: c,
            },
        }],
        ..default()
    };

    // --- Невидимая зона клика поверх кота -----------------------------------
    // Мир -> экран приresolution 1280x720 и камере в центре: world(0,0)=px(640,360).
    commands.spawn((
        Button,
        CatClickZone,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(
                640.0 + cat_pos.x - (CAT_W as f32 * PIXEL_SCALE) / 2.0 - 40.0,
            ),
            top: Val::Px(
                360.0 - cat_pos.y - (CAT_H as f32 * PIXEL_SCALE) / 2.0 - 40.0,
            ),
            width: Val::Px(CAT_W as f32 * PIXEL_SCALE + 80.0),
            height: Val::Px(CAT_H as f32 * PIXEL_SCALE + 80.0),
            ..default()
        },
        BackgroundColor(Color::NONE),
    ));

    // --- Верхняя панель счётчиков ------------------------------------------
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(24.0),
            top: Val::Px(14.0),
            column_gap: Val::Px(28.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((PawsText, txt(String::new(), 40.0, COL_GOLD)));
            row.spawn((RateText, txt(String::new(), 20.0, COL_TEXT)));
            row.spawn((ClickPowerText, txt(String::new(), 18.0, COL_DIM)));
            row.spawn((CatsText, txt(String::new(), 22.0, COL_PINK)));
        });

    // --- Магазин улучшений за лапки (правая колонка) ------------------------
    let mut shop = commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            top: Val::Px(90.0),
            bottom: Val::Px(16.0),
            width: Val::Px(430.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            padding: UiRect::all(Val::Px(10.0)),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        BackgroundColor(COL_PANEL),
    ));

    shop.with_child(txt("МАГАЗИН ЛАПОК".into(), 22.0, COL_GOLD));

    // Переключатель объёма покупки x1 / x10 / x1000
    shop.with_children(|col| {
        col.spawn(Node {
            column_gap: Val::Px(6.0),
            margin: UiRect::bottom(Val::Px(4.0)),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            for &a in &[1u32, 10, 1000] {
                row.spawn((
                    AmountBtn(a),
                    Button,
                    Node {
                        width: Val::Px(70.0),
                        height: Val::Px(30.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(if a == 1 { COL_BTN_OK } else { COL_BTN_OFF }),
                ))
                .with_child(txt(format!("x{a}"), 15.0, COL_TEXT));
            }
            row.spawn(txt("x1000 ~= MAX", 12.0, COL_DIM));
        });
    });

    for (i, def) in UPGRADES.iter().enumerate() {
        shop.with_children(|col| {
            col.spawn((
                Button,
                BuyButton { idx: i },
                Node {
                    min_width: Val::Px(400.0),
                    height: Val::Px(58.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(10.0)),
                    column_gap: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(COL_BTN_OFF),
            ))
            .with_children(|b| {
                b.spawn((
                    PointerInteraction::Ignore,
                    Node {
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        flex_grow: 1.0,
                        ..default()
                    },
                ))
                .with_children(|info| {
                    info.spawn(txt(
                        format!("{}  [{}]  x{{n}}", def.name, effect_tag(def.effect)),
                        15.0,
                        COL_TEXT,
                    ));
                    info.spawn(txt(def.desc.into(), 11.0, COL_DIM));
                });
                b.spawn((
                    BuyLabel { idx: i },
                    PointerInteraction::Ignore,
                    txt(String::new(), 14.0, COL_GOLD),
                ));
            });
        });
    }

    // --- Кнопки перерождения / сброса ---------------------------------------
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(24.0),
            bottom: Val::Px(340.0),
            column_gap: Val::Px(10.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Button,
                PrestigeButton,
                Node {
                    height: Val::Px(54.0),
                    padding: UiRect::horizontal(Val::Px(16.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(COL_BTN),
            ))
            .with_child((PrestigeInfoText, txt("ПЕРЕРОЖДЕНИЕ".into(), 17.0, COL_TEXT)));

            row.spawn((
                Button,
                ResetButton,
                Node {
                    height: Val::Px(54.0),
                    padding: UiRect::horizontal(Val::Px(12.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(COL_BTN_OFF),
            ))
            .with_child(txt("сброс всё".into(), 13.0, COL_DIM));
        });

    // --- Магазин кошачьих улучшений (внизу слева) ----------------------------
    let mut catshop = commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(24.0),
            bottom: Val::Px(16.0),
            width: Val::Px(560.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(5.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(COL_PANEL),
    ));

    catshop.with_child(txt(
        "КОШАЧЬИ УЛУЧШЕНИЯ (за перерождение)".into(),
        18.0,
        COL_PINK,
    ));

    for (i, def) in CAT_UPGRADES.iter().enumerate() {
        catshop.with_children(|col| {
            col.spawn((
                Button,
                CatBuyButton { idx: i },
                Node {
                    height: Val::Px(46.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(8.0)),
                    column_gap: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(COL_BTN_OFF),
            ))
            .with_children(|b| {
                b.spawn((
                    PointerInteraction::Ignore,
                    Node {
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        flex_grow: 1.0,
                        ..default()
                    },
                ))
                .with_children(|info| {
                    info.spawn(txt(def.name.into(), 14.0, COL_TEXT));
                    info.spawn(txt(def.desc.into(), 11.0, COL_DIM));
                });
                b.spawn((
                    CatBuyLabel { idx: i },
                    PointerInteraction::Ignore,
                    txt(String::new(), 14.0, COL_PINK),
                ));
            });
        });
    }
}

fn effect_tag(e: Effect) -> &'static str {
    match e {
        Effect::PerClick(_) => "+клик",
        Effect::ClickMult(_) => "x клик",
        Effect::PerSecond(_) => "/сек",
        Effect::AllMult(_) => "x всё",
        Effect::AutoClick(_) => "авто",
    }
}

// ---------------------------------------------------------------------------
// Игровые системы
// ---------------------------------------------------------------------------

/// Пассивный доход + автокликер начисляются каждый кадр
fn income_system(time: Res<Time>, mut g: ResMut<game_state::GameState>) {
    let rate = g.total_rate();
    if !rate.is_zero() {
        g.earn(rate * time.delta_secs_f64());
    }
}

/// Курсор над зоной кота? (UI Raycasting сам обновляет Interaction зоны)
fn hover_system(mut over: ResMut<CursorOverCat>, zone_q: Single<&Interaction, With<CatClickZone>>) {
    over.0 = **zone_q != Interaction::None;
}

/// Клик по коту: начисляем лапки, запускаем анимацию и всплывающий текст
fn click_system(
    mut g: ResMut<game_state::GameState>,
    mut clicks: ResMut<ButtonInput<MouseButton>>,
    over: Res<CursorOverCat>,
    mut squish_q: Query<&mut CatSquish>,
    mut commands: Commands,
    windows: Single<&Window>,
    cam_q: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
    font: Res<DefaultFont>,
) {
    if !clicks.just_pressed(MouseButton::Left) || !over.0 {
        return;
    }
    let gain = g.click_power();
    g.earn(gain);

    // перезапуск анимации сжатия кота
    for mut s in squish_q.iter_mut() {
        s.t = 0.0;
    }

    // попап "+X" в точке курсора
    if let Some(cursor) = windows.cursor_position() {
        let (cam, cam_tf) = *cam_q;
        if let Some(world) = cam.viewport_to_world_2d(cam_tf, cursor) {
            commands.spawn((
                Text::from_section(
                    format!("+{}", gain.format_short()),
                    TextStyle {
                        font: font.handle.clone(),
                        font_size: 26.0,
                        color: COL_GOLD,
                    },
                ),
                FloatingText {
                    life: 1.0,
                    vel: Vec2::new(0.0, 90.0),
                },
                Transform::from_xyz(world.x, world.y + 20.0, 10.0),
            ));
        }
    }

    clicks.reset(MouseButton::Left);
}

/// Схлопывание кота и полёт всплывающих текстов
fn animation_system(
    time: Res<Time>,
    mut squish_q: Query<(&mut CatSquish, &mut Transform), With<CatRootMarker>>,
    mut float_q: Query<(Entity, &mut FloatingText, &mut Transform)>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    for (mut s, mut tr) in squish_q.iter_mut() {
        if s.t < 0.28 {
            s.t += dt;
            let k = (s.t / 0.28 * std::f32::PI()).sin();
            tr.scale = Vec3::new(1.0 + 0.06 * k, 1.0 - 0.09 * k, 1.0);
        }
    }
    for (entity, mut ft, mut tr) in float_q.iter_mut() {
        ft.life -= dt;
        tr.translation.y += ft.vel.y * dt;
        tr.translation.x += ft.vel.x * dt;
        if ft.life <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Текстовые счётчики сверху
fn ui_text_system(
    g: Res<game_state::GameState>,
    mut paws_q: Query<&mut Text, With<PawsText>>,
    mut rate_q: Query<&mut Text, With<RateText>>,
    mut cp_q: Query<&mut Text, With<ClickPowerText>>,
    mut cats_q: Query<&mut Text, With<CatsText>>,
) {
    let total_rate = g.total_rate();
    **paws_q.single_mut() = Text::from_section(
        format!("\u{1f43e} {}", g.paws.format_short()),
        TextStyle {
            font_size: 40.0,
            color: COL_GOLD,
            ..default()
        },
    );
    **rate_q.single_mut() = Text::from_section(
        format!("{}/сек", total_rate.format_rate()),
        TextStyle {
            font_size: 20.0,
            color: COL_TEXT,
            ..default()
        },
    );
    **cp_q.single_mut() = Text::from_section(
        format!("за глад: {}", g.click_power().format_short()),
        TextStyle {
            font_size: 18.0,
            color: COL_DIM,
            ..default()
        },
    );
    **cats_q.single_mut() = Text::from_section(
        format!("\u{1f408} кошек: {}", g.cats),
        TextStyle {
            font_size: 22.0,
            color: COL_PINK,
            ..default()
        },
    );
}

/// Цены/доступность кнопок лапкового магазина
fn shop_buttons_system(
    g: Res<game_state::GameState>,
    amount: Res<BuyAmount>,
    mut labels: Query<(&BuyLabel, &mut Text)>,
    mut btns: Query<(&BuyButton, &mut BackgroundColor)>,
) {
    for (label, mut text) in labels.iter_mut() {
        let shown = g.max_affordable(label.idx, amount.0).max(1);
        let cost = g.bulk_cost(label.idx, shown);
        let owned = g.owned[label.idx];
        **text = Text::from_section(
            format!("x{shown}\n{}\n({owned})", cost.format_short()),
            TextStyle {
                font_size: 13.0,
                color: COL_GOLD,
                ..default()
            },
        );
    }
    for (btn, mut bg) in btns.iter_mut() {
        let afford = g.max_affordable(btn.idx, amount.0) >= 1;
        **bg = BackgroundColor(if afford { COL_BTN_OK } else { COL_BTN_OFF });
    }
}

/// Цены/статус кнопок кошачьего магазина
fn cat_shop_buttons_system(
    g: Res<game_state::GameState>,
    mut labels: Query<(&CatBuyLabel, &mut Text)>,
    mut btns: Query<(&CatBuyButton, &mut BackgroundColor)>,
) {
    for (label, mut text) in labels.iter_mut() {
        let price = g.cat_upgrade_price(label.idx);
        let s = if g.cat_upgrades_bought[label.idx] {
            "\u{2714}".to_string()
        } else {
            format!("{price} \u{1f408}")
        };
        **text = Text::from_section(
            s,
            TextStyle {
                font_size: 14.0,
                color: COL_PINK,
                ..default()
            },
        );
    }
    for (btn, mut bg) in btns.iter_mut() {
        let ok = !g.cat_upgrades_bought[btn.idx] && g.cats >= g.cat_upgrade_price(btn.idx);
        **bg = BackgroundColor(if ok { COL_BTN_OK } else { COL_BTN_OFF });
    }
}

/// Подпись кнопки перерождения: сколько дам / сколько осталось копить
fn prestige_button_system(
    g: Res<game_state::GameState>,
    mut info: Query<&mut Text, With<PrestigeInfoText>>,
) {
    let gain = g.prestige_gain();
    let style_big = TextStyle {
        font_size: 17.0,
        color: COL_TEXT,
        ..default()
    };
    **info.single_mut() = if gain >= 1 {
        Text::from_section(format!("ПЕРЕРОДИТЬСЯ \u{2192} +{gain} \u{1f408}"), style_big)
    } else {
        let need = PRESTIGE_THRESHOLD.sub_saturating(g.total_paws_run);
        let eta = if g.total_rate().is_zero() {
            "\u{2014}".to_string()
        } else {
            format!("{:.0} сек", need.ratio(g.total_rate()))
        };
        Text::from_section(
            format!(
                "\u{1f300} ПЕРЕРОЖДЕНИЕ: ещё {} лапок ({eta})",
                need.format_short()
            ),
            style_big,
        )
    };
}

/// Все покупки и перерождение по кликам на кнопки
fn buy_click_system(
    mut g: ResMut<game_state::GameState>,
    amount: Res<BuyAmount>,
    mut buy_q: Query<(&mut Interaction, &BuyButton)>,
    mut catbuy_q: Query<(&mut Interaction, &CatBuyButton)>,
    mut pres_q: Query<&mut Interaction, With<PrestigeButton>>,
    mut reset_q: Query<&mut Interaction, With<ResetButton>>,
) {
    for (mut inter, btn) in buy_q.iter_mut() {
        if *inter == Interaction::Pressed {
            *inter = Interaction::None;
            let n = g.max_affordable(btn.idx, amount.0);
            g.try_buy(btn.idx, n);
        }
    }
    for (mut inter, btn) in catbuy_q.iter_mut() {
        if *inter == Interaction::Pressed {
            *inter = Interaction::None;
            g.try_buy_cat_upgrade(btn.idx);
        }
    }
    for mut inter in pres_q.iter_mut() {
        if *inter == Interaction::Pressed {
            *inter = Interaction::None;
            g.do_prestige();
        }
    }
    for mut inter in reset_q.iter_mut() {
        if *inter == Interaction::Pressed {
            *inter = Interaction::None;
            g.hard_reset();
        }
    }
}

/// Отдельная система: применяет выбранный объём покупки x1/x10/x1000
fn amount_switch_system(
    mut amount: ResMut<BuyAmount>,
    mut clicks: Query<(&mut Interaction, &AmountBtn)>,
    mut bgs: Query<(&AmountBtn, &mut BackgroundColor)>,
) {
    let mut chosen: Option<u32> = None;
    for (mut inter, a) in clicks.iter_mut() {
        if *inter == Interaction::Pressed {
            *inter = Interaction::None;
            chosen = Some(a.0);
        }
    }
    if let Some(v) = chosen {
        amount.0 = v;
        for (btn, mut bg) in bgs.iter_mut() {
            **bg = BackgroundColor(if btn.0 == v { COL_BTN_OK } else { COL_BTN_OFF });
        }
    }
}

/// Автосохранение раз в 15 секунд
fn save_system(time: Res<Time>, mut timer: Local<f32>, g: Res<game_state::GameState>) {
    *timer += time.delta_secs();
    if *timer < 15.0 {
        return;
    }
    *timer = 0.0;
    save_now(&g);
}

fn save_now(g: &game_state::GameState) {
    if let Ok(json) = serde_json::to_string(g) {
        let _ = fs::write(SAVE_PATH, json);
    }
}

fn load_or_default() -> game_state::GameState {
    match fs::read_to_string(SAVE_PATH) {
        Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
        Err(_) => game_state::GameState::default(),
    }
}

fn main() {
    let state = load_or_default();
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "\u{1f43e} КОТОТАЙКУН: Лапки и Мемы".into(),
                resolution: (1280.0, 720.0).into(),
                present_mode: bevy::window::PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(state)
        .insert_resource(BuyAmount(1))
        .add_systems(Startup, startup)
        .add_systems(
            Update,
            (
                income_system,
                hover_system,
                click_system,
                animation_system,
                ui_text_system,
                shop_buttons_system,
                cat_shop_buttons_system,
                prestige_button_system,
                buy_click_system,
                amount_switch_system,
                save_system,
            )
                .chain(),
        )
        .run();
}
