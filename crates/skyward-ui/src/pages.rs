use crate::app::{
    Action, BackupAt, BackupEnd, BackupJob, CalcFace, CalcKey, CalcOp, Check, ClashChoice, ClockFace, Confirm, Damage, Data, Dock,
    DownloadEnd,
    EraseStop, Field, FileKind,
    FileOp, FileTask, Fold, Folder, Frame, FsAction, FsChoice, FsRow, Have, HeroArt, Hit, Install, JobEnd, Keys, MusicFace, MusicFilter, Pick, Place, Rows,
    Screen,
    SkywardCheck, SkywardEnd, SkywardJob, SkywardStage, Slider, Source, State, StoreFace, Switch, SystemsWork, Target, TasksFace, UsbDrive, UsbLink, Wipe, ADDABLE,
    BACKUP_PARTS, CLOCK_MOST, DATA_PARTITION, DEFAULT_TIMER_SOUND, DOWNLOADS, FASTBOOT_CHOICES, RULER_MINUTE_DP, SCREEN_OFF, TARGETS, TIME_ZONES,
    WIPES, installs, starting,
};
use crate::canvas::Canvas;
use crate::color::{mix, Rgba};
use crate::editor;
use crate::files;
use crate::fmt::{self, Verb};
use crate::font::Fonts;
use crate::geom::Rect;
use crate::lang::{same, tri, Lang, Text};
use crate::search;
use crate::term::{self, Term};
use crate::texts;
use crate::theme::Theme;
use crate::icons::{self, Icon};
use crate::widgets::{self, Caption, Detail, Fact, Outline, Row, Spec, SystemCard, SystemMark, Tone, Trail};
use skyward_info::{DeviceInfo, VERSION, version_text};
use skyward_install::{Outcome, Reflash, Refusal, Stage};
use skyward_net::download::Progress;
use skyward_net::supplicant::Security;
use skyward_net::wlan::{Bands, Network, Power, Stage as Joining};
use skyward_systems::{Held, Who};
use skyward_vab::{Slot, Standing};
use std::path::Path;

// The colours the stock Settings fills its icons' tiles with, the same in
// light and dark mode.
const BLUE: Rgba = Rgba::rgb(0x34, 0x82, 0xff);
const GREEN: Rgba = Rgba::rgb(0x1d, 0xcd, 0x3a);
const PURPLE: Rgba = Rgba::rgb(0x77, 0x67, 0xf9);
const SLATE: Rgba = Rgba::rgb(0x8c, 0x9d, 0xb0);
const ORANGE: Rgba = Rgba::rgb(0xff, 0x69, 0x00);
const RED: Rgba = Rgba::rgb(0xfa, 0x38, 0x2e);

/// What every page draws from: style, typefaces, the device readings, and
/// what the pages hold, from the choices made on them to where the data
/// partition stands.
pub struct Ctx<'a> {
    pub t: &'a Theme,
    pub f: &'a Fonts,
    pub info: &'a DeviceInfo,
    pub state: &'a State,
    /// The shell the terminal page shows, once one has started.
    pub term: Option<&'a Term>,
    /// The part of the page to draw; all of it if none is given. The page
    /// is laid out whole either way.
    pub band: Option<Band>,
    /// The time the page is drawn at: what the clock's counts read by.
    pub now: u64,
    /// The picture the desktop is drawn on, already filling the screen; a
    /// desktop without one is black.
    pub wallpaper: Option<&'a Canvas>,
}

/// Rows `rows` of a page, and of them the columns `cols`, each from the
/// first to one past the last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Band {
    pub rows: (u32, u32),
    pub cols: (u32, u32),
}

pub fn render(screen: Screen, cx: &Ctx) -> Frame {
    match screen {
        Screen::Decrypt => decrypt(cx),
        Screen::Desktop => desktop(cx),
        Screen::Calculator => calculator(cx),
        Screen::Clock => clock(cx),
        Screen::ClockSettings => clock_settings(cx),
        Screen::ClockSound => clock_sounds(cx),
        Screen::Music => music(cx),
        Screen::Tasks => tasks(cx),
        Screen::Store => store(cx),
        Screen::Home => home(cx),
        Screen::Settings => settings(cx),
        Screen::Personalization => personalization(cx),
        Screen::Personal(pick) => personal_pick(cx, pick),
        Screen::Reboot => reboot(cx),
        Screen::Wipe => wipe(cx),
        Screen::Mount => mount(cx),
        Screen::Install => install(cx),
        Screen::Choose(place) => choose(cx, place),
        Screen::Installing => installing(cx),
        Screen::About => about(cx),
        Screen::Specs => specs(cx),
        Screen::ScreenOff => screen_off(cx),
        Screen::Search => search(cx),
        Screen::Backup => backup(cx),
        Screen::Restore => restore(cx),
        Screen::RestoreBackup => restore_backup(cx),
        Screen::BackupJob => backup_job(cx),
        Screen::Advanced => advanced(cx),
        Screen::Files => files(cx),
        Screen::FileInfo => file_info(cx),
        Screen::Permissions => permissions(cx),
        Screen::Editor => editor(cx),
        Screen::Terminal => terminal(cx),
        Screen::Relabel => relabel(cx),
        Screen::Filesystems => filesystems(cx),
        Screen::Filesystem => filesystem(cx),
        Screen::PartWork => part_work(cx),
        Screen::Language => language(cx),
        Screen::TimeZone => time_zone(cx),
        Screen::Logs => logs(cx),
        Screen::Update => update(cx),
        Screen::SkywardUpdate => skyward_update(cx),
        Screen::License => license(cx),
        Screen::FontLicense => font_license(cx),
        Screen::Slot => slot(cx),
        Screen::Systems => systems(cx),
        Screen::Fastboot => fastboot(cx),
        Screen::Erase => erase(cx),
        Screen::CantStart => cant_start(cx),
        Screen::Unhandled => unhandled(cx),
        Screen::Rename => rename(cx),
        Screen::Clash => clash(cx),
        Screen::FileJob => file_job(cx),
        Screen::Wlan => wlan(cx),
        Screen::WlanPassword => wlan_password(cx),
        Screen::WlanNetwork => wlan_network(cx),
        Screen::WlanAdd => wlan_add(cx),
    }
}

/// Label and colour of each swipe-to-confirm control: the filesystem
/// page's as `s` says what it is to do.
pub fn confirm_label(which: Confirm, s: &State) -> (&'static str, bool) {
    use texts::confirm;
    let (label, danger) = match which {
        Confirm::Filesystem => match s.fs_action {
            FsAction::Repair => (confirm::REPAIR, true),
            FsAction::Remake(_) => (confirm::CHANGE, true),
        },
        Confirm::Reboot => (confirm::REBOOT, false),
        Confirm::Wipe => (confirm::WIPE, true),
        Confirm::Flash => (confirm::FLASH, true),
        Confirm::Backup => (confirm::BACK_UP, false),
        Confirm::Restore => (confirm::RESTORE, true),
        Confirm::Slot => (confirm::SWITCH, true),
        Confirm::FactoryReset => (confirm::FACTORY_RESET, true),
        Confirm::DeleteEntry | Confirm::DeleteBackup => (confirm::DELETE, true),
        Confirm::Install => (confirm::INSTALL, false),
        Confirm::InstallWipe => (confirm::INSTALL_AND_WIPE, true),
        Confirm::RunScript => (confirm::INSTALL, true),
        Confirm::Merge => (confirm::MERGE, true),
        Confirm::Relabel => (confirm::FIX, true),
        Confirm::SwitchSystem => (confirm::SWITCH_AND_REBOOT, false),
        Confirm::RemoveSystem => (confirm::REMOVE, true),
        Confirm::InstallSystem => (confirm::INSTALL, false),
    };
    (s.lang.pick(label), danger)
}

/// The home page's large title: the mode the recovery is in, not the name
/// of the build. The name is kept for the About page's wordmark alone.
pub const HOME_TITLE: Text = tri("恢复模式", "恢復模式", "Recovery");
pub const SEARCH_HINT: Text = tri("搜索功能与分区", "搜尋功能與分割區", "Search features and partitions");

/// The desktop's title, never shown large but kept for the page's records.
pub const DESKTOP_TITLE: Text = tri("桌面", "桌面", "Desktop");
/// The calculator's title, and the name of its icon on the desktop.
pub const CALC_TITLE: Text = tri("计算器", "計算機", "Calculator");
/// The clock's title, and the name of its icon on the desktop.
pub const CLOCK_TITLE: Text = tri("时钟", "時鐘", "Clock");

/// Where the phone can restart to, and what is said under the name of one
/// that wants it.
const REBOOT_TARGETS: [(Text, Option<Text>); 6] = [
    (texts::reboot::target::SYSTEM, None),
    (texts::reboot::target::RECOVERY, None),
    (same("Fastboot"), Some(texts::reboot::target::FASTBOOT_NOTE)),
    (same("Bootloader"), None),
    (same("EDL"), Some(texts::reboot::target::EDL_NOTE)),
    (texts::reboot::target::POWER_OFF, None),
];

/// The rows and words only this build has, which the languages' files have
/// yet to learn: the personalisation page and its pickers.
const PERSONALIZATION: Text = tri("个性化选项", "個人化選項", "Personalization");
const RINGTONE: Text = tri("铃声", "鈴聲", "Ringtone");
const SOUND_EFFECTS: Text = tri("通知声音", "通知聲音", "Notification sounds");
const WALLPAPER_TITLE: Text = tri("壁纸", "桌布", "Wallpaper");
const DESK_WALLPAPER: Text = tri("桌面壁纸", "桌面桌布", "Desktop wallpaper");
const LOCK_WALLPAPER: Text = tri("锁屏壁纸", "鎖定畫面桌布", "Lock screen wallpaper");
/// The note under the sound pickers, which only list: there is no sound
/// layer to play them with yet.
const SOUND_PICK_NOTE: Text = tri(
    "还不能播放，这里只列出可以选的名字，正在使用的已打勾。",
    "還不能播放，這裡只列出可以選的名字，正在使用的已打勾。",
    "Sound cannot play yet; this only lists the names there are, the one in use ticked.",
);
/// The note under the wallpaper pickers, where a row answers a touch.
const WALLPAPER_PICK_NOTE: Text = tri(
    "点一下即可更换，正在使用的已打勾。",
    "點一下即可更換，正在使用的已打勾。",
    "Tap one to change it; the one in use is ticked.",
);
/// A canvas for a page `h` tall in the colour `bg`, holding the rows `cx`
/// asks to be drawn.
fn sheet(cx: &Ctx, h: u32, bg: Rgba) -> Canvas {
    let w = cx.t.m.w;
    let Band { rows, cols } = cx.band.unwrap_or(Band { rows: (0, h), cols: (0, w) });
    Canvas::band(w, h, rows.0..rows.1, cols.0..cols.1, bg)
}

fn page(cx: &Ctx) -> Canvas {
    sheet(cx, cx.t.m.h, cx.t.pal.bg)
}

/// A canvas tall enough for a scrolling page; [`fit`] trims it to the content.
fn long_page(cx: &Ctx) -> Canvas {
    sheet(cx, cx.t.m.h * 3, cx.t.pal.bg)
}

/// How tall a long page whose content ends at `bottom` is: to the bottom
/// margin after it, and never shorter than one screen.
fn fitted(t: &Theme, bottom: f32) -> u32 {
    ((bottom + t.m.margin * 2.0).ceil() as u32).max(t.m.h)
}

/// Trim a long page to its content plus the bottom margin, and never shorter
/// than one screen.
fn fit(c: Canvas, t: &Theme, bottom: f32) -> Canvas {
    c.crop_height(fitted(t, bottom))
}

fn hits(rects: &[Rect], actions: &[Action]) -> Vec<Hit> {
    rects.iter().zip(actions).map(|(&rect, &action)| Hit { rect, action }).collect()
}

/// What a touch on a desktop icon does.
#[derive(Clone, Copy)]
enum Opens {
    /// The page it opens.
    Page(Screen),
    /// Nothing yet: the app is named on the desktop and not built, so a touch
    /// stops the system where it stands and the panel says which app it
    /// stopped for. The word is the one the panel looks the app up by.
    NotYet(&'static str),
}

/// One icon on Project Aether's desktop: the name under it, the glyph on its
/// tile, the colour of that tile, and what a touch on it does.
struct DeskApp {
    name: Text,
    icon: Icon,
    color: Rgba,
    opens: Opens,
}

/// The desktop's icons, laid out four to a row in the order written here.
const DESKTOP_APPS: [DeskApp; 9] = [
    DeskApp {
        name: tri("恢复模式", "恢復模式", "Recovery"),
        icon: Icon::Gear,
        color: Rgba::rgb(0x33, 0x82, 0xf5),
        opens: Opens::Page(Screen::Home),
    },
    DeskApp {
        name: tri("文件管理", "檔案管理", "Files"),
        icon: Icon::Folder,
        color: Rgba::rgb(0xf2, 0x9d, 0x22),
        opens: Opens::Page(Screen::Files),
    },
    DeskApp {
        name: tri("时钟", "時鐘", "Clock"),
        icon: Icon::Clock,
        color: Rgba::rgb(0x11, 0xb8, 0x81),
        opens: Opens::Page(Screen::Clock),
    },
    DeskApp {
        name: tri("计算器", "計算機", "Calculator"),
        icon: Icon::Calculator,
        color: Rgba::rgb(0x67, 0x6e, 0x7a),
        opens: Opens::Page(Screen::Calculator),
    },
    // The player and the tasks page stand pages of their own, empty for now:
    // they open rather than stopping the system, so the way in is settled
    // before there is anything to hear or to read. The browser and the
    // weather page are still nothing but their icons, and a touch on either
    // stops the system with the code for an app that is not built, so the
    // icon answers rather than doing nothing — the panel that catches the
    // stop says which it was, in words.
    DeskApp {
        name: tri("音乐", "音樂", "Music"),
        icon: Icon::Music,
        color: Rgba::rgb(0xf2, 0x6b, 0x3a),
        opens: Opens::Page(Screen::Music),
    },
    DeskApp {
        name: tri("浏览器", "瀏覽器", "Browser"),
        icon: Icon::Browser,
        color: Rgba::rgb(0x1f, 0xa8, 0xb8),
        opens: Opens::NotYet("browser"),
    },
    DeskApp {
        name: tri("任务管理器", "工作管理員", "Tasks"),
        icon: Icon::Tasks,
        color: Rgba::rgb(0x7a, 0x5c, 0xf0),
        opens: Opens::Page(Screen::Tasks),
    },
    // The weather page is the same kind of nothing for now, and answers the
    // same way, so that a tap on it is never silence.
    DeskApp {
        name: tri("天气", "天氣", "Weather"),
        icon: Icon::Weather,
        color: Rgba::rgb(0x2c, 0xa6, 0xdd),
        opens: Opens::NotYet("weather"),
    },
    // The store stands a page of its own with three faces and nothing on
    // them yet, as the tasks page does: the way in is settled before there
    // is anything to list. Its tile is the one gold on the desktop, so it
    // is found by its colour before its name is read.
    DeskApp {
        name: tri("Aether 商店", "Aether 商店", "Aether Store"),
        icon: Icon::Store,
        color: Rgba::rgb(0xd9, 0xa2, 0x1b),
        opens: Opens::Page(Screen::Store),
    },
];

/// How many icons one row of the desktop holds.
const DESKTOP_COLS: usize = 4;

/// How many rows of icons one page of the desktop holds. A page is what a
/// screen shows, not what it scrolls to: the icons past it wait on the page
/// to the right, and the desktop turns sideways to reach them.
const DESKTOP_ROWS: usize = 6;

/// How many icons one page of the desktop holds.
const DESKTOP_PER_PAGE: usize = DESKTOP_COLS * DESKTOP_ROWS;

/// How wide a tile is, in dp, and the space kept between two of them. The
/// tile is the square an app's icon is drawn on, and the square a page
/// opened from it grows out of; a page with no icon to come from — the
/// desktop's own pages opened some other way — starts from a square of the
/// same width at the middle of the screen.
pub const DESKTOP_TILE_DP: f32 = 62.5;
const DESKTOP_GAP_DP: f32 = 34.2;

/// How many pages of icons the desktop has, never fewer than the one that
/// is always showing.
pub fn desk_pages() -> usize {
    DESKTOP_APPS.len().div_ceil(DESKTOP_PER_PAGE).max(1)
}

/// The apps the desktop names and cannot open yet, by the word the panel
/// looks them up by.
///
/// Written out here and nowhere else: the words are what a stop note carries
/// to a panel that has a line for each, so a word this returns and the panel
/// does not know is a reason lost on the one screen that needed it.
pub fn not_built() -> Vec<&'static str> {
    DESKTOP_APPS
        .iter()
        .filter_map(|app| match app.opens {
            Opens::NotYet(tag) => Some(tag),
            Opens::Page(_) => None,
        })
        .collect()
}

/// The desktop's wallpaper: black whichever theme is on, since a desktop
/// carries a picture of its own rather than a page of the system.
const DESKTOP_BG: Rgba = Rgba::rgb(0, 0, 0);

/// The ink of the status bar over that wallpaper: the light set, taken from
/// the dark theme's, because the page under it never turns light. Without
/// this the light theme's grey bar would sit unread on the black.
pub const DESKTOP_STATUS: Rgba = Rgba::rgb(224, 224, 224);

/// The simple desktop Project Aether starts on: a black wallpaper with the
/// apps laid over it in a grid, four to a row, as a phone's desktop lays its
/// own out. The status bar is drawn over the desktop as it is over every
/// page, so the time, the battery and the WLAN stay in view.
///
/// A screen holds one page of icons. More than fit wait on the page to the
/// right: the desktop turns sideways, the way a phone's own does, rather
/// than running on downwards. A row of dots at the foot says how many pages
/// there are, and which one is showing.
pub fn desktop(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let m = &t.m;
    // The wallpaper: the picture read at startup, filling the screen, or
    // plain black where the ramdisk carries none.
    let mut c = match cx.wallpaper {
        Some(picture) if (picture.w, picture.h) == (m.w, m.h) => picture.clone(),
        Some(picture) => Canvas::covering(picture, m.w, m.h),
        None => Canvas::filled(m.w, m.h, DESKTOP_BG),
    };
    let mut out = Vec::new();
    // Where each app's icon stands, for the page it opens to grow out of
    // that square and go back into it. The square is the tile's own top and
    // sides, not the whole cell the touch answers.
    let mut launches = Vec::new();
    // Every tile, and what touching it does, whether it opens a page or has
    // nothing behind it yet: a finger on the bar darkens the one it holds.
    let mut tiles = Vec::new();

    // The grid, measured off a phone's own desktop at 1080 wide: a tile
    // 164 px square with 90 px between tiles, so four tiles and their gaps
    // come to 925 px and leave 77 px at each side; a row is 275 px apart,
    // tile and name together, so the gap between one row's name and the
    // next row's tile is what a phone leaves rather than the tile's own
    // height carried down.
    let tile = m.dp(DESKTOP_TILE_DP);
    let gap = m.dp(DESKTOP_GAP_DP);
    let row_w = tile * DESKTOP_COLS as f32 + gap * (DESKTOP_COLS - 1) as f32;
    let margin_x = (m.w as f32 - row_w) / 2.0;
    let row_h = m.dp(104.8);
    // The first row clears the status bar, but not by much: a phone's own
    // desktop puts its first row of tiles close under the bar rather than
    // leaving a band of empty wallpaper above it. `bar_bottom` is where a
    // page with a title puts its content, and that clearance is for a title
    // the desktop does not have, so the row starts inside it.
    let top = widgets::bar_bottom(t) - m.dp(14.0);
    let ink = Rgba::rgb(255, 255, 255);

    // Which page is showing, and how far a finger has pulled it towards the
    // one beside it. The pull is what a drag leaves in the state; at rest
    // it is nought.
    let pages = desk_pages();
    let page = cx.state.desk_page.min(pages - 1);
    let shift = cx.state.desk_drag;
    let first = page * DESKTOP_PER_PAGE;
    let last = (first + DESKTOP_PER_PAGE).min(DESKTOP_APPS.len());

    for (at, app) in DESKTOP_APPS[first..last].iter().enumerate() {
        let col = (at % DESKTOP_COLS) as f32;
        let row = (at / DESKTOP_COLS) as f32;
        let x = margin_x + tile / 2.0 + col * (tile + gap) + shift;
        let y = top + row * row_h;
        let cy = y + tile / 2.0;
        let (x0, y0) = (x - tile / 2.0, cy - tile / 2.0);
        // A tile holding the glyph, its name under it. The tile is the very
        // one a row's icon sits on, so the desktop and the pages under it
        // agree on what a tile looks like; the colour grades from a lit top
        // to a deeper bottom, so a flat square reads as a key that catches
        // the light.
        let (lit, deep) = (mix(app.color, ink, 0.14), mix(app.color, DESKTOP_BG, 0.10));
        icons::tile_graded(&mut c, x0, y0, tile, lit, deep);
        icons::glyph(&mut c, app.icon, x, cy, tile * 0.64, ink);
        c.text_center(&f.label, l.pick(app.name), x, cy + tile / 2.0 + m.dp(12.5), m.dp(12.3), ink);
        // The whole cell answers a touch, as a phone's desktop icon does —
        // including the ones with nothing behind them, which answer by
        // stopping rather than by opening.
        let square = Rect::new(x0, y0, tile, tile);
        let action = match app.opens {
            Opens::Page(screen) => {
                launches.push((screen, square));
                Action::Open(screen)
            }
            Opens::NotYet(tag) => Action::NotReady(tag),
        };
        tiles.push((action, square));
        out.push(Hit { rect: Rect::new(x - (tile + gap) / 2.0, y, tile + gap, row_h), action });
    }

    // A dot a page, and the one showing lit. One page alone needs none: a
    // single dot says nothing the screen does not already say.
    if pages > 1 {
        let gap = m.dp(7.0);
        let r = m.dp(2.5);
        let y = m.h as f32 - m.dp(16.0);
        let mut x = (m.w as f32 - gap * (pages - 1) as f32) / 2.0;
        for p in 0..pages {
            let dot = if p == page { ink } else { ink.with_alpha(88) };
            c.fill_circle(x, y, r, dot);
            x += gap;
        }
    }

    let mut frame = Frame { back: false, header: false, bg: DESKTOP_BG, ..Frame::new(t, c, out, l.pick(DESKTOP_TITLE), 0.0) };
    frame.launches = launches;
    frame.tiles = tiles;
    frame
}

/// The size of the words an app names its faces with across its top, and
/// the gap between two of them, in dp.
const TAB_PX: f32 = 22.5;
const TAB_GAP: f32 = 25.0;

/// The words across the top of an app's page — the faces it switches
/// between — centred on the page and set on the line the round button at the
/// page's right end stands on, so the words and that button read as one band
/// across the top rather than as two things at two heights.
///
/// The word shown is in the page's ink and the others quiet. Each answers a
/// touch with `action`, with a little room around it.
fn face_tabs<T: Copy + PartialEq>(
    c: &mut Canvas,
    t: &Theme,
    f: &Fonts,
    l: Lang,
    faces: &[(T, Text)],
    shown: T,
    action: fn(T) -> Action,
) -> Vec<Hit> {
    let m = &t.m;
    let px = m.dp(TAB_PX);
    let gap = m.dp(TAB_GAP);
    let words: Vec<f32> = faces.iter().map(|(_, name)| c.text_width(&f.heading, l.pick(*name), px)).collect();
    let strip = words.iter().sum::<f32>() + gap * (words.len() - 1) as f32;
    let mut x = (m.w as f32 - strip) / 2.0;
    // The words are set by the middle of their ink, on the middle of the
    // button's disc: a word set by its baseline instead would stand a
    // typeface's own height above that line, which is what made the words
    // and the button look like two rows rather than one.
    let cy = widgets::button_disc(t).1;
    let pad = m.dp(10.0);
    let mut hits = Vec::with_capacity(faces.len());
    for (k, (face, name)) in faces.iter().enumerate() {
        let ink = if *face == shown { t.pal.text } else { t.pal.subtitle };
        c.text_vc(&f.heading, l.pick(*name), x, cy, px, ink, None);
        // The word answers a touch, with a little room around it.
        let rect = Rect::new(x - pad, cy - px / 2.0 - pad, words[k] + pad * 2.0, px + pad * 2.0);
        hits.push(Hit { rect, action: action(*face) });
        x += words[k] + gap;
    }
    hits
}

/// The player's two faces, in the order it names them across its top: the
/// songs, and the one being played.
const MUSIC_FACES: [(MusicFace, Text); 2] = [(MusicFace::List, tri("列表", "清單", "Songs")), (MusicFace::Play, tri("播放", "播放", "Playing"))];

/// The four shelves the player's list can look at, in the order the page
/// names them under its two faces: every song at once, and the three ways
/// songs are kept. Each carries what the list says while it is empty, none
/// of the four having anything on it yet.
const MUSIC_FILTERS: [(MusicFilter, Text, Text); 4] = [
    (MusicFilter::All, tri("全部", "全部", "All"), tri("还没有歌曲", "還沒有歌曲", "No songs yet")),
    (MusicFilter::Album, tri("专辑", "專輯", "Albums"), tri("还没有专辑", "還沒有專輯", "No albums yet")),
    (MusicFilter::Artist, tri("艺术家", "藝術家", "Artists"), tri("还没有艺术家", "還沒有藝術家", "No artists yet")),
    (MusicFilter::Folder, tri("文件夹", "資料夾", "Folders"), tri("还没有文件夹", "還沒有資料夾", "No folders yet")),
];

/// The row of shelves under the two faces, copied from the stock recorder's
/// own list: four pills of one width filling the page between its margins,
/// the one shown filled with the page's grey and the others left the card's
/// white, its word in the page's ink where it is filled and quiet beside it.
///
/// Their size is the stock one's — 42 dp tall, a 13 dp gap, and as wide as
/// the page's width less its margins and those gaps split four ways. Every
/// pill answers a touch; none of the four is filled, so a touch turns to the
/// shelf it names and the list says there is nothing on it yet.
fn music_filters(c: &mut Canvas, t: &Theme, f: &Fonts, l: Lang, shown: MusicFilter) -> Vec<Hit> {
    let m = &t.m;
    let px = m.dp(15.3);
    let h = m.dp(42.0);
    let gap = m.dp(13.0);
    // Clear of the two faces' words, which stand on the gear's line, and far
    // enough below them that the row reads as the head of the list rather
    // than as part of the band of words.
    let y = m.dp(132.0);
    let count = MUSIC_FILTERS.len() as f32;
    let w = (m.w as f32 - m.margin * 2.0 - gap * (count - 1.0)) / count;

    let mut x = m.margin;
    let mut hits = Vec::new();
    for (filter, name, _) in MUSIC_FILTERS {
        let word = l.pick(name);
        let on = filter == shown;
        let (fill, ink) = if on { (t.pal.switch_off, t.pal.text) } else { (t.pal.card, t.pal.subtitle) };
        c.fill_round_rect(x, y, w, h, h / 2.0, fill);
        c.text_center(&f.body, word, x + w / 2.0, y + h / 2.0, px, ink);
        hits.push(Hit { rect: Rect::new(x, y, w, h), action: Action::MusicFilter(filter) });
        x += w + gap;
    }
    hits
}

/// The player the desktop opens: the songs as a list, and the one being
/// played with its cover, its name and the controls that play it. Neither
/// face has anything behind it yet — there are no songs and nothing comes out
/// of the speaker — so both say where they stand rather than standing empty.
///
/// It keeps the shape the clock and the calculator take: the two words
/// across the top, no `<` and no large title, and a finger dragged level
/// across the page turning to the face beside the one shown.
pub fn music(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let mut out = Vec::new();

    // The two words, the face shown in the page's ink and the other quiet,
    // centred over what that face puts below them.
    out.extend(face_tabs(&mut c, t, f, l, &MUSIC_FACES, cx.state.music_face, Action::MusicFace));

    match cx.state.music_face {
        MusicFace::List => {
            out.extend(music_filters(&mut c, t, f, l, cx.state.music_filter));
            // What the shelf shown says while there is nothing on it. None of
            // the four has anything yet: the songs are not read off the phone,
            // and the shelves are the way in for when they are.
            let at = MUSIC_FILTERS.iter().position(|(filter, ..)| *filter == cx.state.music_filter).unwrap_or(0);
            let (_, _, empty) = MUSIC_FILTERS[at];
            c.text_center(&f.body, l.pick(empty), m.w as f32 / 2.0, m.h as f32 / 2.0, m.sub_px, t.pal.subtitle);
        }
        MusicFace::Play => music_play(&mut c, t, f, l),
    }

    // No `<` and no large title: an app opened from the desktop leaves it the
    // way the desktop's own apps do, and the two words across the top name it.
    // The button at the right end is the clock's own settings gear, so the
    // four apps carry one band of controls; the player has no settings page
    // yet, so it answers a touch with nothing.
    Frame {
        back: false,
        header: false,
        bg: t.pal.bg,
        button: Some((widgets::Glyph::Gear, Action::Idle)),
        button_still: true,
        ..Frame::new(t, c, out, "", m.content_top)
    }
}

/// The player's playing face: the cover of the song, its name under it, the
/// line the count runs along and the key that sets it going. There is no song
/// yet — no cover to show, no count to run and nothing to set going — so the
/// cover is the player's own mark, the name says as much, and the key is
/// quiet. The places are laid out all the same: what will fill them is a song
/// and a sound, not another arrangement.
fn music_play(c: &mut Canvas, t: &Theme, f: &Fonts, l: Lang) {
    let m = &t.m;
    let mid = m.w as f32 / 2.0;
    // The cover: a square the width of the page less its margins, standing
    // high enough that the name and the keys under it have room to breathe.
    let cover = m.w as f32 - m.dp(120.0);
    let top = m.h as f32 * 0.225;
    let x = mid - cover / 2.0;
    c.shadow_round_rect(x, top, cover, cover, m.dp(28.0), m.dp(16.0), t.pal.shadow);
    c.fill_round_rect(x, top, cover, cover, m.dp(28.0), t.pal.card);
    icons::glyph(c, Icon::Music, mid, top + cover / 2.0, cover * 0.46, t.pal.chevron);

    // The name of what is playing and, under it, the act that would start it:
    // the two lines a song's cover carries once there is one.
    let said = l.among("没有在放的歌", "沒有在放的歌", "Nothing playing");
    c.text_center(&f.heading, said, mid, top + cover + m.dp(46.0), m.dp(20.0), t.pal.text);
    let ask = l.among("去列表点一首", "去清單點一首", "Pick one from the list");
    c.text_center(&f.label, ask, mid, top + cover + m.dp(76.0), m.sub_px, t.pal.subtitle);

    // The line the count runs along, with nothing run along it yet.
    let bar_y = top + cover + m.dp(114.0);
    c.fill_round_rect(x, bar_y, cover, m.dp(3.0), m.dp(1.5), t.pal.track);
    let base = bar_y + m.dp(26.0);
    c.text(&f.label, "00:00", x, base, m.sub_px, t.pal.subtitle, None);
    let end_w = c.text_width(&f.label, "00:00", m.sub_px);
    c.text(&f.label, "00:00", x + cover - end_w, base, m.sub_px, t.pal.subtitle, None);

    // The key that would set it going: the system's round chip with a
    // triangle on it, quiet while there is nothing to play.
    let cy = bar_y + m.dp(88.0);
    let r = m.dp(31.0);
    let (art, ox, oy) = widgets::chip_at(t, mid, cy, r);
    c.lay(&art, ox, oy, 1.0);
    icons::glyph(c, Icon::Play, mid, cy, r * 0.84, t.pal.chevron);
}

/// The tasks page's three faces, in the order it names them across its top:
/// the processes the machine runs, the services it keeps and the performance
/// it runs them at.
const TASKS_FACES: [(TasksFace, Text); 3] = [
    (TasksFace::Process, tri("进程", "進程", "Processes")),
    (TasksFace::Service, tri("服务", "服務", "Services")),
    (TasksFace::Performance, tri("性能", "效能", "Performance")),
];

/// The tasks page the desktop opens: the processes as a list, the services
/// the machine keeps and the performance with the figures it will draw.
/// None of the three has anything behind it yet — nothing is read off the
/// phone — so each says where it stands rather than standing empty, as the
/// player's list does.
///
/// It keeps the shape the clock, the calculator and the player take: the
/// words across the top, no `<` and no large title, the gear at the top right
/// and a finger dragged level across the page stepping to the face beside
/// the one shown.
pub fn tasks(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let mut out = Vec::new();

    // The words, the face shown in the page's ink and the others quiet,
    // centred over what each face puts below them.
    out.extend(face_tabs(&mut c, t, f, l, &TASKS_FACES, cx.state.tasks_face, Action::TasksFace));

    let said = match cx.state.tasks_face {
        TasksFace::Process => l.among("还没有读取进程", "還沒有讀取進程", "Processes are not read yet"),
        TasksFace::Service => l.among("还没有读取服务", "還沒有讀取服務", "Services are not read yet"),
        TasksFace::Performance => l.among("还没有读数", "還沒有讀數", "Nothing is measured yet"),
    };
    c.text_center(&f.body, said, m.w as f32 / 2.0, m.h as f32 / 2.0, m.sub_px, t.pal.subtitle);

    // No `<` and no large title: an app opened from the desktop leaves it the
    // way the desktop's own apps do, and the words across the top name it.
    // The button at the right end is the clock's own gear, so the four apps
    // carry one band of controls; this page has no settings behind it yet,
    // so it answers a touch with nothing.
    Frame {
        back: false,
        header: false,
        bg: t.pal.bg,
        button: Some((widgets::Glyph::Gear, Action::Idle)),
        button_still: true,
        ..Frame::new(t, c, out, "", m.content_top)
    }
}

/// The Aether store's three faces, in the order it names them across its
/// top: what it puts in front, everything it lists, and what the phone
/// already has.
const STORE_FACES: [(StoreFace, Text); 3] = [
    (StoreFace::Home, tri("首页", "首頁", "Home")),
    (StoreFace::All, tri("全部", "全部", "All")),
    (StoreFace::Mine, tri("我的", "我的", "Mine")),
];

/// The Aether store the desktop opens: what it puts in front, all it lists
/// and the apps the phone already has. None of the three has a shelf behind
/// it yet — nothing is fetched — so each says where it stands rather than
/// standing empty, as the tasks page does.
///
/// It keeps the shape the clock, the calculator, the player and the tasks
/// page take: the words across the top, no `<` and no large title, the gear
/// at the top right and a finger dragged level across the page stepping to
/// the face beside the one shown.
pub fn store(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let mut out = Vec::new();

    out.extend(face_tabs(&mut c, t, f, l, &STORE_FACES, cx.state.store_face, Action::StoreFace));

    let said = match cx.state.store_face {
        StoreFace::Home => l.among("还没有推荐", "還沒有推薦", "Nothing is recommended yet"),
        StoreFace::All => l.among("还没有应用", "還沒有應用", "No apps are listed yet"),
        StoreFace::Mine => l.among("还没有安装", "還沒有安裝", "Nothing is installed yet"),
    };
    c.text_center(&f.body, said, m.w as f32 / 2.0, m.h as f32 / 2.0, m.sub_px, t.pal.subtitle);

    // As on the tasks page: no `<` and no large title, the clock's own gear
    // at the right end, and nothing behind it yet.
    Frame {
        back: false,
        header: false,
        bg: t.pal.bg,
        button: Some((widgets::Glyph::Gear, Action::Idle)),
        button_still: true,
        ..Frame::new(t, c, out, "", m.content_top)
    }
}

/// How many keys a calculator's row holds, and how many rows it holds.
const CALC_COLS: usize = 4;

/// The calculator's own orange: the one colour on the page that is not the
/// theme's. A phone's calculator sets the keys that change the sum and the
/// operators apart with it, and leaves the figures black on white.
const CALC_ORANGE: Rgba = Rgba::rgb(255, 105, 0);

/// The two words across the top of the page. The left one is the sum being
/// worked; the right one is the page that converts units, which is named
/// and not built — it says so where the keys would be. Either answers a
/// touch, and a finger dragged level across the page turns to the other, the
/// way the clock's two words and the clock's page do.
const CALC_FACES: [(CalcFace, Text); 2] =
    [(CalcFace::Sum, tri("计算", "計算", "Calculate")), (CalcFace::Convert, tri("换算", "換算", "Convert"))];

/// The calculator's keys, in the order a calculator lays them out, with
/// `=` where the right hand of the last row ends. The left of the last row
/// is the key that flips a number's sign, as this one does instead of the
/// phone's own key that shrinks the calculator to a window.
const CALC_KEYS: [[CalcKey; CALC_COLS]; 5] = [
    [CalcKey::Clear, CalcKey::Erase, CalcKey::Percent, CalcKey::Op(CalcOp::Div)],
    [CalcKey::Digit(7), CalcKey::Digit(8), CalcKey::Digit(9), CalcKey::Op(CalcOp::Mul)],
    [CalcKey::Digit(4), CalcKey::Digit(5), CalcKey::Digit(6), CalcKey::Op(CalcOp::Sub)],
    [CalcKey::Digit(1), CalcKey::Digit(2), CalcKey::Digit(3), CalcKey::Op(CalcOp::Add)],
    [CalcKey::Negate, CalcKey::Digit(0), CalcKey::Dot, CalcKey::Eq],
];

const CALC_DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

/// What a key reads. The signs are the ones every calculator uses, so they
/// are the same in every language. Erase is the left arrow, since no font
/// here draws the erase-to-the-left sign.
fn calc_text(key: CalcKey) -> &'static str {
    match key {
        CalcKey::Digit(d) => CALC_DIGITS[d as usize],
        CalcKey::Dot => ".",
        CalcKey::Negate => "±",
        CalcKey::Percent => "%",
        CalcKey::Op(CalcOp::Add) => "+",
        CalcKey::Op(CalcOp::Sub) => "−",
        CalcKey::Op(CalcOp::Mul) => "×",
        CalcKey::Op(CalcOp::Div) => "÷",
        CalcKey::Eq => "=",
        CalcKey::Clear => "AC",
        CalcKey::Erase => "←",
    }
}

/// The tile behind a key: every key is a card, white over the grey page, and
/// only `=` is filled with the orange, so it reads as the key the sum is
/// answered with.
fn calc_fill(t: &Theme, key: CalcKey) -> Rgba {
    match key {
        CalcKey::Eq => CALC_ORANGE,
        _ => t.pal.card,
    }
}

/// The words on a key: the figures in the page's ink, `=` in white over its
/// orange, and everything else in the orange the phone's calculator sets its
/// operators in.
fn calc_ink(t: &Theme, key: CalcKey) -> Rgba {
    match key {
        CalcKey::Eq => Rgba::rgb(255, 255, 255),
        CalcKey::Digit(_) | CalcKey::Dot => t.pal.text,
        _ => CALC_ORANGE,
    }
}

/// How large a key's word is drawn. The phone's calculator sets `AC` small
/// enough for its two letters to sit in a key and the operators widest of
/// all; its typeface draws `+` and `÷` at the height of a figure, where this
/// one draws them shorter, so the signs are given that height back here.
fn calc_px(t: &Theme, key: CalcKey) -> f32 {
    let dp = |v: f32| t.m.dp(v);
    match key {
        CalcKey::Digit(_) | CalcKey::Dot => dp(30.5),
        CalcKey::Clear => dp(21.0),
        CalcKey::Op(_) => dp(45.7),
        // The phone draws a key here that fills the tile; the sign that
        // stands in its place is set large enough to carry the same weight.
        CalcKey::Negate => dp(36.0),
        _ => dp(27.4),
    }
}

/// The calculator: the two words across its top, the sum over the keys and
/// the keys four to a row, measured off the phone's own calculator at 1080
/// wide — keys 205 px square in a 921 px block, 33 px apart across and 29 px
/// down, the last row 122 px above the foot of the screen.
pub fn calculator(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let m = &t.m;
    // The phone's calculator greys its page a shade past the theme's, which
    // is what makes the white keys read as keys rather than patches.
    let ground = if t.dark { t.pal.bg } else { Rgba::rgb(247, 247, 247) };
    let mut c = sheet(cx, m.h, ground);
    let mut out = Vec::new();

    // The two words, the face being shown in the page's ink and the other
    // quiet, centred over what that face puts below them.
    out.extend(face_tabs(&mut c, t, f, l, &CALC_FACES, cx.state.calc_face, Action::CalcFace));

    // The units page is named and not built: it says so where the figures
    // and the keys would stand, and takes no key of its own.
    if cx.state.calc_face == CalcFace::Convert {
        let said = l.among("换算还没做好", "換算還沒做好", "Conversion is not built yet");
        c.text_center(&f.body, said, m.w as f32 / 2.0, m.h as f32 / 2.0, m.sub_px, t.pal.subtitle);
        return Frame {
            back: false,
            header: false,
            bg: ground,
            button: Some((widgets::Glyph::Gear, Action::Idle)),
            button_still: true,
            ..Frame::new(t, c, out, "", m.content_top)
        };
    }

    let left = m.dp(30.3);
    let right = m.w as f32 - left;
    let avail = right - left;

    // The keys take the foot of the page, the last row down to the foot
    // margin, and the sum sits over them at the right.
    let gap = m.dp(12.6);
    let row_gap = m.dp(11.0);
    let rows = CALC_KEYS.len() as f32;
    let key_h = m.dp(64.0);
    let grid_h = key_h * rows + row_gap * (rows - 1.0);
    let top = m.h as f32 - m.dp(46.5) - grid_h;
    let key_w = (avail - gap * (CALC_COLS as f32 - 1.0)) / CALC_COLS as f32;

    // The sum so far, right against the caret the phone keeps after it, its
    // figures shrunk to fit the line where there are many of them. The gap
    // the typeface leaves at the right of its last figure is taken up here,
    // so the sum ends where the phone's does.
    let caret_w = m.dp(2.3);
    let caret_x = right - m.dp(1.2);
    let ink_x = caret_x + m.dp(2.2);
    let shown = cx.state.calc.shown();
    let mut px = m.dp(50.3);
    let mut wide = c.text_width(&f.heading, &shown, px);
    while wide > ink_x - left && px > m.dp(20.0) {
        px -= m.dp(2.0);
        wide = c.text_width(&f.heading, &shown, px);
    }
    let base = top - m.dp(65.2);
    c.text(&f.heading, &shown, ink_x - wide, base, px, t.pal.text, None);
    // The caret is as tall as a line of the phone's figures, standing well
    // past the digits on either side of the baseline.
    let caret_h = m.dp(98.3);
    let caret_cy = top - m.dp(75.1);
    c.fill_rect(caret_x, caret_cy - caret_h / 2.0, caret_w, caret_h, CALC_ORANGE);

    for (r, row) in CALC_KEYS.iter().enumerate() {
        for (k, key) in row.iter().enumerate() {
            let x = left + (key_w + gap) * k as f32;
            let y = top + (key_h + row_gap) * r as f32;
            c.fill_round_rect(x, y, key_w, key_h, m.dp(16.0), calc_fill(t, *key));
            c.text_center(&f.heading, calc_text(*key), x + key_w / 2.0, y + key_h / 2.0, calc_px(t, *key), calc_ink(t, *key));
            // The whole key answers a touch.
            out.push(Hit { rect: Rect::new(x, y, key_w, key_h), action: Action::Calc(*key) });
        }
    }

    // No `<` and no large title here: an app opened from the desktop leaves
    // it the way the desktop's own apps do, by a swipe up from the foot of
    // the screen, and the calculator's top line carries its two words
    // instead of a page's name. The button at its right end is the clock's
    // settings gear; the calculator has no settings page yet, so it answers
    // a touch with nothing.
    Frame {
        back: false,
        header: false,
        bg: ground,
        button: Some((widgets::Glyph::Gear, Action::Idle)),
        button_still: true,
        ..Frame::new(t, c, out, "", m.content_top)
    }
}

/// The clock's two faces, in the order it names them across its top.
const CLOCK_FACES: [(ClockFace, Text); 2] =
    [(ClockFace::Stopwatch, tri("秒表", "碼錶", "Stopwatch")), (ClockFace::Timer, tri("计时", "計時", "Timer"))];

/// The pictures on the clock's keys, on both its faces. Neither face names
/// them: what a key does is told by the picture, and which pictures are shown
/// changes with what the watch is doing.
enum Glyph {
    /// A flag: take an entry on the paper.
    Lap,
    /// Two bars: stop the count where it is.
    Pause,
    /// A ring turning backwards: put the watch back to nothing and throw the
    /// paper.
    Reset,
    /// A triangle: set the count going.
    Start,
    /// A bar: take a minute off the timer's length. Kept for when the timer
    /// can be set again, once its ruler carries the slide.
    #[allow(dead_code)]
    Minus,
    /// Two bars crossed: put a minute on the timer's length. Kept for the
    /// same reason as `Minus`.
    #[allow(dead_code)]
    Plus,
}

/// Draw one of those pictures, `h` tall and centred on (`cx`, `cy`), in `ink`.
fn clock_glyph(c: &mut Canvas, glyph: Glyph, cx: f32, cy: f32, h: f32, ink: Rgba) {
    // The pictures are `icons::Icon`s, drawn by the outline-and-raster code
    // every other glyph on the device goes through — so a key's ink has the
    // same weight and the same edges as the chevrons and tiles round it,
    // instead of edges of a second kind sitting beside them. `h` is the box
    // all four are scaled to fill; see the scales in `icons`.
    let icon = match glyph {
        Glyph::Lap => Icon::Flag,
        Glyph::Pause => Icon::Pause,
        Glyph::Reset => Icon::Replay,
        Glyph::Start => Icon::Play,
        Glyph::Minus => Icon::Minus,
        Glyph::Plus => Icon::Plus,
    };
    // The reset arrow is the one picture here that is wide rather than tall —
    // 28 units by 12.33 — so at a key's box it would stand a third of the
    // height of the pictures beside it. Its box is made as many times wider,
    // until the arrow stands half their height and half the chip's width:
    // the width is what the eye weighs, and a picture that turns left to
    // right across the whole chip reads as a key of its own rather than as
    // the same key turning back.
    //
    // It is also the one picture whose weight is not where its box's middle
    // is: the head at the lower left carries more ink than the arc over it,
    // so a box centred on the chip leaves the arrow reading low. It is lifted
    // by a key's twentieth — about a dp — until what the eye puts in the
    // middle of the chip and the chip's own middle agree.
    let (size, lift) =
        if matches!(glyph, Glyph::Reset) { (h * RESET_BOX, h * RESET_LIFT) } else { (h, 0.0) };
    icons::glyph(c, icon, cx, cy - lift, size, ink);
}

/// How far up the reset arrow is lifted: a share of a key's box.
const RESET_LIFT: f32 = 0.055;

/// The reset arrow's box as a share of a key's: the drawing is 2.27 times as
/// wide as it is tall, so a box of the same shape that leaves the arrow half
/// a key's height is 2.27 × 0.51 of a key's box.
const RESET_BOX: f32 = 1.15;

/// The ink the pictures on the stopwatch's keys are drawn in: the grey the
/// watch's own keys carry, a shade lighter than the page's black. On a pale
/// chip a black triangle reads heavier than the key it sits on; the face it
/// is drawn after inks them grey for the same reason. In the dark the same
/// grey would sink into the chip, so it is lifted until it stands out of it.
fn clock_ink(t: &Theme) -> Rgba {
    if t.dark {
        Rgba::rgb(200, 200, 200)
    } else {
        Rgba::rgb(68, 68, 68)
    }
}

/// The radius of the disc the clock's keys sit on, in dp: the very disc every
/// other page draws its `<` on (`widgets::back_disc` is the same 21.7), so a
/// key here reads as one of the system's round controls instead of a plain
/// circle of its own. Both faces put their keys on it.
const KEY_R: f32 = 21.7;

/// One of the clock's keys: that chip, with a picture on it. A key that
/// cannot be used where the watch stands — a minute's end of the timer's
/// length, or either of the two while it runs — is drawn all the same, its
/// picture only quieted, so the row does not move under a finger that has
/// just reached it. The chip is laid whole every time: the bar's own fades in
/// with the bar as the page scrolls under it, but a key at the foot of the
/// page has no scrolling to wait for, so it is never half shown.
fn clock_key(c: &mut Canvas, t: &Theme, cx: f32, cy: f32, glyph: Glyph, on: bool) -> Rect {
    let r = t.m.dp(KEY_R);
    let (art, ox, oy) = widgets::chip_at(t, cx, cy, r);
    c.lay(&art, ox, oy, 1.0);
    let ink = if on { clock_ink(t) } else { t.pal.chevron };
    clock_glyph(c, glyph, cx, cy, r * 0.84, ink);
    Rect::new(cx - r, cy - r, r * 2.0, r * 2.0)
}

/// A count of milliseconds as a stopwatch reads it: `MM:SS.cc`, and
/// `H:MM:SS.cc` once it has run an hour. It stops at 999 hours, 59
/// minutes, 59 seconds and 99 hundredths, as Smartisan's does.
fn stopwatch_text(ms: u64) -> String {
    let cs = ms / 10 % 100;
    let s = ms / 1_000 % 60;
    let m = ms / 60_000;
    if m >= 60 {
        format!("{}:{:02}:{:02}.{:02}", m / 60, m % 60, s, cs)
    } else {
        format!("{m:02}:{s:02}.{cs:02}")
    }
}

/// What a timer has left, rounded up to the second it is shown by, so a
/// timer with a moment left reads `00:01` rather than nothing. Past an hour
/// it takes the hour place, as the stopwatch's reading does.
fn timer_text(ms: u64) -> String {
    let s = ms.div_ceil(1_000);
    let (h, m, s) = (s / 3_600, s / 60 % 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// The timer's ruler: a rounded bar above the keys, with the scale running
/// from zero to three hours under a fixed mark in its middle. Everything is
/// drawn here — the white patch that ends at zero, the dots that lead to it,
/// the fine marks, the minute figures and the mark — so no picture is read
/// for any of them. The mark is dark green rather than the reference's red.
const RULER_W: f32 = 252.0;
const RULER_H: f32 = 50.0;
const RULER_R: f32 = 5.0;
/// Air between the ruler's foot and the tops of the keys, in dp.
const RULER_GAP: f32 = 12.0;
/// The bar's two greens and its edge, sampled from the reference. The ruler
/// keeps these in both themes.
const RULER_GREEN_TOP: Rgba = Rgba::rgb(230, 235, 231);
const RULER_GREEN_FOOT: Rgba = Rgba::rgb(240, 242, 239);
const RULER_WHITE: Rgba = Rgba::rgb(250, 250, 250);
const RULER_EDGE: Rgba = Rgba::rgb(200, 205, 199);
/// The marks and figures, and the fixed line that chooses between them. The
/// line is a shade deeper than the marks, as a pointer has to be.
const RULER_INK: Rgba = Rgba::rgb(120, 149, 120);
const RULER_MARK: Rgba = Rgba::rgb(76, 112, 76);
/// The tick artwork, laid out to the phone's own ruler: ten strokes to the
/// minute, one every six seconds, and every fifth stroke half again as
/// tall, so a long mark stands on the minute and on the half minute both,
/// the way Smartisan spaces them. It hangs from the bar's top edge — the
/// first row of the picture is the first row of a stroke.
///
/// **The picture is drawn at the size it is shown at**, which is what
/// keeps the strokes crisp: a minute is 24 dp, so on fuxi's 420 dpi it is
/// 63 px, and that is the scale the artwork is drawn in. Laid out at some
/// larger size and shrunk by the phone, six columns of antialiasing are
/// squeezed into one and a third and a stroke turns into a grey smear —
/// which is exactly what it did at the old 210 px to the minute. The
/// lengths are a shade over the reference's own (a long one 17 of the
/// bar's 131 px against its 16, a short one 11 against 10), and the line
/// 1.5 px, so a stroke still holds its weight once it lands.
const RULER_DECO_PIC: &[u8] = include_bytes!("../../../resources/clock/rulerticks.png");
/// One minute of the artwork, in its own pixels. 24 dp at 420 dpi, so
/// the picture goes down on the phone at one to one; on a denser panel
/// it is scaled up rather than down, which keeps its strokes whole.
const RULER_DECO_MINUTE_PX: f32 = 63.0;
/// The artwork already starts at the bar's top edge, so the scale begins
/// at the picture's own first pixel.
const RULER_TICK_TOP: f32 = 0.0;
const RULER_NUM_PX: f32 = 10.5;
/// The figures sit a little under halfway down the bar, clear of the
/// strokes overhead: with the scale hanging from the top edge, the band
/// from the strokes' feet to the bar's foot is where they belong.
const RULER_NUM_CY: f32 = 0.28;
/// In the dark the bar takes its colours from the CPU graph in Windows'
/// Task Manager on a dark theme, which is the same problem solved: a sunken
/// panel of deep teal, gridwork one step off it, and a bright cyan stroke
/// laid on top. The scale becomes that panel, the ticks and figures become
/// the stroke, and the choosing line goes one shade brighter than the
/// stroke so it still reads as the heaviest mark on the bar. The empty
/// patch keeps its role — the part of the scale not yet used — by sinking
/// to the card grey the same window puts behind a list entry, a step above
/// the page's black rather than a white glaring out of it.
const RULER_BLUE_TOP: Rgba = Rgba::rgb(12, 40, 49);
const RULER_BLUE_FOOT: Rgba = Rgba::rgb(15, 49, 59);
/// In the dark the bar carries no frame of its own: the panel sinks straight
/// into the black page and its rounded corners say where it ends. A line
/// there — even one as deep as the panel's own top — reads as a blue ring
/// standing off the page, which is the one thing a sunken panel must not
/// look like.
const RULER_BLUE_EDGE: Rgba = Rgba::rgb(0, 0, 0);
const RULER_BLUE_INK: Rgba = Rgba::rgb(57, 184, 227);
const RULER_BLUE_MARK: Rgba = Rgba::rgb(120, 214, 240);
const RULER_DARK_PATCH: Rgba = Rgba::rgb(38, 38, 38);

/// The tick artwork, once per theme. Every pixel is recoloured to that
/// theme's ruler ink, transparent ones included: bilinear scaling samples
/// their RGB at a stroke's edge, and leaving them black gives the ticks a
/// black fringe. The alpha stays as it was, and the picture is kept whole
/// so the empty stretch that ends it keeps one pair of minutes off the
/// next.
fn ruler_ticks(dark: bool) -> &'static Canvas {
    static LIGHT: std::sync::OnceLock<Canvas> = std::sync::OnceLock::new();
    static DARK: std::sync::OnceLock<Canvas> = std::sync::OnceLock::new();
    let slot = if dark { &DARK } else { &LIGHT };
    slot.get_or_init(|| {
        let ink = if dark { RULER_BLUE_INK } else { RULER_INK };
        let mut ticks = Canvas::from_png(RULER_DECO_PIC).expect("the ruler tick picture must decode");
        for px in ticks.data.as_chunks_mut::<4>().0 {
            px[0] = ink.r;
            px[1] = ink.g;
            px[2] = ink.b;
        }
        ticks
    })
}

/// Draw the ruler centred on (`cx`, `cy`). `value` is where the fixed mark
/// falls, in minutes; the zero of the scale moves with it, and the white
/// patch always reaches from the left edge to that zero. The ticks come
/// from Smartisan's own artwork; the figures stay in the page's type. The
/// returned rect is the part of the page a finger may drag.
///
/// The bar is drawn on a canvas of its own and cut to its rounded frame
/// before it goes down. The scale runs past both ends as it rolls, so it is
/// the frame, not the scale, that says where the scale stops showing — and
/// the corners stay round however far a stroke or a figure overruns them.
#[allow(clippy::too_many_arguments)]
fn timer_ruler(
    c: &mut Canvas, m: &crate::theme::Metrics, f: &Fonts, cx: f32, cy: f32, rw: f32, rh: f32, value: f32, dark: bool,
) -> Rect {
    let (green_top, green_foot, edge, ink, mark, patch) = if dark {
        (RULER_BLUE_TOP, RULER_BLUE_FOOT, RULER_BLUE_EDGE, RULER_BLUE_INK, RULER_BLUE_MARK, RULER_DARK_PATCH)
    } else {
        (RULER_GREEN_TOP, RULER_GREEN_FOOT, RULER_EDGE, RULER_INK, RULER_MARK, RULER_WHITE)
    };
    let (x, y) = (cx - rw / 2.0, cy - rh / 2.0);
    let r = m.dp(RULER_R);
    let mut bar = Canvas::new(rw.ceil() as u32, rh.ceil() as u32);
    bar.fill_round_rect(0.0, 0.0, rw, rh, r, edge);
    let (ix, iy) = (1.0, 1.0);
    let (iw, ih) = (rw - 2.0, rh - 2.0);
    bar.fill_round_rect_graded(ix, iy, iw, ih, (r - 1.0).max(0.0), &|t| mix(green_top, green_foot, t));

    // The rest is placed against the bar's own left edge, half its width
    // from the fixed mark.
    let mid = rw / 2.0;

    // The left patch is tied to the zero mark: at the start it fills the
    // half before the fixed line, and as the scale rolls on it narrows until
    // zero passes out of sight. Its right edge stays a straight cut.
    let minute = m.dp(RULER_MINUTE_DP);
    let white_end = (mid - value * minute).clamp(ix, ix + iw);
    let white_w = white_end - ix;
    if white_w > 0.0 {
        let wr = (r - 1.0).min(white_w / 2.0).min(ih / 2.0);
        bar.fill_round_rect(ix, iy, white_w, ih, wr, patch);
        if white_w > wr {
            // Square off the right end with the frame's own coverage: a
            // plain rect would fill its last row whole and paint over the
            // one-pixel line the frame keeps along the foot.
            bar.fill_round_rect(ix + wr, iy, white_w - wr, ih, 0.0, patch);
        }
        // Three marks, as the reference draws them: a solid left-pointing
        // triangle with three dots behind it, the first tip 35 dp in from
        // the patch's edge and a mark every 27.6 dp. A mark that would run
        // past the patch is left out whole, so a narrowing patch drops the
        // far ones instead of spilling into the scale.
        let lead = ix + m.dp(35.0);
        let group = m.dp(27.6);
        let arrow = m.dp(4.1);
        let half = m.dp(1.75);
        let dot = m.dp(1.0);
        let dot_gap = m.dp(5.0);
        let dot_cy = iy + ih / 2.0;
        for g in 0..3 {
            let tip = lead + g as f32 * group;
            let back = tip + arrow;
            if back + dot_gap * 3.0 + dot > ix + white_w - m.dp(1.0) {
                break;
            }
            bar.fill_convex(&[(tip, dot_cy), (back, dot_cy - half), (back, dot_cy + half)], ink);
            for k in 1..=3 {
                bar.fill_circle(back + k as f32 * dot_gap, dot_cy, dot, ink);
            }
        }
    }

    // The tick artwork belongs to the scale, so it rolls with it. Draw it
    // into a strip that starts at the zero edge: that keeps every stroke
    // off the white patch without needing a clip, and lets the picture's
    // own spacing decide how fine the marks are. The strip begins as the
    // very pixels the bar holds there, so no tint of ours shows behind the
    // strokes; a see-through one would have the picture's soft edge
    // multiplied once by `draw_image` and again by `lay`, which paints it
    // black.
    let deco = ruler_ticks(dark);
    let tick_h = deco.h as f32 * minute / RULER_DECO_MINUTE_PX;
    let top = iy + m.dp(RULER_TICK_TOP);
    let green_x = white_end.max(ix);
    let green_w = (ix + iw - green_x).max(0.0);
    if green_w > 0.0 && tick_h > 0.0 {
        let sx = green_x.round().max(0.0) as u32;
        let sy = top.round().max(0.0) as u32;
        let sw = (green_w.ceil() as u32).min(bar.w.saturating_sub(sx));
        let sh = (tick_h.ceil() as u32).min(bar.h.saturating_sub(sy));
        if sw > 0 && sh > 0 {
            let mut strip = bar.crop(sx, sy, sw, sh);
            let first_pair = ((value + (green_x - mid) / minute) / 2.0).floor() as i32 - 2;
            let last_pair = ((value + (green_x + green_w - mid) / minute) / 2.0).ceil() as i32 + 2;
            for pair in first_pair..=last_pair {
                let even = pair * 2;
                if even < 0 || even >= CLOCK_MOST as i32 {
                    continue;
                }
                let at = mid + (even as f32 - value) * minute;
                strip.draw_image(deco, at - sx as f32, 0.0, 2.0 * minute, tick_h);
            }
            bar.lay(&strip, sx as i32, sy as i32, 1.0);
        }
    }

    // The figures stay in the page's type, just below the ticks where
    // Smartisan puts them, rather than pressed against the bar's foot.
    for n in 0..=CLOCK_MOST as u32 {
        let px = mid + (n as f32 - value) * minute;
        if px < white_end || px > ix + iw {
            continue;
        }
        bar.text_center(&f.label, &n.to_string(), px, iy + ih * RULER_NUM_CY, m.dp(RULER_NUM_PX), ink);
    }

    // The mark itself is the one thing on the ruler that never moves.
    bar.line(mid, iy, mid, iy + ih, 1.5, mark);

    // The frame is the mask: a figure whose glyph runs over an end, strokes
    // rolling out of a corner, anything the scale wrote past the rounded
    // outline is gone before the bar is laid down.
    //
    // The foot line goes back first. The panel and the patch both reach the
    // frame's last row, which paints over the line the edge put there: the
    // patch is the one that shows, and a strip of white along the foot reads
    // as the bar sitting on its own edge. One row of the edge's colour is
    // laid again, and the mask rounds its ends with the rest of the frame.
    bar.fill_rect(0.0, rh - 1.0, rw, 1.0, edge);
    bar.mask_round(0.0, 0.0, rw, rh, r);
    c.lay(&bar, x.round() as i32, y.round() as i32, 1.0);
    Rect::new(x, y, rw, rh)
}

/// The words under the dial before the watch has ever run.
const CLOCK_HINT: Text = tri("点击启动秒表", "點擊啟動碼錶", "Tap to start the stopwatch");

/// The words under the count before a length has been chosen on the ruler.
const TIMER_HINT: Text = tri("滑动尺子以计时", "滑動尺子以計時", "Slide the ruler to set a time");

/// The dial: a disc the colour of a card, its own shadow under it, drawn by
/// the page rather than laid on from a picture. On that disc sit the minute
/// ring from `clockdeck.png` and the two numerals in the page's own type. The
/// blue hand Xiaomi paints lies over them; its colour follows whether the
/// watch is going. The count under it carries the reading for now.
///
/// Across in dp. Smartisan's dial measures about 608 px over a 1080 panel
/// once its shadow is counted; with the shadow this page lays under it, 212 dp
/// brings our disc to the same span and the same place on the panel.
const DIAL_D: f32 = 212.0;
/// The dial's shadow: the disc stands on the page under a light from above,
/// so the shadow leaves its foot at its darkest and spreads `DIAL_SIGMA`
/// wide as it fades, thin at the sides and gone above the dial, where the
/// disc itself is between the light and the page. Measured against
/// Smartisan's panel, the page is 250 and the shadow takes the darkest of it
/// under the rim's foot to about 210, with nothing of it left 30 dp out.
const DIAL_SIGMA: f32 = 11.0;
const DIAL_INK: f32 = 0.28;

/// The minute ring, drawn once by the user at 570 px and kept as a picture
/// in the tree. Its marks run at radii 265..285 of a 285 half-width, so laid
/// out at the disc's own width they would crowd the rim: Smartisan's ring
/// sits at 0.85 of the disc's radius, and the scale below brings ours there.
/// Black at half alpha in the source, so it reads grey on any card colour.
const DECK_PIC: &[u8] = include_bytes!("../../../resources/clock/clockdeco.png");
/// The ring's mark centre, as a fraction of the picture's half width (275 of 285).
const DECK_MARK: f32 = 275.0 / 285.0;
/// Where Smartisan puts that ring, as a fraction of the disc's radius.
const DECK_OF_DIAL: f32 = 0.85;
/// The two numerals, at twelve and three. Smartisan sets their ink centre at
/// 0.70 of the disc radius; the size is the height of the glyphs in dp.
const DIAL_NUM_R: f32 = 0.70;
const DIAL_NUM_PX: f32 = 19.0;
const DIAL_NUMS: [&str; 2] = ["60", "15"];

/// The bail the stopwatch hangs from: the ring, the bar across it and the
/// post running down into the dial's top edge, cut out of Smartisan's own
/// store with the shadow already in it. Laid on as a picture rather than
/// built from the disc and the slab, so the light on it is Smartisan's and
/// not an imitation of it.
const BAIL_PIC: &[u8] = include_bytes!("../../../resources/clock/swmid.png");
/// The picture is 228 px across and its ring 182 px; the ring's centre sits
/// at (113.5, 104) in it, measured off the alpha.
const BAIL_RING_PX: f32 = 182.0;
const BAIL_RING_X_PX: f32 = 113.5;
const BAIL_RING_Y_PX: f32 = 104.0;
/// The ring across, in dp: Smartisan's measures 178 px on its 1080 panel,
/// which at our 2.63 px to the dp is 67.7. The dial here comes out a shade
/// smaller than Smartisan's, so the bail stands a little larger against it;
/// the ring is the piece the eye takes the size of, and it is left at the
/// size Smartisan draws it rather than scaled down with the dial.
const BAIL_RING_DP: f32 = 67.7;
/// How far above the dial's centre the ring hangs, in dial radii: 374 px on
/// a dial of 291. Measured to the ring's centre, so the post's foot lands
/// just inside the dial's top edge and the dial closes over it.
const BAIL_LIFT: f32 = 374.0 / 291.0;

/// The two pieces on the flanks — the crown and the button — cut from the
/// store as the bail is. Each picture is 206×200 px and is laid down as it
/// comes, in either palette; the right one is the left one mirrored, which
/// is what Smartisan's panel shows.
const KNOB_L_PIC: &[u8] = include_bytes!("../../../resources/clock/swleft.png");
const KNOB_R_PIC: &[u8] = include_bytes!("../../../resources/clock/swright.png");
/// On Smartisan's panel each picture's middle sits 1.053 dial radii out from
/// the dial's centre, at 45° up and out; the dial here is a shade smaller,
/// so the knobs are set on that same radius rather than at the panel's own
/// corners, which is what keeps their bite into the rim as deep as his. The
/// `X` is the left picture's corner, in dp.
const KNOB_X: f32 = 84.6;
const KNOB_Y: f32 = 215.1;
const KNOB_W: f32 = 78.6;
const KNOB_H: f32 = 76.3;

/// Loaded once for each palette and shared by every draw of the dial.
/// `from_png` expands the palette to RGBA and keeps the picture's own alpha.
///
/// The ring is drawn in black, which is what the pale dial wants. On the
/// dark one the disc is a shade off the black page, and black marks on it
/// are all but invisible: the same picture turned over is the ring in
/// light, which is how Smartisan's own dark clock sets it.
fn clock_deck(dark: bool) -> &'static Canvas {
    static LIGHT: std::sync::OnceLock<Canvas> = std::sync::OnceLock::new();
    static DARK: std::sync::OnceLock<Canvas> = std::sync::OnceLock::new();
    if dark {
        DARK.get_or_init(|| {
            let mut deck = Canvas::from_png(DECK_PIC).expect("the clock deck picture must decode");
            deck.invert();
            deck
        })
    } else {
        LIGHT.get_or_init(|| Canvas::from_png(DECK_PIC).expect("the clock deck picture must decode"))
    }
}

/// The bail's picture, decoded once and kept. It is laid on as Smartisan
/// cut it in both palettes: the light and the shadow on it are the store's
/// own, and turning it over for the dark page — as the minute ring is turned
/// over — would put a halo where the shadow is.
fn bail_pic() -> &'static Canvas {
    static PIC: std::sync::OnceLock<Canvas> = std::sync::OnceLock::new();
    PIC.get_or_init(|| Canvas::from_png(BAIL_PIC).expect("the bail picture must decode"))
}

/// The bail, laid on above the dial centred on `mid`: the ring hangs clear of
/// the dial and the post runs down behind its top edge, so this is drawn
/// before the dial and the dial closes over the post's foot.
fn clock_bail(c: &mut Canvas, m: &crate::theme::Metrics, mid: f32) {
    let pic = bail_pic();
    let scale = m.dp(BAIL_RING_DP) / BAIL_RING_PX;
    let cy = m.dp(DIAL_CY - DIAL_D / 2.0 * BAIL_LIFT);
    let (w, h) = (pic.w as f32 * scale, pic.h as f32 * scale);
    c.draw_image(pic, mid - BAIL_RING_X_PX * scale, cy - BAIL_RING_Y_PX * scale, w, h);
}

/// One of the two flank pieces, decoded once and kept: the left picture and
/// the right one, which is its mirror.
fn knob_pic(right: bool) -> &'static Canvas {
    static LEFT: std::sync::OnceLock<Canvas> = std::sync::OnceLock::new();
    static RIGHT: std::sync::OnceLock<Canvas> = std::sync::OnceLock::new();
    if right {
        RIGHT.get_or_init(|| Canvas::from_png(KNOB_R_PIC).expect("the right knob must decode"))
    } else {
        LEFT.get_or_init(|| Canvas::from_png(KNOB_L_PIC).expect("the left knob must decode"))
    }
}

/// The crown and the button on the flanks, laid on before the dial so its
/// rim closes over them: each is bitten into the dial's edge, and only the
/// part outside the rim shows.
fn clock_knobs(c: &mut Canvas, m: &crate::theme::Metrics) {
    let (w, h) = (m.dp(KNOB_W), m.dp(KNOB_H));
    let (x, y) = (m.dp(KNOB_X), m.dp(KNOB_Y));
    c.draw_image(knob_pic(false), x, y, w, h);
    c.draw_image(knob_pic(true), m.w as f32 - x - w, y, w, h);
}

/// The dial's still parts — shadow, disc, minute ring and the two numerals —
/// drawn once into a cache and laid down with a single `draw_image` each
/// frame. Only the hand changes as the watch runs, so nothing else on the
/// dial is paid for again. Keyed by width in pixels, density and theme, so
/// the one cache serves every page size and both palettes.
fn clock_face(m: &crate::theme::Metrics, t: &crate::theme::Theme, f: &Fonts) -> &'static Canvas {
    use std::collections::HashMap;
    type FaceCache = std::sync::Mutex<HashMap<(u32, u32, bool), &'static Canvas>>;
    static FACES: std::sync::OnceLock<FaceCache> = std::sync::OnceLock::new();
    let key = (m.w, m.h, t.dark);
    let faces = FACES.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    if let Some(face) = faces.lock().expect("no poison").get(&key) {
        return face;
    }
    let d = m.dp(DIAL_D) * FACE_SS;
    let pad = m.dp(DIAL_FACE_PAD) * FACE_SS;
    let side = (d + pad * 2.0).ceil() as u32;
    let mut face = Canvas::new(side, side);
    let cc = side as f32 / 2.0;
    // The dial is laid on the same disc every round control on the system
    // sits on: the chip's own fill, its bright hairline rim and its fade of
    // shadow, all four times the size. Read on its own the face is a plain
    // circle; made of that material it belongs to the keys under it.
    widgets::disc_lit(
        &mut face,
        t,
        cc,
        cc,
        d / 2.0,
        m.dp(DIAL_SIGMA) * FACE_SS,
        DIAL_INK,
    );
    let span = d * DECK_OF_DIAL / DECK_MARK;
    face.draw_image(clock_deck(t.dark), cc - span / 2.0, cc - span / 2.0, span, span);
    let r = d / 2.0 * DIAL_NUM_R;
    let px = m.dp(DIAL_NUM_PX) * FACE_SS;
    face.text_center(&f.regular, DIAL_NUMS[0], cc, cc - r, px, t.pal.text);
    face.text_center(&f.regular, DIAL_NUMS[1], cc + r, cc, px, t.pal.text);
    let face = if FACE_SS > 1.0 { face.shrink(FACE_SS as u32) } else { face };
    let face: &'static Canvas = Box::leak(Box::new(face));
    let mut faces = faces.lock().expect("no poison");
    faces.entry(key).or_insert(face)
}

/// The blue Xiaomi paints its second hand, sampled off the reference shot.
const HAND_BLUE: Rgba = Rgba::rgb(52, 130, 255);
/// The hand when the watch stands: a quiet charcoal, so the dial does not
/// claim attention until the count runs. The pin stays white either way.
const HAND_STILL: Rgba = Rgba::rgb(88, 92, 98);
/// Room left around the disc for its shadow to fade out: the fade's own
/// reach, plus a pixel of air. This is the margin the cached face is drawn
/// into.
const DIAL_FACE_PAD: f32 = DIAL_SIGMA * 3.0 + 2.0;
/// How many times over the face is drawn before it is halved down to the
/// panel. Every shape here -- the disc, its rim, the minute ring, the two
/// numerals -- smooths an edge across one pixel and no finer, which is as
/// much as a pixel can say; drawn twice the size and averaged back, the same
/// edge thins out across half a pixel and reads smooth when the panel is
/// looked at close. The face is drawn once and cached, so the doubling is
/// paid for once per size and palette rather than once a frame.
const FACE_SS: f32 = 2.0;
/// The fine needle, from the centre to its point. Opposite it a fine line
/// runs out from the hub before widening into the counterweight: Xiaomi
/// leaves about 30 px of thin line before the thick part starts on a 600 px
/// dial, so the thick stroke sits at the far end rather than at the centre.
/// All in dp.
const HAND_TIP: f32 = 82.0;
const HAND_TIP_W: f32 = 2.4;
const HAND_TAIL_GAP: f32 = 10.5;
const HAND_TAIL: f32 = 20.0;
const HAND_TAIL_W: f32 = 4.2;
/// The blue disc over the join, and the white pin on top that reads as the
/// fixed one the hand turns about.
const HAND_HUB: f32 = 7.0;
const HAND_PIN: f32 = 3.0;
/// The shadow the hand lays on the face, and how far below the hand it is
/// set: the light comes from above, so the shadow is a copy of the hand
/// straight under it. At twelve the two lie along each other and nothing of
/// the shadow shows; at six it stands clear of the hand's end, which is what
/// makes it grow as the hand turns back toward twelve and go as the hand
/// comes round.
///
/// It is laid soft rather than solid — the copy is blown out by `BLUR`
/// either side, so the mark it leaves has no edge of its own — and set far
/// enough down that its middle clears the hand's own edge: a copy laid right
/// under the rod just darkens the rod's outline, and it is the band standing
/// free of it that reads as a shadow the light cast. A rod a few dp across
/// casts a shadow with no edge; one with an edge reads as a second hand
/// lying beside the first.
const HAND_SHADOW: Rgba = Rgba::rgb(0, 0, 0).with_alpha(26);
const HAND_SHADOW_DROP: f32 = 3.4;
const HAND_SHADOW_BLUR: f32 = 4.2;

/// The counterweight a hand may carry behind its hub: the fine line that
/// runs on past the centre and the thicker stroke it widens into. The
/// stopwatch's needle has one; the timer's hour hand does not.
#[derive(Clone, Copy)]
struct Tail {
    gap: f32,
    len: f32,
    w: f32,
}

/// The measurements of one hand, so the same drawing serves both faces.
///
/// The stopwatch's is a fine needle. The timer's is that same hand drawn
/// thick and a little short, the way a watch draws its hour hand: it stands
/// for a whole minute's mark rather than a second's, so it is a stout rod
/// and not a hairline. Its rod ends at the hub — Smartisan's hour hand has
/// no line running on behind it, and a tail on a rod this thick reads as a
/// second, smaller hand — while the shadow, the pin and the way it turns
/// about are the stopwatch's own.
struct HandSpec {
    tip: f32,
    tip_w: f32,
    tail: Option<Tail>,
    hub: f32,
    shadow_drop: f32,
    shadow_blur: f32,
}

const SECOND_HAND: HandSpec = HandSpec {
    tip: HAND_TIP,
    tip_w: HAND_TIP_W,
    tail: Some(Tail { gap: HAND_TAIL_GAP, len: HAND_TAIL, w: HAND_TAIL_W }),
    hub: HAND_HUB,
    shadow_drop: HAND_SHADOW_DROP,
    shadow_blur: HAND_SHADOW_BLUR,
};

/// The timer's hour hand: a little short of the needle's reach and nearly
/// four times its width, its counterweight left off so the rod reads as one
/// piece. Its tip stops short of the minute ring, where the numbers are.
const HOUR_HAND: HandSpec = HandSpec {
    tip: 70.0,
    tip_w: 9.0,
    tail: None,
    hub: 10.0,
    shadow_drop: HAND_SHADOW_DROP,
    shadow_blur: 5.0,
};

/// Where the stopwatch's parts sit in dp: the dial's centre from the top of
/// the page, the count's centre and size under it, the short rule the paper
/// hangs from, and where the paper starts and how tall a row of it is. The
/// dial sits where Smartisan's sits on a 1080 panel; the count, rule and
/// paper follow it down by the space it gained, so the paper still clears
/// the keys and keeps its closing rule when it overflows.
pub(crate) const DIAL_CY: f32 = 329.0;
const READ_CY: f32 = 507.0;
const READ_PX: f32 = 30.5;
/// The count run past an hour, when `H:MM:SS.cc` needs the room.
const READ_LONG_PX: f32 = 24.0;
pub(crate) const RULE_CY: f32 = 550.0;
/// The rule before the watch has ever run: the short slip Smartisan draws,
/// 80 dp across and a hair thick, paler than the words under it.
const RULE_SHORT_W: f32 = 80.0;
/// The rule the paper hangs from, and the width of a row of it: as wide as
/// the entries themselves, which is where Smartisan stops it too.
const RULE_W: f32 = 160.0;
/// The rule's thickness. Smartisan draws it two pixels on its panel; at ours
/// that is three quarters of a dp.
const RULE_T: f32 = 0.75;
const HINT_CY: f32 = 566.0;
const HINT_PX: f32 = 11.5;
const PAPER_TOP: f32 = 560.0;
const PAPER_ROW: f32 = 30.0;

/// The foot of the stopwatch's paper: the top of the keys, less the gap the
/// rule under the paper is drawn in. Measured up from the foot of the
/// screen, so the paper — never the keys, never the count — is what gives
/// when a panel is shorter than the one the page was laid out on.
fn clock_paper_foot(m: &crate::theme::Metrics) -> f32 {
    m.h as f32 - m.margin - m.dp(26.0) - m.dp(74.0) / 2.0 - m.dp(KEY_R) - m.dp(20.0)
}

/// How many entries of the paper the room between the rule and the keys
/// takes: past that, the paper is scrolled through. A panel short enough
/// that not one entry fits gives none rather than drawing over the keys or
/// off the screen, and the rule goes with them.
pub fn clock_paper_room(m: &crate::theme::Metrics) -> usize {
    ((clock_paper_foot(m) - m.dp(PAPER_TOP)) / m.dp(PAPER_ROW)).floor().max(0.0) as usize
}

/// The dial's still face laid on centred at (`cx`, `cy`): the cache in one
/// blit, and nothing of the hand. Both hands are laid over it — the
/// stopwatch's needle by `clock_dial`, the timer's hour hand by
/// `timer_dial` — so what is shared is the face, not the hand.
fn clock_face_at(c: &mut Canvas, m: &crate::theme::Metrics, t: &crate::theme::Theme, f: &Fonts, cx: f32, cy: f32) {
    let face = clock_face(m, t, f);
    let side = face.w as f32;
    c.draw_image(face, cx - side / 2.0, cy - side / 2.0, side, side);
}

/// The dial laid on centred at (`cx`, `cy`): the still face comes from the
/// cache in one blit, then the hand is drawn over it. `deg` is the hand's
/// angle clockwise from twelve and `tint` is how blue it reads, 0 charcoal
/// to 1 Xiaomi blue.
#[allow(clippy::too_many_arguments)]
fn clock_dial(
    c: &mut Canvas, m: &crate::theme::Metrics, t: &crate::theme::Theme, f: &Fonts,
    cx: f32, cy: f32, deg: f32, tint: f32,
) {
    clock_bail(c, m, cx);
    clock_knobs(c, m);
    clock_face_at(c, m, t, f, cx, cy);

    // The hand stands off the face, so it lays a shadow on it: the dial is
    // lit from above and a little behind, which puts the shadow straight
    // under the hand. Under it means along it when the hand stands at
    // twelve — the shadow hides behind the hand there and is not seen at
    // all — and furthest out from it when the hand lies down at six, which
    // is the shadow growing as the hand turns, without anything having to
    // follow the turn.
    clock_hand_shadow(c, m, cx, cy, deg, &SECOND_HAND);
    clock_hand(c, m, cx, cy, deg, mix(HAND_STILL, HAND_BLUE, tint), &SECOND_HAND);
    c.fill_circle(cx, cy, m.dp(HAND_PIN) / 2.0, Rgba::rgb(255, 255, 255));
}

/// The timer's dial: the same still face and the same shadow and pin, with
/// the hour hand over it. It wears no bail and no knobs — the timer's dial
/// is the bare disc Smartisan's is, not the pocket watch the stopwatch
/// hangs from — so only the hand itself differs between the two faces.
#[allow(clippy::too_many_arguments)]
fn timer_dial(
    c: &mut Canvas, m: &crate::theme::Metrics, t: &crate::theme::Theme, f: &Fonts,
    cx: f32, cy: f32, deg: f32, tint: f32,
) {
    clock_face_at(c, m, t, f, cx, cy);
    clock_hand_shadow(c, m, cx, cy, deg, &HOUR_HAND);
    clock_hand(c, m, cx, cy, deg, mix(HAND_STILL, HAND_BLUE, tint), &HOUR_HAND);
    c.fill_circle(cx, cy, m.dp(HAND_PIN) / 2.0, Rgba::rgb(255, 255, 255));
}

/// One stroke of the hand: from (`x0`, `y0`) to (`x1`, `y1`), `w` across.
type Stroke = (f32, f32, f32, f32, f32);
/// The hub the hand turns about: its centre and its radius.
type Hub = (f32, f32, f32);

/// The hand's strokes — the one through the hub to its tip, and behind it,
/// when the spec carries a tail, the thin line run on past the hub and the
/// counterweight laid over its outer part — and the hub itself, turning
/// about (`cx`, `cy`).
fn hand_geom(m: &crate::theme::Metrics, cx: f32, cy: f32, deg: f32, s: &HandSpec) -> (Vec<Stroke>, Hub) {
    let a = deg * std::f32::consts::PI / 180.0;
    let (ux, uy) = (a.sin(), -a.cos());
    let tip = m.dp(s.tip);
    let mut strokes = vec![(cx, cy, cx + ux * tip, cy + uy * tip, m.dp(s.tip_w))];
    // A hand with no tail gets no strokes at all behind the hub, rather
    // than two of no length: a line drawn from the hub back to itself still
    // lays a dot there.
    if let Some(tail) = s.tail {
        let gap = m.dp(tail.gap);
        let end = gap + m.dp(tail.len);
        strokes.push((cx, cy, cx - ux * end, cy - uy * end, m.dp(s.tip_w)));
        strokes.push((
            cx - ux * gap,
            cy - uy * gap,
            cx - ux * end,
            cy - uy * end,
            m.dp(tail.w),
        ));
    }
    (strokes, (cx, cy, m.dp(s.hub) / 2.0))
}

/// The hand itself, in `ink`.
fn clock_hand(
    c: &mut Canvas, m: &crate::theme::Metrics, cx: f32, cy: f32, deg: f32, ink: Rgba, s: &HandSpec,
) {
    let (strokes, (hx, hy, hr)) = hand_geom(m, cx, cy, deg, s);
    for (x0, y0, x1, y1, w) in strokes {
        c.line(x0, y0, x1, y1, w, ink);
    }
    c.fill_circle(hx, hy, hr, ink);
}

/// The shadow of the hand, set down the way the light casts it and blown
/// out on either side, so what lands on the face is a soft smudge under the
/// hand rather than a second hand laid beside it.
fn clock_hand_shadow(c: &mut Canvas, m: &crate::theme::Metrics, cx: f32, cy: f32, deg: f32, s: &HandSpec) {
    let blur = m.dp(s.shadow_blur);
    let (strokes, (hx, hy, hr)) = hand_geom(m, cx, cy + m.dp(s.shadow_drop), deg, s);
    for (x0, y0, x1, y1, w) in strokes {
        c.soft_line(x0, y0, x1, y1, w, blur, HAND_SHADOW);
    }
    c.soft_disc(hx, hy, hr, blur, HAND_SHADOW);
}

/// The clock: the two words across the top that switch between the time
/// counted up and the time counted down, the reading large under them, and
/// the keys that work it at the foot. The stopwatch's entries are kept on a
/// slip under the reading, the newest at its foot as a till's paper comes
/// out.
pub fn clock(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let (m, clock) = (&t.m, &cx.state.clock);
    let mut c = page(cx);
    let mut out = Vec::new();

    let mid = m.w as f32 / 2.0;

    // The two words, the one being shown in the page's ink and the other
    // quiet, set as the calculator and the player set their own.
    out.extend(face_tabs(&mut c, t, f, l, &CLOCK_FACES, clock.face, Action::ClockFace));

    // The keys: a row of the same chips on both faces, one or two of them
    // under the stopwatch and three under the timer, centred on the page.
    // They stand on the same line as the three big keys the timer used to
    // have, so switching the face moves neither the buttons nor the paper
    // that hangs above them.
    let sd = m.dp(KEY_R * 2.0);
    let sgap = m.dp(32.0);
    let cy = m.h as f32 - m.margin - m.dp(26.0) - m.dp(74.0) / 2.0;
    // How high the chips reach, which is what the room above them ends at.
    let keys_top = cy - sd / 2.0;
    // Where the `k`th of `n` chips has its middle: the row's is the page's.
    let sat = |k: usize, n: usize| mid + (k as f32 - (n as f32 - 1.0) / 2.0) * (sd + sgap);

    // The stopwatch is read off a dial: a bare disc, the count small under
    // it, and the paper of entries hanging from a short rule under the count.
    // The timer has no dial, so its count takes the middle of the room the
    // words and the keys leave.
    let mut scroll = None;
    let mut ruler = None;
    let live = match clock.face {
        ClockFace::Stopwatch => {
            // The hand turns once a minute with the count and fades between
            // charcoal and blue as the watch starts, stops and settles. Its
            // angle and colour are both pure reads of the clock's own state.
            let deg = clock.hand_deg(cx.now);
            let tint = clock.hand_tint(cx.now);
            clock_dial(&mut c, m, t, f, mid, m.dp(DIAL_CY), deg, tint);
            let reading = stopwatch_text(clock.stopwatch(cx.now));
            // The typeface's own weight, not the one the pages set their
            // titles in: the count is read rather than looked at.
            let px = m.dp(if reading.chars().count() > 8 { READ_LONG_PX } else { READ_PX });
            let read_cy = m.dp(READ_CY);
            c.text_center(&f.regular, &reading, mid, read_cy, px, t.pal.text);

            // The rule the paper hangs from: short, so it reads as a slip of
            // its own rather than a division of the page. It belongs to the
            // paper, so it shows when there is one to show — under the short
            // words before the watch has run, and over the entries once it
            // has been worked. A watch running with nothing taken yet has
            // neither, and its page is the dial and the count alone.
            let rule_w = m.dp(RULE_W);
            let (rl, rr) = (mid - rule_w / 2.0, mid + rule_w / 2.0);
            // The rule is set on whole pixels: Smartisan's is two of them, and
            // one begun between two rows spreads over three and reads heavier.
            let rule_y = m.dp(RULE_CY).round();
            let rule_h = m.dp(RULE_T).round().max(1.0);

            let started = clock.stop_since.is_some() || clock.stop > 0 || !clock.laps.is_empty();
            let paper = m.dp(PAPER_TOP);
            // The paper stops short of the keys, and a rule under it closes
            // the room taken when there are more entries than it shows.
            let foot = clock_paper_foot(m);
            let row_h = m.dp(PAPER_ROW);
            let count = clock.laps.len();
            let room = clock_paper_room(m);
            if !started {
                // The short slip, centred where the paper's rule will stand:
                // it is the same rule, only as long as the words under it
                // and no longer, as Smartisan draws it.
                let short = m.dp(RULE_SHORT_W);
                c.fill_rect(mid - short / 2.0, rule_y, short, rule_h, t.pal.hairline);
                c.text_center(&f.label, l.pick(CLOCK_HINT), mid, m.dp(HINT_CY), m.dp(HINT_PX), t.pal.value);
            } else if count > 0 && room > 0 {
                c.fill_rect(rl, rule_y, rule_w, rule_h, t.pal.hairline);
                let shown = count.min(room);
                if count > room {
                    // What does not fit is scrolled through under the finger
                    // rather than cut off: the room taken, and the height of
                    // a row within it.
                    scroll = Some((Rect::new(rl, paper, rule_w, foot - paper), row_h));
                }
                // The entries run down from the rule, the oldest first, the
                // count kept at the left of the row and the split taken at
                // its right. Each row closes on a rule of its own — the last
                // one too, so the paper reads as ruled rather than open.
                // The figures are Smartisan's sizes: the total a step above
                // the count and the split, all of them within the rule the
                // row is drawn on.
                let first = clock.paper.min(count - shown);
                for k in 0..shown {
                    let lap = &clock.laps[first + k];
                    let y = paper + row_h * (k as f32 + 0.5);
                    let n = (first + k + 1).to_string();
                    c.text_vc(&f.label, &n, rl + m.dp(4.0), y, m.dp(11.0), t.pal.value, None);
                    let total = stopwatch_text(lap.total);
                    c.text_vc(&f.regular, &total, rl + m.dp(25.0), y, m.dp(13.5), t.pal.text, None);
                    let split = format!("+ {}", stopwatch_text(lap.split));
                    let w = c.text_width(&f.label, &split, m.dp(11.0));
                    c.text_vc(&f.label, &split, rr - m.dp(4.0) - w, y, m.dp(11.0), t.pal.value, None);
                    let close = (paper + row_h * (k as f32 + 1.0)).round();
                    c.fill_rect(rl, close, rule_w, rule_h, t.pal.hairline);
                }
            }
            // The hand and the count are the two things here that go on
            // changing while nothing is touched. One band from the dial's
            // top down past the reading covers both, so each frame lays the
            // cached face down once, turns the hand and sets the count.
            let dial_top = m.dp(DIAL_CY - DIAL_D / 2.0 - DIAL_FACE_PAD).max(0.0);
            Some((dial_top as u32, (read_cy + px).min(m.h as f32) as u32))
        }
        ClockFace::Timer => {
            // The same dial the stopwatch wears, laid where it sits there,
            // with the hour hand over it: a turn an hour, so the marks it
            // passes are minutes and the hand creeps across one rather than
            // sitting on it and then jumping. It stands at twelve before a
            // length is chosen, and a reset sweeps it back there beside the
            // ruler, so the two settle together.
            let deg = clock.timer_hand_deg(cx.now);
            let tint = clock.timer_hand_tint(cx.now);
            timer_dial(&mut c, m, t, f, mid, m.dp(DIAL_CY), deg, tint);
            // The timer's count takes the same place and size as the
            // stopwatch's: only what it says is different.
            let reading = timer_text(clock.timer(cx.now));
            let px = m.dp(if reading.chars().count() > 8 { READ_LONG_PX } else { READ_PX });
            let read_cy = m.dp(READ_CY);
            c.text_center(&f.regular, &reading, mid, read_cy, px, t.pal.text);

            // Before a length is chosen the page says how one is chosen, the
            // way the stopwatch says how it is started: the same short rule
            // under the count, with the words under that. Once the ruler has
            // set a length the pair goes, and neither the count running nor
            // one held at pause brings them back.
            if !clock.timer_set() {
                let short = m.dp(RULE_SHORT_W);
                let (at, thick) = (m.dp(RULE_CY).round(), m.dp(RULE_T).round().max(1.0));
                c.fill_rect(mid - short / 2.0, at, short, thick, t.pal.hairline);
                c.text_center(&f.label, l.pick(TIMER_HINT), mid, m.dp(HINT_CY), m.dp(HINT_PX), t.pal.value);
            }

            // The ruler: the scale sits just above the keys, with its
            // middle line fixed and its white patch ending at zero. It is
            // offered to a finger only while the timer is not running.
            let rw = m.dp(RULER_W);
            let rh = m.dp(RULER_H);
            let ruler_cy = keys_top - m.dp(RULER_GAP) - rh / 2.0;
            ruler = clock
                .timer_can_slide()
                .then_some(timer_ruler(&mut c, m, f, mid, ruler_cy, rw, rh, clock.ruler_value(cx.now), t.dark));

            // A reset carries the ruler home, so while it is on its way the
            // band drawn again a frame at a time takes in the bar as well as
            // the count; once it lands the count alone moves.
            // The hand and the count both change while the timer runs, so
            // the band drawn again a frame at a time has to reach the top of
            // the dial as well. Drawing the reading alone would leave the
            // hand standing wherever it was when the band was first taken.
            let dial_top = m.dp(DIAL_CY - DIAL_D / 2.0 - DIAL_FACE_PAD).max(0.0);
            let (mut top, mut bottom) = (dial_top, (read_cy + px).min(m.h as f32));
            if clock.ruler_sweeping(cx.now) {
                top = top.min(ruler_cy - rh / 2.0 - m.dp(2.0));
                bottom = bottom.max(ruler_cy + rh / 2.0 + m.dp(2.0));
            }
            Some((top as u32, bottom as u32))
        }
    };

    // The stopwatch's keys: one before the watch has ever run and two after.
    // The pair changes with the state — take an entry and stop it while it
    // goes, put it back to nothing and set it off again while it rests.
    if clock.face == ClockFace::Stopwatch {
        let going = clock.stop_since.is_some();
        // Nothing to reset and nothing to take an entry of before the first
        // run, so the watch offers the one key and keeps its paper shut.
        let started = going || clock.stop > 0 || !clock.laps.is_empty();
        if started {
            let pair: [(Glyph, Action); 2] = if going {
                [(Glyph::Lap, Action::ClockLap), (Glyph::Pause, Action::ClockGo)]
            } else {
                [(Glyph::Reset, Action::ClockReset), (Glyph::Start, Action::ClockGo)]
            };
            for (k, (glyph, action)) in pair.into_iter().enumerate() {
                let rect = clock_key(&mut c, t, sat(k, 2), cy, glyph, true);
                out.push(Hit { rect, action });
            }
        } else {
            // The one key is the same disc of the same size: it is the count
            // that has not started, not the key that is drawn differently.
            let rect = clock_key(&mut c, t, mid, cy, Glyph::Start, true);
            out.push(Hit { rect, action: Action::ClockGo });
        }
    }

    // The timer's keys: both stay where they are and both wait until a
    // minute has been chosen on the ruler. The right one then follows the
    // count — pause while it runs, start while it is held at pause.
    if clock.face == ClockFace::Timer {
        let ready = clock.timer_set();
        let going = clock.left_since.is_some();
        let keys: [(Glyph, Action); 2] = [
            (Glyph::Reset, Action::ClockReset),
            (if going { Glyph::Pause } else { Glyph::Start }, Action::ClockGo),
        ];
        for (k, (glyph, action)) in keys.into_iter().enumerate() {
            let rect = clock_key(&mut c, t, sat(k, 2), cy, glyph, ready);
            if ready {
                out.push(Hit { rect, action });
            }
        }
    }

    // No `<` here either: the clock leaves the way the desktop's own apps do.
    // No large title either: the two words are the top of the page, as they
    // are on the calculator.
    //
    // The gear at the page's top right is the bar's round button — the `<`
    // chip mirrored, at the same height. Its chip is laid as the `<`'s is,
    // fading in with the bar as a page scrolls under it; this page does not
    // scroll, so what stands there is the gear on its own, the way the
    // back button stands on a page held at its top.
    Frame {
        back: false,
        header: false,
        button: Some((widgets::Glyph::Gear, Action::Open(Screen::ClockSettings))),
        button_still: true,
        live,
        scrollback: scroll,
        clock_ruler: ruler,
        ..Frame::new(t, c, out, "", m.content_top)
    }
}

/// The clock's settings, opened by the gear at the page's top right: the
/// sound the timer's end makes, and whether the clock ticks.
const CLOCK_SETTINGS_TITLE: Text = tri("设置", "設定", "Settings");
const TIMER_SOUND_ROW: Text = tri("计时结束声音", "計時結束聲音", "Timer end sound");
const CLOCK_TICKING_ROW: Text = tri("钟表运作音效", "鐘錶運作音效", "Clock ticking");
const CLOCK_BACKGROUND_ROW: Text = tri("后台仍播放钟表运作声音", "背景仍播放鐘錶運作聲音", "Keep the ticking in the background");

/// The sounds a timer's end can make: every sound the ramdisk carries as a
/// ring, by its key and the name the names file gives it in `l`, in the
/// order of their keys. A ramdisk that names none still offers the one a
/// recovery starts with, so the page never stands empty.
pub fn timer_sounds(s: &State, l: Lang) -> Vec<(String, String)> {
    let mut sounds: Vec<(String, String)> = s
        .sound_names
        .ring
        .iter()
        .map(|(key, name)| (key.clone(), sound_name_of(l, name, key)))
        .collect();
    sounds.sort_by(|a, b| a.0.cmp(&b.0));
    if sounds.is_empty() {
        sounds.push((DEFAULT_TIMER_SOUND.to_string(), l.pick(TIMER_SOUND_DEFAULT).to_string()));
    }
    sounds
}

/// The name the sound a timer's end makes goes by: the one the names file
/// gives it, or, where the file has no such sound, its key as it stands.
fn timer_sound_name(s: &State, l: Lang) -> String {
    timer_sounds(s, l)
        .into_iter()
        .find(|(key, _)| *key == s.timer_sound)
        .map_or_else(|| s.timer_sound.clone(), |(_, name)| name)
}

/// What the one sound a recovery starts with is called where the ramdisk
/// names nothing.
const TIMER_SOUND_DEFAULT: Text = tri("恋爱裁判", "戀愛裁判", "Love Trial");

/// A sound's name in `l`, the key it is kept by where the names file has
/// none.
fn sound_name_of(l: Lang, name: &skyward_config::SoundName, key: &str) -> String {
    let picked = if l == Lang::En { &name.en } else { &name.zh };
    if picked.is_empty() { key.to_string() } else { picked.clone() }
}

/// The clock's settings: the sound the timer's end makes, and the two
/// switches that say whether the clock ticks. No icon stands before any of
/// them — they are four words to read, not four apps. The choice and the
/// switches stand in two cards, divided the way the Settings pages divide
/// their entries: by the gap between cards, not by a rule inside one.
pub fn clock_settings(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let name = timer_sound_name(s, l);
    let sound = widgets::list(&mut c, t, f, top, &[
        Row::new(l.pick(TIMER_SOUND_ROW)).trail(Trail::Value(&name)),
    ]);
    out.extend(hits(&sound.rows, &[Action::Open(Screen::ClockSound)]));
    let switches = widgets::list(&mut c, t, f, sound.bottom + t.m.card_gap, &[
        Row::new(l.pick(CLOCK_TICKING_ROW)).trail(Trail::Switch(s.clock_ticking)),
        Row::new(l.pick(CLOCK_BACKGROUND_ROW)).trail(Trail::Switch(s.clock_ticking_background)),
    ]);
    out.extend(hits(&switches.rows, &[
        Action::Toggle(Switch::ClockTicking),
        Action::Toggle(Switch::ClockTickingBackground),
    ]));
    Frame::new(t, fit(c, t, switches.bottom), out, l.pick(CLOCK_SETTINGS_TITLE), top)
}

/// The sounds a timer's end can make, the one in use ticked.
pub fn clock_sounds(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let sounds = timer_sounds(s, l);
    let rows: Vec<Row> = sounds
        .iter()
        .map(|(key, name)| Row::new(name.as_str()).trail(Trail::Check(*key == s.timer_sound)))
        .collect();
    let list = widgets::list(&mut c, t, f, top, &rows);
    let picks: Vec<Action> = (0..sounds.len()).map(Action::ClockSound).collect();
    out.extend(hits(&list.rows, &picks));
    Frame::new(t, fit(c, t, list.bottom), out, l.pick(TIMER_SOUND_ROW), top)
}

/// Recovery home, laid out like the Settings root: title, search, a device
/// card, then grouped entries. The status bar carries the readings: the
/// slot, the data partition's lock and the battery's temperature.
pub fn home(cx: &Ctx) -> Frame {
    let (t, f, i, l) = (cx.t, cx.f, cx.info, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    // The search bar is left room on the page and drawn over it, folding
    // away as the page scrolls up.
    let bar = Rect::new(t.m.margin, t.m.content_top, t.m.w as f32 - t.m.margin * 2.0, t.m.search_h);
    let mut y = bar.y + bar.h + t.m.after_search;
    let cards_top = y;
    let fold = Fold { bar, hint: l.pick(texts::search::HINT), span: cards_top - bar.y };
    out.push(Hit { rect: bar, action: Action::Open(Screen::Search) });

    let sub = match &i.os_version {
        Some(v) => format!("{} · {v}", i.codename),
        None => i.codename.clone(),
    };
    // Under the phone's name the card holds only what asks for something to
    // be done: a battery too low to install on, an update waiting to merge,
    // and a restart into the slot the system does not run from now.
    let low = match i.battery.as_ref().and_then(skyward_install::battery_refusal) {
        Some(Refusal::Battery { level, charging }) => Some(fmt::battery_low(l, level, charging)),
        _ => None,
    };
    let next = match (i.slot, &cx.state.boot_slots) {
        (Some(now), Some(Ok(slots))) if slots.active != now => Some(fmt::next_boot(l, slots.active, now)),
        _ => None,
    };
    let mut rows = Vec::new();
    let mut opens = Vec::new();
    if let Some((title, note)) = &low {
        rows.push(Row::new(title).icon(RED, Icon::Battery).subtitle(note).trail(Trail::None));
        opens.push(None);
    }
    if let Some(note) = fmt::snapshot_note(l, &i.snapshot) {
        rows.push(Row::new(fmt::snapshot(l, &i.snapshot)).icon(ORANGE, Icon::Layers).subtitle(note).trail(Trail::Arrow));
        opens.push(Some(Action::Open(Screen::Update)));
    }
    if let Some((title, note)) = &next {
        rows.push(Row::new(title).icon(ORANGE, Icon::Power).subtitle(note).trail(Trail::Arrow));
        opens.push(Some(Action::Open(Screen::Slot)));
    }
    if cx.state.system_notice.is_some() {
        let why = l.pick(texts::home::TAP_TO_SEE_WHY);
        rows.push(Row::new(l.pick(texts::systems::SECOND_FAILED)).icon(RED, Icon::Systems).subtitle(why).trail(Trail::Arrow));
        opens.push(Some(Action::Open(Screen::Systems)));
    }
    let card = widgets::device_card(&mut c, t, f, y, &i.model, &sub, &rows);
    out.push(Hit { rect: card.rows[0], action: Action::Open(Screen::About) });
    out.extend(card.rows[1..].iter().zip(&opens).filter_map(|(&rect, open)| open.map(|action| Hit { rect, action })));
    y = card.bottom + t.m.card_gap;
    // On a phone whose WLAN the recovery brings up, the WLAN comes first
    // under the device, as on the stock settings home; with it, where the
    // phone can hold a second system, the two systems.
    let mut rows = Vec::new();
    let mut opens = Vec::new();
    if let Some(w) = &cx.state.wlan {
        rows.push(Row::new(l.pick(texts::page::WLAN)).icon(BLUE, Icon::Wlan).trail(Trail::Value(fmt::wlan_value(l, w))));
        opens.push(Action::Open(Screen::Wlan));
    }
    let starts = systems_value(cx);
    if cx.state.offers(Screen::Systems) {
        let row = Row::new(l.pick(texts::page::DUAL_BOOT)).icon(PURPLE, Icon::Systems);
        rows.push(match &starts {
            Some(value) => row.trail(Trail::Value(value)),
            None => row,
        });
        opens.push(Action::Open(Screen::Systems));
    }
    if !rows.is_empty() {
        let first = widgets::list(&mut c, t, f, y, &rows);
        out.extend(hits(&first.rows, &opens));
        y = first.bottom + t.m.card_gap;
    }
    let tools = widgets::list(&mut c, t, f, y, &[
        Row::new(l.pick(texts::page::INSTALL)).icon(GREEN, Icon::Install),
        Row::new(l.pick(texts::page::BACKUP)).icon(BLUE, Icon::Backup),
        Row::new(l.pick(texts::page::RESTORE)).icon(PURPLE, Icon::Restore),
        Row::new(l.pick(texts::page::MOUNT)).icon(SLATE, Icon::Mount),
        Row::new(l.pick(texts::page::DECRYPT)).icon(BLUE, Icon::Lock).trail(Trail::Value(fmt::data(l, &cx.state.data))),
    ]);
    out.extend(hits(&tools.rows, &[
        Action::Open(Screen::Install),
        Action::Open(Screen::Backup),
        Action::Open(Screen::Restore),
        Action::Open(Screen::Mount),
        Action::Open(Screen::Decrypt),
    ]));
    y = tools.bottom + t.m.card_gap;
    let target = l.pick(REBOOT_TARGETS[cx.state.reboot].0);
    let system = widgets::list(&mut c, t, f, y, &[
        Row::new(l.pick(texts::page::WIPE)).icon(RED, Icon::Wipe),
        Row::new(l.pick(texts::page::ADVANCED)).icon(SLATE, Icon::More),
        Row::new(l.pick(texts::page::SETTINGS)).icon(SLATE, Icon::Gear),
        Row::new(l.pick(texts::page::REBOOT)).icon(ORANGE, Icon::Power).trail(Trail::Value(target)),
    ]);
    out.extend(hits(&system.rows, &[
        Action::Open(Screen::Wipe),
        Action::Open(Screen::Advanced),
        Action::Open(Screen::Settings),
        Action::Open(Screen::Reboot),
    ]));
    // However little it holds, the page scrolls far enough to fold its bar
    // away.
    let height = fitted(t, system.bottom).max(t.m.h + fold.span.ceil() as u32);
    Frame { back: false, fold: Some(fold), ..Frame::new(t, c.crop_height(height), out, l.pick(HOME_TITLE), cards_top) }
}

/// What the home page says beside its entry to the systems: the one that
/// starts, where a second system is installed, once they are read.
fn systems_value(cx: &Ctx) -> Option<String> {
    let l = cx.state.lang;
    match &cx.state.systems {
        Some(Ok(systems)) => Some(match (&systems.other, starting(systems)) {
            (None, _) => l.pick(texts::home::NO_SECOND_SYSTEM).into(),
            (Some(_), Who::Installed) => systems.installed.name.clone(),
            (Some(other), Who::Other) => other.name.clone(),
        }),
        _ => None,
    }
}

/// What the advanced page says beside its system update entry: a download
/// under way, a newer system found, or where an update the system installed
/// stands.
fn update_value(cx: &Ctx) -> &'static str {
    let (s, l) = (cx.state, cx.state.lang);
    if s.downloading() {
        return l.pick(texts::update::DOWNLOADING);
    }
    if s.newest().is_some_and(|r| cx.info.os_version.as_deref() != Some(r.version.as_str())) {
        return l.pick(texts::update::NEW_VERSION);
    }
    fmt::update_value(l, &s.update)
}

/// Preferences, opening with the stock「显示与亮度」theme previews.
pub fn settings(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let themes = widgets::theme_picker(&mut c, t, f, l, top + t.m.dp(8.9));
    out.extend(hits(&themes.rows, &[Action::Theme { dark: false }, Action::Theme { dark: true }]));
    let mut y = widgets::section(&mut c, t, f, themes.bottom, l.pick(texts::settings::BRIGHTNESS));
    let off = fmt::screen_off(l, SCREEN_OFF[s.screen_off]);
    let below = Row::new(l.pick(texts::page::SCREEN_TIMEOUT)).trail(Trail::Value(&off));
    let slider = widgets::slider_card(&mut c, t, f, y, Caption::Sun, l.pick(texts::settings::SCREEN_BRIGHTNESS), s.brightness, Some(below));
    out.push(Hit { rect: slider.track, action: Action::Slider(Slider::Brightness) });
    if let Some(r) = slider.below {
        out.push(Hit { rect: r, action: Action::Open(Screen::ScreenOff) });
    }
    y = widgets::section(&mut c, t, f, slider.bottom, l.pick(texts::settings::VIBRATION));
    let below = Row::new(l.pick(texts::settings::TOUCH_VIBRATION)).trail(Trail::Switch(s.vibrate));
    let vibration = widgets::slider_card(&mut c, t, f, y, Caption::Vibrate, l.pick(texts::settings::VIBRATION_INTENSITY), s.vibration, Some(below));
    out.push(Hit { rect: vibration.track, action: Action::Slider(Slider::Vibration) });
    if let Some(r) = vibration.below {
        out.push(Hit { rect: r, action: Action::Toggle(Switch::Vibrate) });
    }
    y = widgets::section(&mut c, t, f, vibration.bottom, l.pick(texts::settings::GENERAL));
    let general = widgets::list(&mut c, t, f, y, &[
        Row::new(l.among("音量键用作光标", "音量鍵用作游標", "Volume keys move the light")).icon(SLATE, Icon::More).trail(Trail::Switch(s.volume_keys)),
        Row::new(l.pick(PERSONALIZATION)).icon(PURPLE, Icon::More),
        Row::new(l.pick(texts::page::LANGUAGE)).icon(BLUE, Icon::Globe).trail(Trail::Value(l.name())),
        Row::new(l.pick(texts::page::TIME_ZONE)).icon(ORANGE, Icon::Clock).trail(Trail::Value(l.pick(TIME_ZONES[s.time_zone].0))),
    ]);
    out.extend(hits(&general.rows, &[Action::Toggle(Switch::VolumeKeys), Action::Open(Screen::Personalization), Action::Open(Screen::Language), Action::Open(Screen::TimeZone)]));
    y = widgets::section(&mut c, t, f, general.bottom, "USB");
    let usb = widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::settings::FILE_TRANSFER)).icon(BLUE, Icon::Usb).trail(Trail::Switch(s.file_transfer))]);
    out.extend(hits(&usb.rows, &[Action::Toggle(Switch::FileTransfer)]));
    y = widgets::footnote(&mut c, t, f, usb.bottom, l.pick(texts::settings::FILE_TRANSFER_NOTE));
    y = widgets::section(&mut c, t, f, y, l.pick(texts::settings::DEBUGGING));
    let debug = widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::page::TERMINAL)).icon(SLATE, Icon::Terminal)]);
    out.extend(hits(&debug.rows, &[Action::Open(Screen::Terminal)]));
    y = widgets::section(&mut c, t, f, debug.bottom, l.pick(texts::settings::ABOUT));
    // The row borrows the version, so it outlives the list drawn from it.
    let version = version_text();
    let mut rows = vec![Row::new(l.pick(texts::page::ABOUT)).trail(Trail::Value(version.as_str()))];
    let mut opens = vec![Action::Open(Screen::About)];
    if s.offers(Screen::SkywardUpdate) {
        let row = Row::new(l.pick(texts::page::SKYWARD_UPDATE));
        rows.push(match skyward_value(cx) {
            Some(value) => row.trail(Trail::Value(value)),
            None => row,
        });
        opens.push(Action::Open(Screen::SkywardUpdate));
    }
    let about = widgets::list(&mut c, t, f, y, &rows);
    out.extend(hits(&about.rows, &opens));
    Frame::new(t, fit(c, t, about.bottom), out, l.pick(texts::page::SETTINGS), top)
}

/// What the settings page says beside its Skyward update entry: an update
/// under way, or a newer release found.
fn skyward_value(cx: &Ctx) -> Option<&'static str> {
    let (s, l) = (cx.state, cx.state.lang);
    if s.updating_skyward() {
        return Some(l.pick(texts::skyward_update::UPDATING));
    }
    match (s.skyward_newest(), &s.skyward_job) {
        (_, Some(SkywardJob { outcome: Some(SkywardEnd::Updated(_)), .. })) => Some(l.pick(texts::skyward_update::UPDATED)),
        (Some(n), _) if n.newer && n.build.is_some() => Some(l.pick(texts::update::NEW_VERSION)),
        _ => None,
    }
}

/// How long the screen stays on with nothing touched, opened from the
/// settings.
pub fn screen_off(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let top = t.m.content_top;
    let labels = SCREEN_OFF.map(|ms| fmt::screen_off(l, ms));
    let rows: Vec<Row> = labels.iter().enumerate().map(|(k, label)| Row::new(label).trail(Trail::Check(s.screen_off == k))).collect();
    let times = widgets::list(&mut c, t, f, top, &rows);
    let picks: Vec<Action> = (0..SCREEN_OFF.len()).map(Action::ScreenOff).collect();
    let y = widgets::footnote(&mut c, t, f, times.bottom, l.pick(texts::screen_off::NOTE));
    if SCREEN_OFF[s.screen_off].is_none() {
        widgets::footnote(&mut c, t, f, y, l.pick(texts::screen_off::STAYS_ON));
    }
    Frame::new(t, c, hits(&times.rows, &picks), l.pick(texts::page::SCREEN_TIMEOUT), top)
}

/// Mount manager: the slot's partitions, each with a switch to mount it
/// read-only or unmount it; one that programs are using says which, and
/// stays.
pub fn mount(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;

    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::mount::PARTITIONS));
    let bottom = match &s.mounts {
        Err(why) => {
            let y = widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::mount::UNREADABLE)).trail(Trail::None)]).bottom;
            widgets::footnote(&mut c, t, f, y, why)
        }
        Ok(mounts) if mounts.is_empty() => widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::mount::NONE)).trail(Trail::None)]).bottom,
        Ok(mounts) => {
            let subs: Vec<String> = mounts
                .iter()
                .map(|m| match m.holders.is_empty() {
                    true => m.point.clone(),
                    false => texts::mount::in_use_by(l, &m.point, m.holders.join(l.pick(texts::list::SEPARATOR))),
                })
                .collect();
            let rows: Vec<Row> = mounts
                .iter()
                .zip(&subs)
                .map(|(m, sub)| {
                    let trail = if m.holders.is_empty() { Trail::Switch(m.mounted) } else { Trail::Text(l.pick(texts::mount::IN_USE)) };
                    Row::new(&m.base).subtitle(sub).trail(trail)
                })
                .collect();
            let list = widgets::list(&mut c, t, f, y, &rows);
            out.extend(list.rows.iter().zip(mounts).enumerate().map(|(i, (&rect, m))| {
                let action = match m.holders.is_empty() {
                    true => Action::MountToggle(i),
                    false => Action::Unavailable(l.pick(texts::mount::CANT_UNMOUNT_IN_USE)),
                };
                Hit { rect, action }
            }));
            let y = match &s.mount_failure {
                Some(failure) => widgets::footnote(&mut c, t, f, list.bottom, &fmt::mount_failure(l, failure)),
                None => list.bottom,
            };
            let about = match cx.info.slot.map(|s| s.letter()) {
                Some(slot) => texts::mount::shows_slot(l, slot),
                None => l.pick(texts::mount::SHOWS).into(),
            };
            widgets::footnote(&mut c, t, f, y, &about)
        }
    };

    Frame::new(t, fit(c, t, bottom), out, l.pick(texts::page::MOUNT), top)
}

/// Who made the recovery and who keeps this extension, as the About page
/// names them under its version.
const AUTHOR: &str = "© 2026 Kirk Lin & 天某人";

/// About this device, modelled on the stock「我的设备」: the wordmark, the
/// version and its author over a moving colour field, and the summary,
/// hardware and link cards scrolling over it. Only the cards are drawn
/// here; the canvas is clear wherever the field shows.
pub fn about(cx: &Ctx) -> Frame {
    let (t, f) = (cx.t, cx.f);
    let m = &t.m;
    // The page is drawn whole, under the field: laid out first to learn
    // how tall it comes to, then drawn that tall.
    let (_, _, bottom) = about_cards(cx, &mut Canvas::band(m.w, m.h * 3, 0..0, 0..0, Rgba { r: 0, g: 0, b: 0, a: 0 }));
    let mut c = Canvas::new(m.w, fitted(t, bottom));
    let (out, summary_top, bottom) = about_cards(cx, &mut c);
    let canvas = fit(c, t, bottom);
    let art = HeroArt {
        wordmark: widgets::wordmark(t, f, "Skyward-Extra", m.dp(233.7), m.dp(37.8)),
        version: widgets::version_line(t, f, &version_text()),
        author: widgets::author_line(t, f, AUTHOR),
        runs: canvas.runs(),
    };
    Frame { header: false, hero: Some(art), ..Frame::new(t, canvas, out, cx.state.lang.pick(texts::page::ABOUT), summary_top) }
}

/// The phone's processor by its marketing name in `l`, from the phone's
/// settings, or else by its cores.
fn soc(l: Lang, i: &DeviceInfo) -> Option<&str> {
    i.soc.as_ref().map(|n| l.among(n.hans.as_str(), n.hant.as_str(), n.en.as_str()))
}

/// The About page's cards on `c`: what touching them does, where the
/// first starts, and where the last ends.
fn about_cards(cx: &Ctx, c: &mut Canvas) -> (Vec<Hit>, f32, f32) {
    let (t, f, i, l) = (cx.t, cx.f, cx.info, cx.state.lang);
    let m = &t.m;
    let tone = Tone::hero(t);
    let mut out = Vec::new();

    let storage = fmt::storage_used(&i.storage).unwrap_or_else(|| fmt::capacity(i.storage.total));
    let mut facts = vec![
        Fact { title: l.pick(texts::specs::MODEL), value: i.model.clone(), link: false },
        Fact { title: l.pick(texts::specs::STORAGE), value: storage, link: true },
    ];
    if let Some(v) = &i.os_version {
        facts.push(Fact { title: l.pick(texts::specs::OS_VERSION), value: v.clone(), link: false });
    }
    if let Some(r) = &i.android {
        facts.push(Fact { title: l.pick(texts::specs::ANDROID_VERSION), value: r.clone(), link: false });
    }
    let summary = widgets::facts(c, t, f, m.dp(490.7), &facts, &tone);
    out.push(Hit { rect: summary.rows[1], action: Action::Open(Screen::Specs) });

    let processor = soc(l, i).map_or_else(|| fmt::cpu(l, &i.cpu), str::to_string);
    let mut specs = vec![(l.pick(texts::specs::PROCESSOR), processor), (l.pick(texts::specs::RAM), fmt::ram(i.ram_kib))];
    if let Some(design) = i.battery.as_ref().and_then(|b| b.design) {
        specs.push((l.pick(texts::specs::BATTERY_CAPACITY), fmt::mah(design)));
    }
    if let Some(d) = i.screen {
        specs.push((l.pick(texts::specs::SCREEN_SIZE), fmt::inches(d)));
    }
    if let Some(p) = i.panel {
        specs.push((l.pick(texts::specs::RESOLUTION), fmt::panel(p)));
    }
    let y = widgets::spec_card(c, t, f, summary.bottom + m.card_gap, &fmt::model_lines(&i.model), &specs, &tone);
    let links = widgets::list_toned(c, t, f, y + m.card_gap, &[Row::new(l.pick(texts::page::SPECS)), Row::new(l.pick(texts::page::LOGS))], &tone);
    out.extend(hits(&links.rows, &[Action::Open(Screen::Specs), Action::Open(Screen::Logs)]));
    (out, summary.rows[0].y, links.bottom)
}

/// Every reading the recovery has, modelled on the stock「全部参数与信息」.
pub fn specs(cx: &Ctx) -> Frame {
    let (t, f, i, l) = (cx.t, cx.f, cx.info, cx.state.lang);
    let gap = t.m.card_gap;
    let mut c = long_page(cx);
    let mut out = Vec::new();
    // This page leaves more room under the title than the settings pages.
    let top = t.m.dp(188.0);

    let mut grid = vec![Spec { icon: Outline::Phone, label: l.pick(texts::specs::MODEL), value: fmt::model_lines(&i.model) }];
    if let Some(design) = i.battery.as_ref().and_then(|b| b.design) {
        grid.push(Spec { icon: Outline::Battery, label: l.pick(texts::specs::BATTERY_CAPACITY), value: fmt::mah(design) });
    }
    let cpu = match soc(l, i) {
        Some(soc) => format!("{soc}\n{}", fmt::cpu(l, &i.cpu)),
        None => fmt::cpu(l, &i.cpu),
    };
    grid.push(Spec { icon: Outline::Chip, label: l.pick(texts::specs::PROCESSOR), value: cpu });
    grid.push(Spec { icon: Outline::Memory, label: l.pick(texts::specs::RAM), value: fmt::ram(i.ram_kib) });
    if let Some(p) = i.panel {
        grid.push(Spec { icon: Outline::Resolution, label: l.pick(texts::specs::RESOLUTION), value: fmt::panel(p) });
    }
    if let Some(d) = i.screen {
        grid.push(Spec { icon: Outline::Screen, label: l.pick(texts::specs::SCREEN_SIZE), value: fmt::inches(d) });
    }
    grid.push(Spec { icon: Outline::Storage, label: l.pick(texts::specs::STORAGE_CAPACITY), value: fmt::capacity(i.storage.total) });
    grid.push(Spec { icon: Outline::Tag, label: l.pick(texts::specs::CODENAME), value: i.codename.clone() });
    let mut y = widgets::spec_grid(&mut c, t, f, top, &grid);

    if let Some(v) = &i.os_version {
        y = widgets::info_card(&mut c, t, f, y + gap, &[(l.pick(texts::specs::OS_VERSION), v.clone())]);
    }
    if let Some(r) = &i.android {
        y = widgets::info_card(&mut c, t, f, y + gap, &[(l.pick(texts::specs::ANDROID_VERSION), fmt::android(l, r, i.security_patch.as_ref()))]);
    }
    y = widgets::info_card(&mut c, t, f, y + gap, &[
        (l.pick(texts::specs::RECOVERY_VERSION), version_text()),
        (l.pick(texts::specs::KERNEL_VERSION), i.kernel.clone()),
    ]);

    let mut boot = Vec::new();
    if let Some(s) = i.slot {
        boot.push((l.pick(texts::slot::CURRENT), s.letter().to_string()));
        boot.push((l.pick(texts::page::UPDATE), fmt::snapshot(l, &i.snapshot).to_string()));
    }
    if let Some(unlocked) = i.boot.unlocked {
        let state = if unlocked { texts::specs::UNLOCKED } else { texts::specs::LOCKED };
        boot.push(("Bootloader", l.pick(state).to_string()));
    }
    if let Some(v) = &i.boot.verified {
        boot.push((l.pick(texts::specs::VERIFIED_BOOT), fmt::verified(l, v)));
    }
    boot.push((l.pick(texts::specs::DATA_PARTITION), fmt::data(l, &cx.state.data).to_string()));
    y = widgets::info_card(&mut c, t, f, y + gap, &boot);

    if let Some(b) = &i.battery {
        let mut rows = vec![(l.pick(texts::specs::LEVEL), fmt::level(l, b))];
        if let Some(v) = b.temp {
            rows.push((l.pick(texts::specs::TEMPERATURE), fmt::celsius(v)));
        }
        if let Some(v) = fmt::wear(l, b) {
            rows.push((l.pick(texts::specs::HEALTH), v));
        }
        if let Some(v) = b.cycles {
            rows.push((l.pick(texts::specs::CHARGE_CYCLES), texts::specs::cycles(l, v)));
        }
        if let Some(v) = b.voltage {
            rows.push((l.pick(texts::specs::VOLTAGE), fmt::volts(v)));
        }
        y = widgets::info_card(&mut c, t, f, y + gap, &rows);
    }

    let total = fmt::capacity(i.storage.total);
    let storage = match i.storage.free.map(fmt::gb) {
        Some(free) => texts::specs::free_of(l, free, total),
        None => texts::specs::total_unmounted(l, total),
    };
    y = widgets::info_card(&mut c, t, f, y + gap, &[(l.pick(texts::specs::INTERNAL_STORAGE), storage)]);
    let links = widgets::list(&mut c, t, f, y + gap, &[
        Row::new(l.pick(texts::page::LICENSE)).trail(Trail::Value("GPL-3.0")),
        Row::new(l.pick(texts::page::FONTS)).trail(Trail::Value("MiSans · Lexend Deca")),
    ]);
    out.extend(hits(&links.rows, &[Action::Open(Screen::License), Action::Open(Screen::FontLicense)]));
    Frame::new(t, fit(c, t, links.bottom), out, l.pick(texts::page::SPECS), top)
}

/// The data partition and its users' own files. The page opens the
/// partition as it shows, which is how it learns each user's screen lock;
/// a user whose files wait on one is asked with the control that lock is
/// answered with, from the start.
pub fn decrypt(cx: &Ctx) -> Frame {
    let (c, l) = (page(cx), cx.state.lang);
    match &cx.state.data {
        Data::Locked { user, keys, checking } => code(cx, c, *user, *keys, *checking),
        Data::Closed | Data::Opening => data_status(
            cx,
            c,
            Icon::Lock,
            l.pick(texts::decrypt::DATA_PARTITION),
            Some(l.pick(texts::decrypt::READING_LOCK)),
            None,
        ),
        Data::Open => data_status(cx, c, Icon::Folder, l.pick(texts::decrypt::DECRYPTED), Some(l.pick(texts::decrypt::SEE_FILES)), None),
        Data::Failed(why) => data_status(cx, c, Icon::Lock, l.pick(texts::decrypt::FAILED), None, Some(why)),
        Data::Erased => data_status(cx, c, Icon::Wipe, l.pick(texts::data::ERASED), Some(l.pick(texts::decrypt::MADE_ANEW)), None),
    }
}

/// How far apart, in dp, the lines of the Decrypt page's title are where
/// it takes more than one.
const TITLE_PITCH: f32 = 32.0;

/// Where the partition stands, under its sign: a line on it, or why it
/// did not open and a way to try again; then the way off the page.
fn data_status(cx: &Ctx, mut c: Canvas, icon: Icon, title: &str, note: Option<&str>, why: Option<&str>) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let m = &t.m;
    let x = m.w as f32 / 2.0;
    let sign_cy = m.dp(150.0);
    c.fill_circle(x, sign_cy, m.dp(30.0), t.pal.card);
    icons::glyph(&mut c, icon, x, sign_cy, m.dp(40.0), t.pal.accent);
    // What stands under the title moves down by the lines the title takes.
    let room = m.w as f32 - m.title_x * 2.0;
    let more = widgets::centred(&mut c, &f.heading, title, x, sign_cy + m.dp(64.0), m.dp(24.0), m.dp(TITLE_PITCH), t.pal.text, room);
    if let Some(note) = note {
        widgets::centred(&mut c, &f.body, note, x, sign_cy + m.dp(98.0) + more, m.sub_px, m.sub_pitch, t.pal.subtitle, room);
    }
    let mut out = Vec::new();
    if let Some(why) = why {
        widgets::paragraph(&mut c, t, f, sign_cy + m.dp(88.0) + more, why);
        let again = widgets::button(&mut c, t, f, m.h as f32 * 0.62, l.pick(texts::common::TRY_AGAIN));
        out.push(Hit { rect: again, action: Action::OpenData });
    }
    let leave_cy = m.h as f32 - m.dp(64.0);
    let leave = l.pick(if matches!(cx.state.data, Data::Open | Data::Erased) { texts::common::DONE } else { texts::decrypt::NOT_NOW });
    c.text_center(&f.heading, leave, x, leave_cy, m.row_title_px, t.pal.accent);
    out.push(Hit { rect: Rect::new(x - m.dp(80.0), leave_cy - m.dp(24.0), m.dp(160.0), m.dp(48.0)), action: Action::Skip });
    Frame { header: false, ..Frame::new(t, c, out, l.pick(texts::page::DECRYPT), 0.0) }
}

/// A user's own files wait on their screen lock: the control it is
/// answered with, which takes nothing while what was entered is checked.
fn code(cx: &Ctx, mut c: Canvas, user: u32, keys: Keys, checking: bool) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let x = m.w as f32 / 2.0;
    let mut out = Vec::new();
    let title = match (keys == Keys::Pattern, user) {
        (true, 0) => l.pick(texts::decrypt::DRAW_PATTERN).to_string(),
        (true, user) => texts::decrypt::draw_pattern_of(l, user),
        (false, 0) => l.pick(texts::decrypt::ENTER_LOCK).to_string(),
        (false, user) => texts::decrypt::enter_lock_of(l, user),
    };
    let note = l.pick(if checking { texts::decrypt::CHECKING } else { texts::decrypt::TO_SEE_FILES });
    let room = m.w as f32 - m.title_x * 2.0;
    let more = widgets::centred(&mut c, &f.heading, &title, x, m.dp(130.0), m.dp(24.0), m.dp(TITLE_PITCH), t.pal.text, room);
    widgets::centred(&mut c, &f.body, note, x, m.dp(166.0) + more, m.sub_px, m.sub_pitch, t.pal.subtitle, room);
    let mut pattern = None;
    let bottom = match keys {
        Keys::Letters => {
            let top = m.h as f32 - widgets::keyboard_height(t);
            let caps = widgets::keyboard(&mut c, t, f, l, top, s.layer, s.key_held(), l.pick(texts::page::DECRYPT));
            if !checking {
                out.extend(caps.into_iter().map(|(rect, cap)| Hit { rect, action: Action::Type(cap) }));
            }
            top
        }
        Keys::Pattern => {
            let span = m.dp(300.0);
            let cy = m.dp(300.0) + span / 2.0;
            widgets::pattern_lock(&mut c, t, x, cy, span, &s.pattern);
            if !checking {
                out.push(Hit { rect: Rect::new(x - span, cy - span, span * 2.0, span * 2.0), action: Action::Pattern });
                pattern = Some((x, cy, span));
            }
            m.h as f32 - m.dp(26.0)
        }
        Keys::Digits => {
            let top = m.h as f32 - widgets::keypad_height(t);
            let caps = widgets::keypad(&mut c, t, f, l, top, s.key_held());
            if !checking {
                out.extend(caps.into_iter().map(|(rect, cap)| Hit { rect, action: Action::Type(cap) }));
            }
            top
        }
    };
    // The dots sit just above whatever types into them; a pattern is its
    // own showing of what has been drawn.
    if keys != Keys::Pattern {
        widgets::code_dots(&mut c, t, x, bottom - m.dp(92.0), s.code.chars().count());
    }
    let skip_cy = bottom - m.dp(34.0);
    c.text_center(&f.heading, l.pick(texts::decrypt::NOT_NOW), x, skip_cy, m.row_title_px, t.pal.accent);
    out.push(Hit { rect: Rect::new(x - m.dp(80.0), skip_cy - m.dp(24.0), m.dp(160.0), m.dp(48.0)), action: Action::Skip });
    Frame { header: false, pattern, ..Frame::new(t, c, out, l.pick(texts::page::DECRYPT), 0.0) }
}

/// Reboot target picker with a swipe-to-confirm control.
pub fn reboot(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let mut y = widgets::section_first(&mut c, t, f, top, l.pick(texts::reboot::TO));
    let rows: Vec<Row> = REBOOT_TARGETS
        .iter()
        .enumerate()
        .map(|(k, &(title, sub))| {
            let r = Row::new(l.pick(title)).trail(Trail::Check(s.reboot == k));
            match sub {
                Some(text) => r.subtitle(l.pick(text)),
                None => r,
            }
        })
        .collect();
    let targets = widgets::list(&mut c, t, f, y, &rows);
    let picks: Vec<Action> = (0..TARGETS.len()).map(Action::RebootTarget).collect();
    out.extend(hits(&targets.rows, &picks));
    y = targets.bottom;
    if let Some(slot) = cx.info.slot {
        let next = match &s.boot_slots {
            Some(Ok(slots)) if slots.active == slot => texts::reboot::current(l, slot.letter()),
            Some(Ok(slots)) => slots.active.letter().to_string(),
            Some(Err(_)) => l.pick(texts::common::UNREADABLE).to_string(),
            None => String::new(),
        };
        y = widgets::section(&mut c, t, f, y, l.pick(texts::slot::TITLE));
        let slots = widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::reboot::NEXT_BOOT)).trail(Trail::Value(&next))]);
        out.extend(hits(&slots.rows, &[Action::Open(Screen::Slot)]));
        y = slots.bottom;
    }
    let (label, danger) = confirm_label(Confirm::Reboot, cx.state);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(28.0), label, danger, s.confirm);
    out.push(Hit { rect: track, action: Action::Confirm(Confirm::Reboot) });
    Frame::new(t, c, out, l.pick(texts::page::REBOOT), top)
}

/// Installing, opened from the home page: the system on the phone now, and
/// where a package can come from.
pub fn install(cx: &Ctx) -> Frame {
    let (t, f, i, l) = (cx.t, cx.f, cx.info, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let mut facts = Vec::new();
    if let Some(v) = &i.os_version {
        facts.push((l.pick(texts::install::CURRENT_SYSTEM), v.clone()));
    }
    if let Some(p) = &i.security_patch {
        facts.push((l.pick(texts::install::SECURITY_PATCH), fmt::patch(l, p)));
    }
    if let Some(s) = i.slot {
        facts.push((l.pick(texts::slot::CURRENT), texts::install::slot_detail(l, s.letter(), s.other().letter())));
    }
    let ways = l.pick(texts::install::WAYS);
    let y = if facts.is_empty() {
        widgets::section_first(&mut c, t, f, top, ways)
    } else {
        let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::common::THIS_PHONE));
        let y = widgets::info_card(&mut c, t, f, y, &facts);
        widgets::section(&mut c, t, f, y, ways)
    };
    let mut rows = Vec::new();
    if let Some(note) = fmt::snapshot_note(l, &i.snapshot) {
        rows.push(Row::new(fmt::snapshot(l, &i.snapshot)).icon(ORANGE, Icon::Layers).subtitle(note).trail(Trail::None));
    }
    // Internal storage opens with the data partition, which the decrypt
    // page opens or tells why it does not.
    let data = &cx.state.data;
    let (storage, from_storage) = if data.storage_open() {
        (texts::install::CHOOSE_IN_STORAGE, Action::Open(Screen::Choose(Place::Storage)))
    } else if *data == Data::Erased {
        (texts::data::ERASED, Action::Open(Screen::Decrypt))
    } else {
        (texts::decrypt::NEEDED, Action::Open(Screen::Decrypt))
    };
    // The drive's page waits for one to be plugged in, and says why one
    // that is cannot be read.
    let usb = match &cx.state.usb_drive {
        Some(UsbDrive::Shown) => texts::install::CHOOSE_ON_USB_DRIVE,
        Some(UsbDrive::Unreadable(_)) => texts::usb_drive::UNREADABLE,
        None => texts::usb_drive::WANTED,
    };
    rows.push(Row::new(install_title(l, Source::File(Place::Storage))).icon(BLUE, Icon::Phone).subtitle(l.pick(storage)));
    rows.push(Row::new(install_title(l, Source::File(Place::UsbDrive))).icon(PURPLE, Icon::UsbDrive).subtitle(l.pick(usb)));
    let sideload = l.pick(texts::install::SIDELOAD);
    rows.push(Row::new(install_title(l, Source::Computer)).icon(GREEN, Icon::Usb).subtitle(sideload));
    let methods = widgets::list(&mut c, t, f, y, &rows);
    let from_usb = Action::Open(Screen::Choose(Place::UsbDrive));
    let mut out = hits(&methods.rows[methods.rows.len() - 3..], &[from_storage, from_usb, Action::Sideload]);
    // A phone that can hold a second system installs one from here too.
    let what = l.pick(match cx.state.dual_boot {
        true => texts::install::WHAT_WITH_SECOND_SYSTEM,
        false => texts::install::WHAT,
    });
    let y = widgets::footnote(&mut c, t, f, methods.bottom, what);
    let y = widgets::section(&mut c, t, f, y, l.pick(texts::common::OPTIONS));
    let check = Row::new(l.pick(texts::install::VERIFY_SIGNATURES)).trail(Trail::Switch(cx.state.check_signature));
    let signature = widgets::list(&mut c, t, f, y, &[check]);
    out.extend(hits(&signature.rows, &[Action::Toggle(Switch::Signature)]));
    let mut y = widgets::footnote(&mut c, t, f, signature.bottom, l.pick(texts::install::VERIFY_SIGNATURES_NOTE));
    if cx.state.recovery_partitions {
        let row = Row::new(l.pick(texts::install::FLASH_SKYWARD_AGAIN)).trail(Trail::Switch(cx.state.keep_recovery));
        let options = widgets::list(&mut c, t, f, y + t.m.card_gap, &[row]);
        out.extend(hits(&options.rows, &[Action::Toggle(Switch::KeepRecovery)]));
        y = widgets::footnote(&mut c, t, f, options.bottom, l.pick(texts::install::FLASH_SKYWARD_AGAIN_NOTE));
    }
    Frame::new(t, fit(c, t, y), out, l.pick(texts::page::INSTALL), top)
}

/// How installing from `source` is named: on the install page, and as the
/// title of the page that installs.
fn install_title(l: Lang, source: Source) -> &'static str {
    use texts::install::title;
    l.pick(match source {
        Source::Computer => title::FROM_COMPUTER,
        Source::File(Place::Storage) => title::FROM_STORAGE,
        Source::File(Place::UsbDrive) => title::FROM_USB_DRIVE,
        Source::Opened { .. } => title::FROM_FILES,
        Source::Asked { security: false } => title::SYSTEM_UPDATE,
        Source::Asked { security: true } => title::SECURITY_UPDATE,
        Source::Rescue => title::RESCUE,
    })
}

/// What a place is called at the top of its folders.
fn place_name(l: Lang, place: Place) -> &'static str {
    l.pick(match place {
        Place::Storage => texts::place::STORAGE,
        Place::UsbDrive => texts::place::USB_DRIVE,
    })
}

/// A one-line card on how things stand.
fn standing<'a>(title: &'a str, color: Rgba, icon: Icon, detail: &'a str) -> Row<'a> {
    Row::new(title).icon(color, icon).subtitle(detail).trail(Trail::None)
}

fn share(done: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        (done as f64 / total as f64) as f32
    }
}

fn percent(share: f32) -> u32 {
    (share * 100.0).floor() as u32
}

/// Where an update stands, and the share of its step done when known.
fn update_stage(l: Lang, stage: Option<&Stage>) -> (String, Option<f32>) {
    match stage {
        Some(Stage::Writing { done, total }) => {
            let s = share(*done, *total);
            (texts::installing::update_writing(l, percent(s)), Some(s))
        }
        Some(Stage::Verifying { index, count, partition }) => {
            (texts::installing::update_verifying(l, partition, index + 1, count), Some(*index as f32 / *count as f32))
        }
        Some(Stage::Finishing) => (l.pick(texts::installing::UPDATE_FINISHING).into(), None),
        Some(Stage::Reflashing { done, total }) => (l.pick(texts::installing::REFLASHING).into(), Some(*done as f32 / *total as f32)),
        Some(Stage::Flashing { .. } | Stage::Script { .. } | Stage::Signature { .. } | Stage::Allocating { .. } | Stage::Root { .. } | Stage::Storing) | None => {
            (l.pick(texts::installing::GETTING_READY).into(), None)
        }
    }
}

/// Where installing a system beside the installed one stands, and the share
/// of its step done when known.
fn system_stage(l: Lang, stage: Option<&Stage>) -> (String, Option<f32>) {
    match stage {
        Some(Stage::Allocating { done, total }) => {
            let s = share(*done, *total);
            (texts::installing::system_allocating(l, percent(s)), Some(s))
        }
        Some(Stage::Root { done, total }) => {
            let s = share(*done, *total);
            let said = match done * 2 <= *total {
                true => texts::installing::system_writing(l, percent(s)),
                false => texts::installing::system_verifying(l, percent(s)),
            };
            (said, Some(s))
        }
        Some(Stage::Storing) => (l.pick(texts::installing::SYSTEM_STORING).into(), None),
        _ => (l.pick(texts::installing::GETTING_READY).into(), None),
    }
}

/// What the card says while an install script runs, and how far it is: the
/// last line the script showed and how far it says it is, then Skyward
/// going back into the recovery partitions it wrote over.
fn script_stage<'a>(l: Lang, stage: Option<&Stage>, said: &'a [String]) -> (&'a str, Option<f32>) {
    let running = l.pick(texts::installing::RUNNING_SCRIPT);
    let last = said.iter().rev().map(|line| line.trim()).find(|line| !line.is_empty()).unwrap_or(running);
    match stage {
        Some(Stage::Reflashing { done, total }) => (l.pick(texts::installing::REFLASHING), Some(*done as f32 / *total as f32)),
        Some(Stage::Script { thousandths }) => (last, Some(f32::from(*thousandths) / 1000.0)),
        _ => (last, None),
    }
}

/// How Skyward went back into the recovery partitions the install wrote
/// over, where it did.
fn reflashed(c: &mut Canvas, cx: &Ctx, y: f32) -> f32 {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    match &cx.state.reflash {
        None => y,
        Some(Reflash::Written(written)) => {
            let said = match written.as_slice() {
                [slot] => texts::installing::reflashed_one(l, slot.letter()),
                [a, b] => texts::installing::reflashed_two(l, a.letter(), b.letter()),
                other => unreachable!("a phone has two slots, and Skyward went back into {other:?}"),
            };
            widgets::footnote(c, t, f, y, &said)
        }
        Some(Reflash::Failed(why)) => {
            let y = widgets::footnote(c, t, f, y, l.pick(texts::installing::REFLASH_FAILED));
            widgets::footnote(c, t, f, y, why)
        }
    }
}

/// What the install script showed, where there is any: its heading, and
/// under it the lines as the page's long list, the last thing on the page.
fn script_output(c: &mut Canvas, cx: &Ctx, y: f32, rows: &mut Option<Rows>) -> f32 {
    let lines = &cx.state.script_output;
    if lines.is_empty() {
        return y;
    }
    let y = widgets::section(c, cx.t, cx.f, y, cx.state.lang.pick(texts::installing::SCRIPT_OUTPUT));
    *rows = Some(Rows { edges: widgets::output_edges(c, cx.t, cx.f, y, lines), action: None });
    y
}

/// The slots an image can be flashed to, each by its letter where the
/// slot the phone runs from is known.
fn pick_title(l: Lang, pick: Pick, current: Option<Slot>) -> String {
    match (pick, current) {
        (Pick::Current, Some(slot)) => texts::flash::current_slot(l, slot.letter()),
        (Pick::Current, None) => l.pick(texts::slot::CURRENT).into(),
        (Pick::Other, Some(slot)) => texts::flash::other_slot(l, slot.other().letter()),
        (Pick::Other, None) => l.pick(texts::flash::OTHER_SLOT_UNNAMED).into(),
        (Pick::Both, _) => l.pick(texts::flash::BOTH_SLOTS).into(),
    }
}

/// Installing a package step by step: receiving what the computer sends
/// with `adb sideload`, or reading the file chosen, then installing it.
pub fn installing(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = long_page(cx);
    let (mut out, mut rows) = (Vec::new(), None);
    let top = t.m.content_top;
    let computer = s.source == Source::Computer;
    let bottom = match &s.install {
        Install::Idle | Install::Starting | Install::Ending if computer => {
            let row = standing(l.pick(texts::installing::SWITCHING_USB), GREEN, Icon::Usb, l.pick(texts::installing::TO_SIDELOAD));
            widgets::list(&mut c, t, f, top, &[row]).bottom
        }
        Install::Idle | Install::Starting | Install::Ending if s.source == Source::Rescue => {
            let row = standing(l.pick(texts::installing::SWITCHING_USB), GREEN, Icon::Usb, l.pick(texts::installing::TO_RESCUE));
            widgets::list(&mut c, t, f, top, &[row]).bottom
        }
        Install::Idle | Install::Starting | Install::Ending => {
            widgets::list(&mut c, t, f, top, &[Row::new(l.pick(texts::installing::READING_FILE)).icon(GREEN, Icon::Install).trail(Trail::None)]).bottom
        }
        Install::Opening => {
            let opening = Row::new(l.pick(texts::installing::OPENING_DATA)).icon(GREEN, Icon::Lock).trail(Trail::None);
            let y = widgets::list(&mut c, t, f, top, &[opening]).bottom;
            asked_package(&mut c, cx, y)
        }
        Install::Locked(_) => {
            let row = standing(l.pick(texts::installing::CANT_READ_YET), ORANGE, Icon::Lock, l.pick(texts::decrypt::NEEDED));
            let y = widgets::list(&mut c, t, f, top, &[row]).bottom;
            let open = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(l.pick(texts::page::DECRYPT))]);
            out.extend(hits(&open.rows, &[Action::Open(Screen::Decrypt)]));
            let y = widgets::footnote(&mut c, t, f, open.bottom, l.pick(texts::installing::INSTALLS_ONCE_DECRYPTED));
            asked_package(&mut c, cx, y)
        }
        Install::Waiting if s.source == Source::Rescue => {
            let row = standing(l.pick(texts::installing::WAITING), GREEN, Icon::Usb, l.pick(texts::installing::CONNECT));
            let y = widgets::list(&mut c, t, f, top, &[row]).bottom;
            let y = widgets::section(&mut c, t, f, y, l.pick(texts::installing::RUN_ON_COMPUTER));
            let y = widgets::info_card(&mut c, t, f, y, &[
                (l.pick(texts::installing::RESCUE_INSTALL), l.pick(texts::installing::RESCUE_INSTALL_NOTE).into()),
                ("adb rescue getprop", l.pick(texts::installing::RESCUE_GETPROP_NOTE).into()),
                ("adb rescue wipe userdata", l.pick(texts::installing::RESCUE_WIPE_NOTE).into()),
            ]);
            widgets::footnote(&mut c, t, f, y, l.pick(texts::installing::RESCUE_NOTE))
        }
        Install::Waiting => {
            let row = standing(l.pick(texts::installing::WAITING), GREEN, Icon::Usb, l.pick(texts::installing::CONNECT));
            let y = widgets::list(&mut c, t, f, top, &[row]).bottom;
            let y = widgets::section(&mut c, t, f, y, l.pick(texts::installing::RUN_ON_COMPUTER));
            let y = widgets::info_card(&mut c, t, f, y, &[(l.pick(texts::installing::SIDELOAD), l.pick(texts::installing::SIDELOAD_FILE_NOTE).into())]);
            widgets::footnote(&mut c, t, f, y, l.pick(texts::installing::SIDELOAD_NOTE))
        }
        Install::Checking { stage: Some(Stage::Signature { done, total }), .. } => {
            let read = share(*done, *total);
            let detail = texts::installing::read(l, percent(read));
            let y = widgets::progress_card(&mut c, t, f, top, l.pick(texts::installing::VERIFYING_SIGNATURE), &detail, Some(read));
            let note = l.pick(match s.source {
                Source::Computer => texts::installing::CANCEL_ENDS_SIDELOAD,
                Source::Rescue => texts::installing::CANCEL_LEAVES_RESCUE,
                Source::File(Place::Storage) | Source::Opened { drive: false } | Source::Asked { .. } => texts::installing::CANCEL,
                Source::File(Place::UsbDrive) | Source::Opened { drive: true } => texts::installing::CANCEL_KEEP_USB_DRIVE,
            });
            widgets::footnote(&mut c, t, f, y, note)
        }
        Install::Checking { size, .. } => {
            let detail = texts::installing::file_size(l, fmt::bytes(*size));
            let y = widgets::list(&mut c, t, f, top, &[standing(l.pick(texts::installing::READING_FILE), GREEN, Icon::Install, &detail)]).bottom;
            match s.source {
                Source::Computer | Source::Rescue => widgets::footnote(&mut c, t, f, y, l.pick(texts::installing::KEEP_CABLE)),
                Source::File(Place::Storage) | Source::Opened { drive: false } | Source::Asked { .. } => y,
                Source::File(Place::UsbDrive) | Source::Opened { drive: true } => widgets::footnote(&mut c, t, f, y, l.pick(texts::installing::KEEP_USB_DRIVE)),
            }
        }
        Install::Choosing(choice) => {
            let (name, holds) = fmt::image_kind(l, choice.kind);
            let detail = texts::installing::size_and_holds(l, fmt::bytes(choice.size), holds);
            let y = widgets::list(&mut c, t, f, top, &[standing(name, GREEN, Icon::Install, &detail)]).bottom;
            let y = widgets::section(&mut c, t, f, y, l.pick(texts::flash::PARTITION));
            let rows: Vec<Row> = choice
                .partitions
                .iter()
                .enumerate()
                .map(|(k, p)| Row::new(p).subtitle(fmt::partition_role(l, p)).trail(Trail::Check(k == choice.partition)))
                .collect();
            let parts = widgets::list(&mut c, t, f, y, &rows);
            let choose: Vec<Action> = (0..choice.partitions.len()).map(Action::FlashPartition).collect();
            out.extend(hits(&parts.rows, &choose));
            let y = widgets::section(&mut c, t, f, parts.bottom, l.pick(texts::slot::TITLE));
            let titles = Pick::ALL.map(|p| pick_title(l, p, i.slot));
            let rows: Vec<Row> =
                Pick::ALL.iter().zip(&titles).map(|(p, title)| Row::new(title).trail(Trail::Check(*p == choice.pick))).collect();
            let picks = widgets::list(&mut c, t, f, y, &rows);
            out.extend(hits(&picks.rows, &Pick::ALL.map(Action::FlashSlots)));
            let (label, danger) = confirm_label(Confirm::Flash, cx.state);
            let track = widgets::confirm(&mut c, t, f, picks.bottom + t.m.dp(28.0), label, danger, s.confirm);
            out.push(Hit { rect: track, action: Action::Confirm(Confirm::Flash) });
            let note = l.pick(match s.source {
                Source::Computer => texts::flash::OVERWRITES_ENDS_SIDELOAD,
                Source::Rescue => texts::flash::OVERWRITES_LEAVES_RESCUE,
                Source::File(_) | Source::Opened { .. } | Source::Asked { .. } => texts::flash::OVERWRITES,
            });
            widgets::footnote(&mut c, t, f, track.y + track.h, note)
        }
        Install::Confirming { version, wipe } => {
            use texts::installing::confirm;
            let (title, color, icon, detail, which) = if *wipe {
                let detail = match (version, s.cache_partition) {
                    (Some(v), true) => confirm::version_resets_with_cache(l, v),
                    (Some(v), false) => confirm::version_resets(l, v),
                    (None, true) => l.pick(confirm::UPDATE_RESETS_WITH_CACHE).into(),
                    (None, false) => l.pick(confirm::UPDATE_RESETS).into(),
                };
                (l.pick(confirm::WIPE_NEEDED).to_string(), ORANGE, Icon::Wipe, detail, Confirm::InstallWipe)
            } else {
                let title = match version {
                    Some(v) => confirm::version(l, v),
                    None => l.pick(confirm::UPDATE).into(),
                };
                let detail = match i.slot.map(|s| s.other().letter()) {
                    Some(slot) => confirm::to_slot(l, slot),
                    None => l.pick(confirm::STARTS_AFTER_REBOOT).into(),
                };
                (title, GREEN, Icon::Install, detail, Confirm::Install)
            };
            let y = widgets::list(&mut c, t, f, top, &[standing(&title, color, icon, &detail)]).bottom;
            let (label, danger) = confirm_label(which, cx.state);
            let track = widgets::confirm(&mut c, t, f, y + t.m.dp(28.0), label, danger, s.confirm);
            out.push(Hit { rect: track, action: Action::Confirm(which) });
            widgets::footnote(&mut c, t, f, track.y + track.h, l.pick(texts::installing::CANCEL_WRITES_NOTHING))
        }
        Install::Updating { version, stage, overwriting } => {
            let title = match version {
                Some(v) => texts::installing::named(l, v),
                None => l.pick(texts::installing::UPDATE).into(),
            };
            let (detail, done) = update_stage(l, stage.as_ref());
            let y = widgets::progress_card(&mut c, t, f, top, &title, &detail, done);
            let until = until_installed(l, s.source);
            let note = match i.slot.map(|s| s.other().letter()) {
                Some(slot) => texts::installing::until_and_slot(l, until, slot),
                None => until.to_string(),
            };
            let y = widgets::footnote(&mut c, t, f, y, &note);
            if *overwriting {
                widgets::footnote(&mut c, t, f, y, l.pick(texts::installing::OVERWRITING))
            } else {
                y
            }
        }
        Install::Flashing { partition, stage } => {
            let (detail, done) = match stage {
                Some(Stage::Flashing { done, total }) => {
                    let s = share(*done, *total);
                    let detail = match done * 2 < *total {
                        true => texts::flash::writing(l, percent(s)),
                        false => texts::flash::verifying(l, percent(s)),
                    };
                    (detail, Some(s))
                }
                _ => (l.pick(texts::installing::GETTING_READY).into(), None),
            };
            let y = widgets::progress_card(&mut c, t, f, top, &texts::flash::flashing(l, partition), &detail, done);
            let keep = l.pick(match s.source {
                Source::Computer | Source::Rescue => texts::flash::KEEP_CABLE,
                Source::File(Place::Storage) | Source::Opened { drive: false } | Source::Asked { .. } => texts::flash::KEEP_POWER,
                Source::File(Place::UsbDrive) | Source::Opened { drive: true } => texts::flash::KEEP_USB_DRIVE,
            });
            widgets::footnote(&mut c, t, f, y, keep)
        }
        Install::SystemChoosing(choice) => {
            let detail = texts::installing::as_second_system(l, &choice.version);
            let y = widgets::list(&mut c, t, f, top, &[standing(&choice.name, PURPLE, Icon::Systems, &detail)]).bottom;
            let y = widgets::section(&mut c, t, f, y, l.pick(texts::installing::ROOM));
            let sizes: Vec<String> = choice.sizes.iter().map(|&b| fmt::bytes(b)).collect();
            let least = l.pick(texts::installing::LEAST_ROOM);
            let rows: Vec<Row> = sizes
                .iter()
                .enumerate()
                .map(|(k, size)| {
                    let row = Row::new(size).trail(Trail::Check(k == choice.size));
                    if k == 0 { row.subtitle(least) } else { row }
                })
                .collect();
            let list = widgets::list(&mut c, t, f, y, &rows);
            let picks: Vec<Action> = (0..choice.sizes.len()).map(Action::SystemSize).collect();
            out.extend(hits(&list.rows, &picks));
            let y = widgets::footnote(&mut c, t, f, list.bottom, l.pick(texts::installing::ROOM_NOTE));
            let (label, danger) = confirm_label(Confirm::InstallSystem, cx.state);
            let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
            out.push(Hit { rect: track, action: Action::Confirm(Confirm::InstallSystem) });
            widgets::footnote(&mut c, t, f, track.y + track.h, l.pick(texts::installing::CANCEL_WRITES_NOTHING))
        }
        Install::Systeming { name, stage } => {
            let (detail, done) = system_stage(l, stage.as_ref());
            let y = widgets::progress_card(&mut c, t, f, top, &texts::installing::named(l, name), &detail, done);
            widgets::footnote(&mut c, t, f, y, until_installed(l, s.source))
        }
        Install::ConfirmingScript => {
            let name = s.package_name.as_deref().unwrap_or(l.pick(texts::installing::FLASHABLE_ZIP));
            let detail = l.pick(texts::installing::SCRIPT_WARNING);
            let mut y = widgets::list(&mut c, t, f, top, &[standing(name, ORANGE, Icon::Install, detail)]).bottom;
            if let Some(note) = fmt::snapshot_script_note(l, &i.snapshot) {
                y = widgets::footnote(&mut c, t, f, y, note);
            }
            let (label, danger) = confirm_label(Confirm::RunScript, cx.state);
            let track = widgets::confirm(&mut c, t, f, y + t.m.dp(28.0), label, danger, s.confirm);
            out.push(Hit { rect: track, action: Action::Confirm(Confirm::RunScript) });
            widgets::footnote(&mut c, t, f, track.y + track.h, l.pick(texts::installing::CANCEL_RUNS_NOTHING))
        }
        Install::Scripting { stage, .. } => {
            let (detail, done) = script_stage(l, stage.as_ref(), &s.script_output);
            let y = widgets::progress_card(&mut c, t, f, top, l.pick(texts::installing::SCRIPT), detail, done);
            let y = widgets::footnote(&mut c, t, f, y, until_installed(l, s.source));
            script_output(&mut c, cx, y, &mut rows)
        }
        Install::Wiping => {
            let y = widgets::progress_card(&mut c, t, f, top, l.pick(texts::wipe::WIPING), factory_reset(s), None);
            widgets::footnote(&mut c, t, f, y, l.pick(texts::wipe::KEEP_POWER))
        }
        Install::Wiped(Ok(())) => {
            let made_anew = l.pick(texts::installing::DATA_MADE_ANEW);
            let y = widgets::list(&mut c, t, f, top, &[standing(l.pick(texts::wipe::WIPED), GREEN, Icon::Wipe, made_anew)]).bottom;
            widgets::footnote(&mut c, t, f, y, l.pick(texts::installing::MORE_COMMANDS))
        }
        Install::Wiped(Err(stop)) => {
            let (what, detail) = fmt::erase_stop(l, stop);
            let mut y = widgets::list(&mut c, t, f, top, &[standing(l.pick(texts::wipe::INCOMPLETE), RED, Icon::Wipe, &what)]).bottom;
            if let Some(d) = detail {
                y = widgets::footnote(&mut c, t, f, y, &d);
            }
            widgets::footnote(&mut c, t, f, y, l.pick(texts::installing::MORE_COMMANDS))
        }
        Install::Finished { outcome, damage } => finished(&mut c, cx, top, outcome, *damage, &mut out, &mut rows),
    };
    // What the system asked for says why, where it said.
    let bottom = match (s.source, &s.asked.reason) {
        (Source::Asked { .. }, Some(reason)) => widgets::info_card(&mut c, t, f, bottom + t.m.dp(12.0), &[(l.pick(texts::installing::REASON), reason.clone())]),
        _ => bottom,
    };
    Frame { rows, ..Frame::new(t, fit(c, t, bottom), out, install_title(l, s.source), top) }
}

/// The package the system asked for, by the path it gave, at `y`.
fn asked_package(c: &mut Canvas, cx: &Ctx, y: f32) -> f32 {
    let name = cx.state.package_name.clone().unwrap_or_default();
    widgets::info_card(c, cx.t, cx.f, y + cx.t.m.dp(12.0), &[(cx.state.lang.pick(texts::installing::PACKAGE), name)])
}

/// What not to do while a package from `source` is installed.
fn until_installed(l: Lang, source: Source) -> &'static str {
    l.pick(match source {
        Source::Computer | Source::Rescue => texts::installing::UNTIL_KEEP_CABLE,
        Source::File(Place::Storage) | Source::Opened { drive: false } | Source::Asked { .. } => texts::installing::UNTIL_KEEP_POWER,
        Source::File(Place::UsbDrive) | Source::Opened { drive: true } => texts::installing::UNTIL_KEEP_USB_DRIVE,
    })
}

/// How an install ended, and where to go from there: another package from
/// the computer, or back to choose another file.
fn finished(
    c: &mut Canvas,
    cx: &Ctx,
    top: f32,
    outcome: &Outcome,
    damage: Option<Damage>,
    out: &mut Vec<Hit>,
    rows: &mut Option<Rows>,
) -> f32 {
    let (t, f, source, l) = (cx.t, cx.f, cx.state.source, cx.state.lang);
    let gap = t.m.card_gap;
    use texts::installed;
    let choose_another = installed::CHOOSE_ANOTHER;
    let (again, another) = match source {
        Source::Computer => (Row::new(l.pick(installed::SEND_AGAIN)).icon(GREEN, Icon::Usb), Action::Sideload),
        Source::File(_) | Source::Opened { .. } => (Row::new(l.pick(choose_another)).icon(GREEN, Icon::Folder), Action::Back),
        Source::Asked { .. } => (Row::new(l.pick(choose_another)).icon(GREEN, Icon::Folder), Action::Open(Screen::Install)),
        Source::Rescue => (Row::new(l.pick(installed::LEAVE_RESCUE)).icon(GREEN, Icon::Usb), Action::Back),
    };
    let restart = Row::new(l.pick(installed::REBOOT_TO_SYSTEM)).icon(ORANGE, Icon::Power);
    let done = l.pick(installed::INSTALLED);
    let nothing_written = l.pick(installed::NOTHING_WRITTEN);
    match outcome {
        Outcome::Installed { slot, .. } => {
            let detail = installed::to_slot(l, slot.letter());
            let y = widgets::list(c, t, f, top, &[standing(done, GREEN, Icon::Install, &detail)]).bottom;
            let next = widgets::list(c, t, f, y + gap, &[restart]);
            out.extend(hits(&next.rows, &[Action::RebootTo(Target::System)]));
            let y = reflashed(c, cx, next.bottom);
            afterwards_of(c, cx, y, true)
        }
        Outcome::Flashed { partition, slots } => {
            let detail = match slots.as_slice() {
                [slot] => installed::flashed_one(l, partition, slot.letter()),
                [a, b] => installed::flashed_two(l, partition, a.letter(), b.letter()),
                other => unreachable!("a phone has two slots, and the image went to {other:?}"),
            };
            let y = widgets::list(c, t, f, top, &[standing(l.pick(installed::FLASHED), GREEN, Icon::Install, &detail)]).bottom;
            let mut rows = vec![restart];
            let mut acts = vec![Action::RebootTo(Target::System)];
            if *partition == "recovery" {
                rows.push(Row::new(l.pick(installed::REBOOT_TO_RECOVERY)).icon(ORANGE, Icon::Power));
                acts.push(Action::RebootTo(Target::Recovery));
            }
            rows.push(again);
            acts.push(another);
            let next = widgets::list(c, t, f, y + gap, &rows);
            out.extend(hits(&next.rows, &acts));
            afterwards_of(c, cx, next.bottom, true)
        }
        Outcome::Ran { .. } => {
            let ran = l.pick(installed::SCRIPT_RAN);
            let y = widgets::list(c, t, f, top, &[standing(done, GREEN, Icon::Install, ran)]).bottom;
            let next = widgets::list(c, t, f, y + gap, &[restart, again]);
            out.extend(hits(&next.rows, &[Action::RebootTo(Target::System), another]));
            let y = reflashed(c, cx, next.bottom);
            let y = afterwards_of(c, cx, y, true);
            script_output(c, cx, y, rows)
        }
        Outcome::SystemInstalled { name } => {
            let detail = installed::second_system(l, name);
            let y = widgets::list(c, t, f, top, &[standing(done, GREEN, Icon::Systems, &detail)]).bottom;
            let next = widgets::list(c, t, f, y + gap, &[Row::new(l.pick(texts::page::DUAL_BOOT)).icon(PURPLE, Icon::Systems)]);
            out.extend(hits(&next.rows, &[Action::Open(Screen::Systems)]));
            afterwards_of(c, cx, next.bottom, true)
        }
        Outcome::Cancelled => {
            let y = widgets::list(c, t, f, top, &[standing(l.pick(installed::CANCELLED), SLATE, Icon::Install, nothing_written)]).bottom;
            let next = widgets::list(c, t, f, y + gap, &[again]);
            out.extend(hits(&next.rows, &[another]));
            next.bottom
        }
        Outcome::Refused(r) => {
            let reason = fmt::refusal(l, r, &cx.info.codename, source, cx.state.time_zone);
            let y = widgets::list(c, t, f, top, &[standing(l.pick(installed::CANT_INSTALL), ORANGE, Icon::Install, &reason)]).bottom;
            // What the system asked for and did not go in leaves the phone
            // where it was, from which it may go back to the system.
            let (rows, acts) = match source {
                Source::Asked { .. } => (vec![again, restart], vec![another, Action::RebootTo(Target::System)]),
                _ => (vec![again], vec![another]),
            };
            let next = widgets::list(c, t, f, y + gap, &rows);
            out.extend(hits(&next.rows, &acts));
            let y = widgets::footnote(c, t, f, next.bottom, nothing_written);
            afterwards_of(c, cx, y, false)
        }
        Outcome::Failed(failure) => {
            let (what, detail) = fmt::failure(l, failure, source);
            let y = widgets::list(c, t, f, top, &[standing(l.pick(installed::FAILED), RED, Icon::Install, &what)]).bottom;
            let next = widgets::list(c, t, f, y + gap, &[again, restart]);
            out.extend(hits(&next.rows, &[another, Action::RebootTo(Target::System)]));
            // The package is sent again, or installed or flashed again.
            let sent = matches!(source, Source::Computer | Source::Rescue);
            let warning = match (damage, sent) {
                (Some(Damage::System), true) => l.pick(installed::damage::SYSTEM_SEND).into(),
                (Some(Damage::System), false) => l.pick(installed::damage::SYSTEM_INSTALL).into(),
                (Some(Damage::Partition(p)), true) => installed::damage::partition_send(l, p),
                (Some(Damage::Partition(p)), false) => installed::damage::partition_flash(l, p),
                (Some(Damage::Script), true) => l.pick(installed::damage::SCRIPT_SEND).into(),
                (Some(Damage::Script), false) => l.pick(installed::damage::SCRIPT_INSTALL).into(),
                (None, _) => l.pick(installed::damage::NONE).to_string(),
            };
            let mut y = widgets::footnote(c, t, f, next.bottom, &warning);
            if let Some(d) = detail {
                y = widgets::footnote(c, t, f, y, &d);
            }
            let y = reflashed(c, cx, y);
            let y = afterwards_of(c, cx, y, false);
            script_output(c, cx, y, rows)
        }
    }
}

/// What comes after an install, where it is not the user's to choose:
/// rescue waits for the computer's next command, and a package the system
/// asked for, once in, has the phone leave on its own.
fn afterwards_of(c: &mut Canvas, cx: &Ctx, y: f32, installed: bool) -> f32 {
    let l = cx.state.lang;
    let leaving = match cx.state.asked.shutdown_after {
        true => texts::installed::leaving::POWER_OFF,
        false => texts::installed::leaving::REBOOT,
    };
    match cx.state.source {
        Source::Rescue => widgets::footnote(c, cx.t, cx.f, y, l.pick(texts::installing::MORE_COMMANDS)),
        Source::Asked { .. } if installed => widgets::footnote(c, cx.t, cx.f, y, l.pick(leaving)),
        _ => y,
    }
}

/// What the wipe page lists of [`WIPES`] beyond everything, under the
/// partition each is in.
const WIPE_GROUPS: [(Text, &[Wipe]); 2] = [
    (texts::wipe::part::DATA, &[Wipe::AppData, Wipe::Media, Wipe::AppCaches, Wipe::SystemCaches, Wipe::Art, Wipe::Logs, Wipe::Updates]),
    (texts::wipe::part::CACHE, &[Wipe::Cache]),
];

/// What a factory reset erases, as the wipe page tells it of its first
/// choice, and the page that carries one out for the system tells it.
fn factory_reset(s: &State) -> &'static str {
    s.lang.pick(match s.cache_partition {
        true => texts::wipe::factory_reset::WITH_CACHE,
        false => texts::wipe::factory_reset::WITHOUT_CACHE,
    })
}

/// A choice of the wipe page as it lists it: its title, and exactly what it
/// erases.
fn wipe_row(w: Wipe, s: &State) -> (&'static str, String) {
    use texts::wipe::{part, what};
    let l = s.lang;
    let (title, what): (Text, String) = match w {
        Wipe::All => (texts::wipe::ALL, factory_reset(s).into()),
        Wipe::AppData => (part::APP_DATA, l.pick(what::APP_DATA).into()),
        Wipe::Media => (part::MEDIA, l.pick(what::MEDIA).into()),
        Wipe::AppCaches => (part::APP_CACHES, l.pick(what::APP_CACHES).into()),
        Wipe::SystemCaches => (part::SYSTEM_CACHES, l.pick(what::SYSTEM_CACHES).into()),
        Wipe::Art => (part::ART, l.pick(what::ART).into()),
        Wipe::Logs => match s.phone_logs {
            0 => (part::LOGS, l.pick(what::LOGS).into()),
            n => (part::LOGS, what::logs_and_own(l, n as u64)),
        },
        Wipe::Updates => (part::UPDATES, l.pick(what::UPDATES).into()),
        Wipe::Cache => (texts::wipe::EVERYTHING, l.pick(what::CACHE).into()),
    };
    (l.pick(title), what)
}

/// What to erase: everything, as a factory reset does, or parts of it on
/// their own, the cache partition among them where the phone has one; then
/// a swipe erases what is chosen.
pub fn wipe(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let chosen = |w: Wipe| s.wipe[WIPES.iter().position(|&x| x == w).expect("every choice is one of WIPES")];
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::wipe::RESET));
    let (name, sub) = wipe_row(Wipe::All, s);
    let all = widgets::list(&mut c, t, f, y, &[Row::new(name).subtitle(&sub).trail(Trail::Check(chosen(Wipe::All)))]);
    out.extend(hits(&all.rows, &[Action::WipeItem(Wipe::All)]));
    let mut y = widgets::footnote(&mut c, t, f, all.bottom, l.pick(texts::wipe::STAYS));
    for (title, items) in WIPE_GROUPS {
        let shown: Vec<Wipe> = items.iter().copied().filter(|&w| w != Wipe::Cache || s.cache_partition).collect();
        if shown.is_empty() {
            continue;
        }
        let below = widgets::section(&mut c, t, f, y, l.pick(title));
        let lines: Vec<(&str, String)> = shown.iter().map(|&w| wipe_row(w, s)).collect();
        let rows: Vec<Row> =
            shown.iter().zip(&lines).map(|(&w, (name, sub))| Row::new(name).subtitle(sub).trail(Trail::Check(chosen(w)))).collect();
        let list = widgets::list(&mut c, t, f, below, &rows);
        out.extend(hits(&list.rows, &shown.iter().map(|&w| Action::WipeItem(w)).collect::<Vec<_>>()));
        y = list.bottom;
    }
    let y = widgets::footnote(&mut c, t, f, y, l.pick(texts::wipe::NOTE));
    let (label, danger) = confirm_label(Confirm::Wipe, cx.state);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit { rect: track, action: Action::Confirm(Confirm::Wipe) });
    Frame::new(t, fit(c, t, track.y + track.h), out, l.pick(texts::page::WIPE), top)
}

/// A page laid out above the on-screen keyboard: where the keyboard's top
/// is, and its keys as touch targets.
fn keyboard_hits(c: &mut Canvas, t: &Theme, f: &Fonts, s: &State, enter: &str, out: &mut Vec<Hit>) -> f32 {
    let top = t.m.h as f32 - widgets::keyboard_height(t);
    let keys = widgets::keyboard(c, t, f, s.lang, top, s.layer, s.key_held(), enter);
    out.extend(keys.into_iter().map(|(rect, cap)| Hit { rect, action: Action::Type(cap) }));
    top
}

/// Finding a feature or partition: the field where the title would be, the
/// button that leaves the search beside it, what it finds, and the
/// keyboard.
pub fn search(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let mut out = Vec::new();
    let (at, cancel) = widgets::search_layout(t);
    let field = widgets::search_field(&mut c, t, f, at.x, at.w, at.y + at.h / 2.0, &s.search, l.pick(texts::search::HINT));
    widgets::cancel_button(&mut c, t, cancel);
    out.push(Hit { rect: cancel, action: Action::CloseSearch });
    let top = field.field.y + field.field.h + m.dp(12.0);
    let kb_top = m.h as f32 - widgets::keyboard_height(t);
    if s.search.trim().is_empty() {
        widgets::footnote(&mut c, t, f, top, l.pick(texts::search::HOW));
    } else {
        let found = search::find_on(&s.search, l, |screen| s.offers(screen));
        if found.is_empty() {
            let none = texts::search::none(l, s.search.trim());
            widgets::list(&mut c, t, f, top, &[Row::new(&none).trail(Trail::None)]);
        } else {
            let rows: Vec<Row> = found.iter().map(|e| Row::new(l.pick(e.title)).subtitle(l.pick(e.place))).collect();
            let list = widgets::list(&mut c, t, f, top, &rows);
            // The keyboard covers what does not fit above it.
            let shown = list.rows.iter().zip(&found).filter(|(r, _)| r.y + r.h <= kb_top);
            out.extend(shown.map(|(&rect, e)| Hit { rect, action: Action::Open(e.screen) }));
        }
    }
    let find = l.pick(texts::search::SEARCH);
    keyboard_hits(&mut c, t, f, s, find, &mut out);
    if let Some(rect) = field.clear {
        out.push(Hit { rect, action: Action::ClearSearch });
    }
    Frame { back: false, header: false, caret: Some(field.caret), ..Frame::new(t, c, out, find, 0.0) }
}

/// Backing up: what to take, where it goes, and how.
pub fn backup(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::backup::CONTENTS));
    let system = fmt::snapshot_backup_note(l, &i.snapshot).unwrap_or(l.pick(texts::backup::SYSTEM_HOLDS));
    let data = l.pick(match s.data == Data::Open {
        true => texts::backup::DATA_HOLDS,
        false => texts::backup::DATA_HOLDS_LOCKED,
    });
    let boot = l.pick(texts::backup::BOOT_HOLDS);
    let modem = l.pick(texts::backup::MODEM_HOLDS);
    let subs = [boot, system, data, modem];
    let rows: Vec<Row> = BACKUP_PARTS
        .iter()
        .zip(subs)
        .enumerate()
        .map(|(k, (name, sub))| Row::new(l.pick(*name)).subtitle(sub).trail(Trail::Check(s.backup[k])))
        .collect();
    let parts = widgets::list(&mut c, t, f, y, &rows);
    out.extend(hits(&parts.rows, &[0, 1, 2, 3].map(Action::BackupPart)));
    let y = widgets::section(&mut c, t, f, parts.bottom, l.pick(texts::backup::SAVE_TO));
    let storage = match (s.data.storage_open(), i.storage.free.map(fmt::gb)) {
        (true, Some(free)) => texts::backup::free(l, free),
        (true, None) => l.pick(texts::backup::FREE_UNKNOWN).to_string(),
        (false, _) if s.data == Data::Erased => l.pick(texts::data::ERASED).to_string(),
        (false, _) => l.pick(texts::decrypt::NEEDED).to_string(),
    };
    let usb = l.pick(match &s.usb_drive {
        Some(UsbDrive::Shown) => texts::backup::CONNECTED,
        Some(UsbDrive::Unreadable(_)) => texts::usb_drive::UNREADABLE,
        None => texts::usb_drive::WANTED,
    });
    let places = widgets::list(&mut c, t, f, y, &[
        Row::new(place_name(l, Place::Storage)).subtitle(&storage).trail(Trail::Check(s.backup_to == Place::Storage)),
        Row::new(place_name(l, Place::UsbDrive)).subtitle(usb).trail(Trail::Check(s.backup_to == Place::UsbDrive)),
    ]);
    out.extend(hits(&places.rows, &[Action::BackupTo(Place::Storage), Action::BackupTo(Place::UsbDrive)]));
    let y = widgets::section(&mut c, t, f, places.bottom, l.pick(texts::common::OPTIONS));
    let options = widgets::list(&mut c, t, f, y, &[
        Row::new(l.pick(texts::backup::COMPRESS)).subtitle(l.pick(texts::backup::COMPRESS_NOTE)).trail(Trail::Switch(s.compress)),
        Row::new(l.pick(texts::backup::VERIFY)).subtitle(l.pick(texts::backup::VERIFY_NOTE)).trail(Trail::Switch(s.verify)),
    ]);
    out.extend(hits(&options.rows, &[Action::Toggle(Switch::Compress), Action::Toggle(Switch::Verify)]));
    let y = widgets::footnote(&mut c, t, f, options.bottom, l.pick(texts::backup::NOTE));
    let (label, danger) = confirm_label(Confirm::Backup, cx.state);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit { rect: track, action: Action::Confirm(Confirm::Backup) });
    Frame::new(t, fit(c, t, track.y + track.h), out, l.pick(texts::page::BACKUP), top)
}

/// The backups found, newest first, to restore one; those that cannot be
/// read are listed after them, to be deleted.
pub fn restore(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::restore::BACKUPS));
    let bottom = if s.backups.is_empty() {
        widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::restore::NONE)).trail(Trail::None)]).bottom
    } else {
        let lines: Vec<(String, String)> = s
            .backups
            .iter()
            .map(|b| match &b.held {
                Ok(h) => (fmt::datetime(l, h.made, s.time_zone), format!("{} · {} · {}", fmt::parts(l, &h.parts), fmt::bytes(h.size), place_name(l, b.place))),
                Err(_) => (b.name.clone(), format!("{} · {}", l.pick(texts::common::UNREADABLE), place_name(l, b.place))),
            })
            .collect();
        let rows: Vec<Row> = lines.iter().map(|(title, sub)| Row::new(title).subtitle(sub)).collect();
        let list = widgets::list(&mut c, t, f, y, &rows);
        out.extend(list.rows.iter().enumerate().map(|(k, &rect)| Hit { rect, action: Action::OpenBackup(k) }));
        list.bottom
    };
    let bottom = widgets::footnote(&mut c, t, f, bottom, l.pick(texts::restore::NOTE));
    Frame::new(t, fit(c, t, bottom), out, l.pick(texts::page::RESTORE), top)
}

/// One backup: when and where it was made, and which of its parts to
/// restore. One made on another phone is not restored here, and one that
/// cannot be read can only be deleted.
pub fn restore_backup(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let (entry, chosen) = s.restore.as_ref().expect("the restore page shows a backup");
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let location = l.pick(texts::common::LOCATION);
    let y = match &entry.held {
        Err(why) => {
            let facts = [(l.pick(texts::restore::FOLDER), entry.name.clone()), (location, place_name(l, entry.place).to_string())];
            let y = widgets::info_card(&mut c, t, f, top, &facts);
            widgets::footnote(&mut c, t, f, y, &texts::restore::unreadable(l, why))
        }
        Ok(held) => {
            let mut facts = vec![
                (l.pick(texts::restore::MADE), fmt::datetime(l, held.made, s.time_zone)),
                (l.pick(texts::common::SIZE), fmt::bytes(held.size)),
                (location, place_name(l, entry.place).to_string()),
            ];
            if let Some(system) = &held.system {
                facts.push((l.pick(texts::restore::SYSTEM_VERSION), system.clone()));
            }
            if let Some(slot) = held.slot {
                facts.push((l.pick(texts::slot::TITLE), slot.letter().to_string()));
            }
            let y = widgets::info_card(&mut c, t, f, top, &facts);
            if held.device != i.codename {
                widgets::footnote(&mut c, t, f, y, &texts::backup::refused::other_phone(l, &held.device))
            } else {
                let y = widgets::section(&mut c, t, f, y, l.pick(texts::restore::CONTENTS));
                let kept: Vec<usize> = (0..BACKUP_PARTS.len()).filter(|&k| held.parts[k]).collect();
                let rows: Vec<Row> = kept.iter().map(|&k| Row::new(l.pick(BACKUP_PARTS[k])).trail(Trail::Check(chosen[k]))).collect();
                let list = widgets::list(&mut c, t, f, y, &rows);
                out.extend(list.rows.iter().zip(&kept).map(|(&rect, &k)| Hit { rect, action: Action::RestorePart(k) }));
                let mut y = widgets::footnote(&mut c, t, f, list.bottom, l.pick(texts::restore::OVERWRITES));
                if chosen[2] {
                    y = widgets::footnote(&mut c, t, f, y, l.pick(texts::restore::DATA_NOTE));
                }
                // The boot partitions' vbmeta checks the system's: one of
                // another version without the other may not start.
                if let (Some(was), Some(now)) = (&held.system, &i.os_version)
                    && was != now
                    && chosen[0] != chosen[1]
                {
                    y = widgets::footnote(&mut c, t, f, y, &texts::restore::other_version(l, was, now));
                }
                let (label, danger) = confirm_label(Confirm::Restore, cx.state);
                let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
                out.push(Hit { rect: track, action: Action::Confirm(Confirm::Restore) });
                track.y + track.h
            }
        }
    };
    let delete = widgets::list(&mut c, t, f, y + t.m.dp(28.0), &[Row::new(l.pick(texts::restore::DELETE)).tint(t.pal.danger).trail(Trail::None)]);
    out.extend(hits(&delete.rows, &[Action::AskDelete]));
    let mut bottom = delete.bottom;
    if s.asking_delete {
        let y = widgets::footnote(&mut c, t, f, delete.bottom, l.pick(texts::restore::ASK_DELETE));
        let (label, danger) = confirm_label(Confirm::DeleteBackup, cx.state);
        let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
        out.push(Hit { rect: track, action: Action::Confirm(Confirm::DeleteBackup) });
        bottom = track.y + track.h;
    }
    Frame::new(t, fit(c, t, bottom), out, l.pick(texts::restore::TITLE), top)
}

/// The share of its pass the part under way has done.
fn part_share(a: &BackupAt) -> f32 {
    if a.of == 0 {
        1.0
    } else {
        share(a.done, a.of).min(1.0)
    }
}

/// How far backing up or restoring has come over all, 0 to 1, and the
/// bytes of the parts the pass under way has gone through, the part under
/// way by its share. Each part counts by its bytes in each pass: a restore
/// checks all its parts first, and a backup checks them last.
fn job_progress(job: &BackupJob, a: &BackupAt) -> (f32, u64) {
    let total: u64 = job.plan.iter().map(|(_, n)| n).sum();
    let before: u64 = job.plan[..a.item].iter().map(|(_, n)| n).sum();
    let passed = before as f64 + f64::from(part_share(a)) * job.plan[a.item].1 as f64;
    let second = job.checks && a.checking != job.restore;
    let passes = if job.checks { 2.0 } else { 1.0 };
    let over = (if second { total as f64 } else { 0.0 } + passed) / (passes * total.max(1) as f64);
    (over as f32, passed as u64)
}

/// What a backup or a restore does to a part as it goes through them:
/// takes it, puts it back, or checks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pass {
    BackUp,
    Restore,
    Verify,
}

impl Pass {
    /// The pass that writes: a restore's, or a backup's.
    fn writing(restore: bool) -> Pass {
        if restore { Pass::Restore } else { Pass::BackUp }
    }

    /// The pass a job is on at `a`.
    fn at(job: &BackupJob, a: &BackupAt) -> Pass {
        if a.checking { Pass::Verify } else { Pass::writing(job.restore) }
    }

    /// Under way: 正在备份, Backing up.
    fn doing(self) -> Text {
        match self {
            Pass::BackUp => texts::backup::job::BACKING_UP,
            Pass::Restore => texts::backup::job::RESTORING,
            Pass::Verify => texts::backup::job::VERIFYING,
        }
    }

    /// Done to a part: 已备份, Backed up.
    fn done(self) -> Text {
        match self {
            Pass::BackUp => texts::backup::job::BACKED_UP,
            Pass::Restore => texts::backup::job::RESTORED,
            Pass::Verify => texts::backup::job::VERIFIED,
        }
    }

    /// Under way on the part `name`: 正在备份 boot.
    fn doing_part(self, l: Lang, name: &str) -> String {
        match self {
            Pass::BackUp => texts::backup::job::backing_up_part(l, name),
            Pass::Restore => texts::backup::job::restoring_part(l, name),
            Pass::Verify => texts::backup::job::verifying_part(l, name),
        }
    }

    /// Under way on a part, `p` hundredths of it done: 正在备份 37%.
    fn doing_share(self, l: Lang, p: u32) -> String {
        match self {
            Pass::BackUp => texts::backup::job::backing_up_share(l, p),
            Pass::Restore => texts::backup::job::restoring_share(l, p),
            Pass::Verify => texts::backup::job::verifying_share(l, p),
        }
    }

    /// How much of the whole is done: 已备份 1.2GB，共 5.0GB.
    fn done_of(self, l: Lang, passed: &str, total: &str) -> String {
        match self {
            Pass::BackUp => texts::backup::job::backed_up_of(l, passed, total),
            Pass::Restore => texts::backup::job::restored_of(l, passed, total),
            Pass::Verify => texts::backup::job::verified_of(l, passed, total),
        }
    }
}

/// Where part `k` of a backup or restore stands, as its row says it.
fn part_state(l: Lang, job: &BackupJob, k: usize) -> String {
    use std::cmp::Ordering;
    use texts::backup::job as said;
    let wrote = Pass::writing(job.restore).done();
    let ended = job.outcome.is_some();
    let a = match (&job.outcome, job.at) {
        (Some(BackupEnd::Saved(_)), _) => return l.pick(if job.checks { said::VERIFIED } else { wrote }).to_string(),
        (Some(BackupEnd::Restored { .. }), _) => return l.pick(wrote).to_string(),
        (_, None) => return l.pick(if ended { said::NOT_STARTED } else { texts::common::WAITING }).to_string(),
        (_, Some(a)) => a,
    };
    let state = match k.cmp(&a.item) {
        Ordering::Less if a.checking => said::VERIFIED,
        Ordering::Less => wrote,
        Ordering::Equal if ended => texts::common::UNFINISHED,
        Ordering::Equal => return Pass::at(job, &a).doing_share(l, percent(part_share(&a))),
        // Ahead of the pass under way: a backup checks its parts once all
        // are written, and a restore writes them once all are checked.
        Ordering::Greater => match (a.checking, job.restore, ended) {
            (true, false, false) => wrote,
            (true, false, true) => said::NOT_VERIFIED,
            (false, false, false) | (true, true, false) => texts::common::WAITING,
            (false, false, true) => said::NOT_BACKED_UP,
            (false, true, false) => said::VERIFIED,
            (_, true, true) => said::NOT_RESTORED,
        },
    };
    l.pick(state).to_string()
}

/// Backing up or restoring: how far it has come over all, each part and
/// where it stands, and a way to stop while stopping breaks nothing. Once
/// it has ended, how, and where to go.
pub fn backup_job(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let job = s.backup_job.as_ref().expect("the backup page shows work");
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    use texts::backup::job as said;
    let icon = if job.restore { Icon::Restore } else { Icon::Backup };
    let writing = job.at.is_some_and(|a| !a.checking);
    let mut y = match &job.outcome {
        None => {
            let (title, detail, done) = match job.at {
                Some(a) => {
                    let (over, passed) = job_progress(job, &a);
                    let total = fmt::bytes(job.plan.iter().map(|(_, n)| n).sum());
                    let pass = Pass::at(job, &a);
                    (pass.doing_part(l, &job.plan[a.item].0), pass.done_of(l, &fmt::bytes(passed), &total), over)
                }
                None if job.restore => (l.pick(said::READING).to_string(), l.pick(said::CHECKING).to_string(), 0.0),
                None => (l.pick(texts::installing::GETTING_READY).to_string(), l.pick(said::SIZING).to_string(), 0.0),
            };
            widgets::progress_card(&mut c, t, f, top, &title, &detail, Some(done))
        }
        Some(end) => {
            let gone = l.pick(if job.restore { texts::installed::NOTHING_WRITTEN } else { said::NOTHING_SAVED });
            let failed = l.pick(if job.restore { said::RESTORE_FAILED } else { said::BACKUP_FAILED });
            let (title, color, detail) = match end {
                BackupEnd::Saved(name) => {
                    let saved = match job.place {
                        Place::Storage => said::saved_to_storage(l, name),
                        Place::UsbDrive => said::saved_to_usb_drive(l, name),
                    };
                    (l.pick(said::BACKUP_COMPLETE), GREEN, saved)
                }
                BackupEnd::Restored { .. } => (l.pick(said::RESTORE_COMPLETE), GREEN, l.pick(said::USES_RESTORED).to_string()),
                BackupEnd::Stopped => (l.pick(texts::common::STOPPED), SLATE, gone.to_string()),
                BackupEnd::Refused(r) => {
                    let title = l.pick(if job.restore { said::CANT_RESTORE } else { said::CANT_BACK_UP });
                    (title, ORANGE, fmt::backup_refusal(l, r, job.restore))
                }
                BackupEnd::Failed(_) if job.restore && writing => (failed, RED, l.pick(said::PARTLY_WRITTEN).to_string()),
                BackupEnd::Failed(_) => (failed, RED, gone.to_string()),
            };
            widgets::list(&mut c, t, f, top, &[standing(title, color, icon, &detail)]).bottom
        }
    };
    match &job.outcome {
        None => {
            let note = match (job.restore, job.at, job.place) {
                (false, _, Place::Storage) => said::BACKUP_KEEP_POWER,
                (false, _, Place::UsbDrive) => said::BACKUP_KEEP_USB_DRIVE,
                (true, Some(a), Place::Storage) if !a.checking => said::RESTORE_KEEP_POWER,
                (true, Some(a), Place::UsbDrive) if !a.checking => said::RESTORE_KEEP_USB_DRIVE,
                (true, _, _) => said::NOTHING_UNTIL_VERIFIED,
            };
            y = widgets::footnote(&mut c, t, f, y, l.pick(note));
            // A restore that has begun writing goes on to its end.
            if !(job.restore && writing) {
                let label = l.pick(if job.stopping { texts::common::STOPPING } else { texts::common::STOP });
                let stop = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(label).trail(Trail::None)]);
                if !job.stopping {
                    out.extend(hits(&stop.rows, &[Action::StopBackup]));
                }
                y = stop.bottom;
            }
        }
        Some(end) => {
            match end {
                BackupEnd::Restored { unmade } if !unmade.is_empty() => {
                    let note = said::users_gone(l, unmade.join(l.pick(texts::list::SEPARATOR)));
                    y = widgets::footnote(&mut c, t, f, y, &note);
                }
                BackupEnd::Failed(why) => y = widgets::footnote(&mut c, t, f, y, why),
                _ => {}
            }
            let (rows, acts): (Vec<Row>, Vec<Action>) = match end {
                BackupEnd::Restored { .. } => (
                    vec![Row::new(l.pick(texts::installed::REBOOT_TO_SYSTEM)).icon(ORANGE, Icon::Power), Row::new(l.pick(texts::common::DONE)).trail(Trail::None)],
                    vec![Action::RebootTo(Target::System), Action::Back],
                ),
                _ => (vec![Row::new(l.pick(texts::common::DONE)).trail(Trail::None)], vec![Action::Back]),
            };
            let next = widgets::list(&mut c, t, f, y + t.m.card_gap, &rows);
            out.extend(hits(&next.rows, &acts));
            y = next.bottom;
        }
    }
    // The parts come after what can be done, which a long list would push
    // off the screen.
    if !job.plan.is_empty() {
        let heading = l.pick(if job.restore { texts::restore::CONTENTS } else { texts::backup::CONTENTS });
        let y0 = widgets::section(&mut c, t, f, y, heading);
        let states: Vec<String> = (0..job.plan.len()).map(|k| part_state(l, job, k)).collect();
        let rows: Vec<Row> = job.plan.iter().zip(&states).map(|((name, _), state)| Row::new(name).trail(Trail::Text(state))).collect();
        y = widgets::list(&mut c, t, f, y0, &rows).bottom;
    }
    let title = l.pick(match (&job.outcome, job.restore) {
        (None, restore) => Pass::writing(restore).doing(),
        (Some(BackupEnd::Saved(_)), _) => said::BACKUP_COMPLETE,
        (Some(BackupEnd::Restored { .. }), _) => said::RESTORE_COMPLETE,
        (Some(_), false) => said::BACKUP_INCOMPLETE,
        (Some(_), true) => said::RESTORE_INCOMPLETE,
    });
    Frame { back: job.outcome.is_some(), ..Frame::new(t, fit(c, t, y), out, title, top) }
}

const DESKTOP_APPS_TITLE: Text = tri("桌面应用", "桌面應用", "Desktop apps");

/// Tools for the experienced: the recovery's own tools, then the apps the
/// desktop carries. The second list is the desktop's own apps less the two
/// that open pages already named above — the recovery and the files — so
/// the two lists never offer the same thing twice. What each row opens is
/// what its icon on the desktop opens, including the ones not built yet,
/// which answer by stopping rather than by opening an empty page.
pub fn advanced(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let tools = widgets::list(&mut c, t, f, top, &[
        Row::new(l.pick(texts::page::FILES)).icon(BLUE, Icon::Folder),
        Row::new(l.pick(texts::page::TERMINAL)).icon(SLATE, Icon::Terminal),
        Row::new(l.pick(texts::page::RELABEL)).icon(PURPLE, Icon::Shield),
        Row::new(l.pick(texts::page::FILESYSTEMS)).icon(RED, Icon::Mount),
        Row::new(l.pick(texts::page::LOGS)).icon(ORANGE, Icon::Log),
        Row::new(l.pick(texts::page::UPDATE)).icon(GREEN, Icon::Layers).trail(Trail::Value(update_value(cx))),
    ]);
    out.extend(hits(&tools.rows, &[
        Action::Open(Screen::Files),
        Action::Open(Screen::Terminal),
        Action::Open(Screen::Relabel),
        Action::Open(Screen::Filesystems),
        Action::Open(Screen::Logs),
        Action::Open(Screen::Update),
    ]));

    let y = widgets::section(&mut c, t, f, tools.bottom, l.pick(DESKTOP_APPS_TITLE));
    let rows: Vec<Row> = DESKTOP_APPS
        .iter()
        .filter(|app| !matches!(app.opens, Opens::Page(Screen::Home | Screen::Files)))
        .map(|app| Row::new(l.pick(app.name)).icon(app.color, app.icon))
        .collect();
    let apps = widgets::list(&mut c, t, f, y, &rows);
    let opens: Vec<Action> = DESKTOP_APPS
        .iter()
        .filter(|app| !matches!(app.opens, Opens::Page(Screen::Home | Screen::Files)))
        .map(|app| match app.opens {
            Opens::Page(screen) => Action::Open(screen),
            Opens::NotYet(tag) => Action::NotReady(tag),
        })
        .collect();
    out.extend(hits(&apps.rows, &opens));

    Frame::new(t, fit(c, t, apps.bottom), out, l.pick(texts::page::ADVANCED), top)
}

/// Giving the data partition's files the SELinux labels the installed
/// system's rules give them: what it does, and a swipe to start it; then
/// how far it has come, a way to stop it, and how it ended.
pub fn relabel(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let title = l.pick(texts::page::RELABEL);
    let Some(job) = &s.relabel else {
        let mut y = widgets::paragraph(&mut c, t, f, top, l.pick(texts::relabel::WHAT));
        let note = match s.data {
            Data::Open | Data::Locked { .. } => None,
            Data::Erased => Some(texts::relabel::DATA_ERASED),
            Data::Closed | Data::Opening | Data::Failed(_) => Some(texts::relabel::OPEN_FIRST),
        };
        if let Some(note) = note {
            y = widgets::footnote(&mut c, t, f, y, l.pick(note));
        }
        let (label, danger) = confirm_label(Confirm::Relabel, cx.state);
        let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
        out.push(Hit { rect: track, action: Action::Confirm(Confirm::Relabel) });
        return Frame::new(t, c, out, title, top);
    };
    let repair = Verb::of(&crate::app::PartKind::Repair);
    let heading = match &job.outcome {
        None => repair.going(l),
        Some(JobEnd::Done) => repair.complete(l),
        Some(JobEnd::Stopped) => l.pick(texts::common::STOPPED).into(),
        Some(JobEnd::Failed(_)) => repair.failed(l),
    };
    let detail = match (job.total, job.checked) {
        (0, _) if job.outcome.is_none() => l.pick(texts::installing::GETTING_READY).to_string(),
        (total, checked) => texts::relabel::checked(l, checked, total),
    };
    let y = widgets::progress_card(&mut c, t, f, top, &heading, &detail, Some(share(job.checked, job.total)));
    match &job.outcome {
        None => {
            let label = l.pick(if job.stopping { texts::common::STOPPING } else { texts::common::STOP });
            let stop = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(label).trail(Trail::None)]);
            if !job.stopping {
                out.extend(hits(&stop.rows, &[Action::StopRelabel]));
            }
        }
        Some(end) => {
            let changed = job.changed;
            let note = match end {
                JobEnd::Done if changed == 0 => l.pick(texts::relabel::NOTHING_NEEDED).to_string(),
                JobEnd::Done => texts::relabel::changed(l, changed),
                JobEnd::Stopped => texts::relabel::stopped(l, changed),
                JobEnd::Failed(why) => texts::relabel::failed(l, why),
            };
            let y = widgets::footnote(&mut c, t, f, y, &note);
            let finish = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(l.pick(texts::common::DONE)).trail(Trail::None)]);
            out.extend(hits(&finish.rows, &[Action::Back]));
        }
    }
    Frame { back: job.outcome.is_some(), ..Frame::new(t, c, out, title, top) }
}

/// The partitions whose filesystems can be repaired or made afresh as
/// another, each with what it holds and its size.
pub fn filesystems(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::mount::PARTITIONS));
    let bottom = match &s.filesystems {
        None => widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::filesystems::READING)).trail(Trail::None)]).bottom,
        Some(Err(why)) => {
            let y = widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::mount::UNREADABLE)).trail(Trail::None)]).bottom;
            widgets::footnote(&mut c, t, f, y, why)
        }
        Some(Ok(rows)) if rows.is_empty() => widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::filesystems::NONE)).trail(Trail::None)]).bottom,
        Some(Ok(rows)) => {
            let subs: Vec<String> = rows.iter().map(|r| format!("{} · {}", fs_held(l, r, &s.data), fmt::bytes(r.size))).collect();
            let shown: Vec<Row> = rows.iter().zip(&subs).map(|(r, sub)| Row::new(&r.name).subtitle(sub)).collect();
            let list = widgets::list(&mut c, t, f, y, &shown);
            out.extend(hits(&list.rows, &(0..rows.len()).map(Action::OpenFilesystem).collect::<Vec<_>>()));
            widgets::footnote(&mut c, t, f, list.bottom, l.pick(texts::filesystems::NOTE))
        }
    };
    Frame::new(t, fit(c, t, bottom), out, l.pick(texts::page::FILESYSTEMS), top)
}

/// What a partition holds, as its row and page name it: the data
/// partition's is read only once it is open, and until then the row says
/// where it stands, as the home page does.
fn fs_held(l: Lang, row: &FsRow, data: &Data) -> String {
    match (&row.filesystem, row.name == DATA_PARTITION) {
        (Some(filesystem), _) => filesystem.clone(),
        (None, true) => fmt::data(l, data).into(),
        (None, false) => l.pick(texts::filesystems::UNRECOGNIZED).into(),
    }
}

/// One partition: what it holds, what can be done to it, and a swipe to do
/// what is chosen.
pub fn filesystem(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let Some(Ok(rows)) = &s.filesystems else { unreachable!("a partition's page opens from the list read") };
    let row = &rows[s.fs_chosen];
    let data = row.name == DATA_PARTITION;
    let facts = [
        (l.pick(texts::filesystems::MOUNT_POINT), row.mount_point.clone()),
        (l.pick(texts::filesystems::FILESYSTEM), fs_held(l, row, &s.data)),
        (l.pick(texts::common::SIZE), fmt::bytes(row.size)),
    ];
    let y = widgets::info_card(&mut c, t, f, top, &facts);
    let y = widgets::section(&mut c, t, f, y, l.pick(texts::common::ACTIONS));
    let chosen = |action: &FsAction| s.fs_action == *action;
    let remakes: Vec<String> = row.others.iter().map(|fs| texts::filesystems::change_to(l, fs)).collect();
    let made: Vec<String> = row.others.iter().map(|_| remade(row, s)).collect();
    let repair = Row::new(l.pick(texts::filesystems::REPAIR))
        .subtitle(l.pick(texts::filesystems::REPAIR_NOTE))
        .trail(Trail::Check(chosen(&FsAction::Repair)));
    let mut shown = vec![repair];
    let mut actions = vec![Action::ChooseFs(FsChoice::Repair)];
    for (k, fs) in row.others.iter().enumerate() {
        shown.push(Row::new(&remakes[k]).subtitle(&made[k]).trail(Trail::Check(chosen(&FsAction::Remake(fs.clone())))));
        actions.push(Action::ChooseFs(FsChoice::Remake(k)));
    }
    let list = widgets::list(&mut c, t, f, y, &shown);
    out.extend(hits(&list.rows, &actions));
    let mut y = list.bottom;
    if row.others.is_empty() {
        y = widgets::footnote(&mut c, t, f, y, l.pick(texts::filesystems::CANT_CHANGE));
    }
    if data {
        y = widgets::footnote(&mut c, t, f, y, l.pick(texts::filesystems::DATA_CLOSES));
    }
    if !row.others.is_empty() {
        y = widgets::footnote(&mut c, t, f, y, l.pick(texts::filesystems::CANT_UNDO));
    }
    let (label, danger) = confirm_label(Confirm::Filesystem, s);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit { rect: track, action: Action::Confirm(Confirm::Filesystem) });
    Frame::new(t, fit(c, t, track.y + track.h), out, partition_title(l, &row.name), top)
}

/// The title of the page of a partition the filesystem page lists.
fn partition_title(l: Lang, name: &str) -> &'static str {
    l.pick(match name {
        DATA_PARTITION => texts::wipe::part::DATA,
        "metadata" => texts::wipe::part::METADATA,
        "cache" => texts::wipe::part::CACHE,
        _ => texts::flash::PARTITION,
    })
}

/// What making `row` afresh as another filesystem erases: the data
/// partition goes as a factory reset goes, with /metadata and the cache
/// partition beside it.
fn remade(row: &FsRow, s: &State) -> String {
    match (row.name == DATA_PARTITION, s.cache_partition) {
        (true, true) => s.lang.pick(texts::filesystems::FORMATS_DATA_AND_CACHE).into(),
        (true, false) => s.lang.pick(texts::filesystems::FORMATS_DATA).into(),
        (false, _) => texts::filesystems::formats(s.lang, &row.name),
    }
}

/// Work on whole partitions: each partition and where it stands, what the
/// program said of the last, and once it has ended, how. What the system
/// asked for leaves on its own once it went through every partition, and
/// offers where to go otherwise; what the user chose goes back to its page.
pub fn part_work(cx: &Ctx) -> Frame {
    use crate::app::{ItemEnd, ItemState, PartKind};
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let work = s.part_work.as_ref().expect("the page of partition work shows work");
    let mut c = long_page(cx);
    let (mut out, mut rows) = (Vec::new(), None);
    let top = t.m.content_top;
    let verb = Verb::of(&work.kind);
    // Work on one partition fails; work through several may get partway.
    let (title, several) = match &work.kind {
        PartKind::Disable => (texts::parts::title::WIPE, true),
        PartKind::Unshare => (texts::parts::title::UNSHARE, true),
        PartKind::Repair => (texts::parts::title::REPAIR, false),
        PartKind::Remake(_) => (texts::parts::title::FORMAT, false),
    };
    let failed = |state: &ItemState| matches!(state, ItemState::Ended(ItemEnd::Failed(_)));
    let short = matches!(work.outcome, Some(Err(_))) || work.items.iter().any(|(_, state)| failed(state));
    let heading = match (&work.outcome, short, several) {
        (None, _, _) => verb.going(l),
        (Some(_), false, _) => verb.complete(l),
        (Some(_), true, true) => verb.incomplete(l),
        (Some(_), true, false) => verb.failed(l),
    };
    let mut y = widgets::section_first(&mut c, t, f, top, &heading);
    if !work.items.is_empty() {
        let states: Vec<String> = work
            .items
            .iter()
            .map(|(_, state)| match state {
                ItemState::Waiting if work.outcome.is_some() => verb.not_done(l),
                ItemState::Waiting => l.pick(texts::common::WAITING).into(),
                ItemState::Working => verb.going(l),
                ItemState::Ended(end) => fmt::item_end(l, end, verb),
            })
            .collect();
        let held: Vec<Option<&str>> = work
            .items
            .iter()
            .map(|(_, state)| match state {
                ItemState::Ended(ItemEnd::NotShared(fs)) if !fs.is_empty() => Some(fs.as_str()),
                _ => None,
            })
            .collect();
        let shown: Vec<Row> = work
            .items
            .iter()
            .zip(&states)
            .zip(&held)
            .map(|(((name, _), state), held)| {
                let row = Row::new(name).trail(Trail::Text(state));
                match held {
                    Some(fs) => row.subtitle(fs),
                    None => row,
                }
            })
            .collect();
        y = widgets::list(&mut c, t, f, y, &shown).bottom;
    }
    let leaving = match s.asked.shutdown_after {
        true => texts::parts::leaving::POWER_OFF,
        false => texts::parts::leaving::REBOOT,
    };
    let mut notes: Vec<String> = Vec::new();
    match &work.outcome {
        None if work.asked => notes.push(l.pick(leaving).into()),
        None => notes.push(verb.hold_on(l)),
        Some(Err(stop)) => {
            let (what, detail) = fmt::part_stop(l, stop, &cx.info.codename);
            notes.push(what);
            notes.extend(detail);
        }
        Some(Ok(())) => {}
    }
    for (name, state) in &work.items {
        if let ItemState::Ended(ItemEnd::Failed(Some(why))) = state {
            notes.push(texts::common::with_reason(l, verb.failed_on(l, name), why));
        }
    }
    for note in &notes {
        y = widgets::footnote(&mut c, t, f, y, note);
    }
    if let (true, Some(reason)) = (work.asked, &s.asked.reason) {
        y = widgets::info_card(&mut c, t, f, y + t.m.dp(12.0), &[(l.pick(texts::installing::REASON), reason.clone())]);
    }
    match (&work.outcome, work.asked) {
        (None, _) => {}
        (Some(_), true) if !short => {}
        // The system's work ran without the slot's partitions, which going
        // on in the recovery brings up first, as leaving fastboot does.
        (Some(_), true) => {
            let below = widgets::section(&mut c, t, f, y, l.pick(texts::common::ACTIONS));
            let rows: Vec<Row> = AFTERWARDS.iter().map(|&(title, _)| Row::new(l.pick(title)).trail(Trail::None)).collect();
            let actions = AFTERWARDS.map(|(_, action)| if action == Action::Dismiss { Action::EnterRecovery } else { action });
            let choices = widgets::list(&mut c, t, f, below, &rows);
            out.extend(hits(&choices.rows, &actions));
            y = choices.bottom;
        }
        (Some(_), false) => {
            let finish = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(l.pick(texts::common::DONE)).trail(Trail::None)]);
            out.extend(hits(&finish.rows, &[Action::Back]));
            y = finish.bottom;
        }
    }
    if !work.output.is_empty() {
        let below = widgets::section(&mut c, t, f, y, l.pick(texts::parts::OUTPUT));
        rows = Some(Rows { edges: widgets::output_edges(&c, t, f, below, &work.output), action: None });
        y = below;
    }
    let back = work.outcome.is_some() && !work.asked;
    Frame { rows, back, ..Frame::new(t, fit(c, t, y), out, l.pick(title), top) }
}

/// The system on the phone now; the newest system the phone maker's
/// server knows, and downloading its full package to install it; and
/// where an update the system installed stands, merging it into the slot it
/// went to: how far a merge under way has come, and a swipe to merge one
/// that waits there for the system to start it.
pub fn update(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let mut out = Vec::new();
    if let Standing::Merging(done) = s.update {
        let share = done as f32;
        let detail = texts::common::share_done(l, percent(share));
        let y = widgets::progress_card(&mut c, t, f, top, l.pick(texts::snapshot::MERGING), &detail, Some(share));
        let y = widgets::footnote(&mut c, t, f, y, l.pick(texts::update::MERGING_NOTE));
        return Frame::new(t, fit(c, t, y), out, l.pick(texts::page::UPDATE), top);
    }
    let mut y = top;
    if let Some(v) = &i.os_version {
        y = widgets::section_first(&mut c, t, f, y, l.pick(texts::common::THIS_PHONE));
        y = widgets::info_card(&mut c, t, f, y, &[(l.pick(texts::update::CURRENT_VERSION), v.clone())]);
    }
    if s.update_server {
        let online_update = l.pick(texts::update::ONLINE);
        y = if y == top { widgets::section_first(&mut c, t, f, y, online_update) } else { widgets::section(&mut c, t, f, y, online_update) };
        y = online(&mut c, cx, y, &mut out);
    }
    let next = match &s.boot_slots {
        Some(Ok(slots)) => Some(slots.active),
        _ => None,
    };
    let (title, detail) = fmt::update(l, &s.update, next);
    let color = match s.update {
        Standing::None => SLATE,
        Standing::Merged => GREEN,
        Standing::Failed(_) | Standing::Unknown => RED,
        _ => ORANGE,
    };
    let mut row = Row::new(title).icon(color, Icon::Layers).trail(Trail::None);
    if let Some(detail) = &detail {
        row = row.subtitle(detail);
    }
    let merging = l.pick(texts::update::MERGING);
    y = if y == top { widgets::section_first(&mut c, t, f, y, merging) } else { widgets::section(&mut c, t, f, y, merging) };
    y = widgets::list(&mut c, t, f, y, &[row]).bottom;
    if s.update == Standing::Waiting {
        let (label, danger) = confirm_label(Confirm::Merge, cx.state);
        let track = widgets::confirm(&mut c, t, f, y + t.m.dp(28.0), label, danger, s.confirm);
        out.push(Hit { rect: track, action: Action::Confirm(Confirm::Merge) });
        y = widgets::footnote(&mut c, t, f, track.y + track.h, l.pick(texts::update::CANT_GO_BACK));
    }
    Frame::new(t, fit(c, t, y), out, l.pick(texts::page::UPDATE), top)
}

/// Asking the update server for the newest system, then its full package:
/// downloading it, how far that has come, and installing it once it is
/// whole; then what to note before installing it.
fn online(c: &mut Canvas, cx: &Ctx, y: f32, out: &mut Vec<Hit>) -> f32 {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let release = match &s.check {
        Check::Asking => return widgets::list(c, t, f, y, &[Row::new(l.pick(texts::update::CHECKING)).trail(Trail::None)]).bottom,
        Check::Answered(Some(r)) => r,
        other => {
            let mut row = Row::new(l.pick(texts::update::CHECK));
            if !s.online() {
                row = row.subtitle(l.pick(texts::update::WLAN_FIRST));
            }
            let check = widgets::list(c, t, f, y, &[row]);
            out.extend(hits(&check.rows, &[Action::CheckUpdate]));
            return match other {
                Check::Failed(why) => widgets::footnote(c, t, f, check.bottom, &fmt::unanswered(l, why)),
                Check::Answered(None) => widgets::footnote(c, t, f, check.bottom, l.pick(texts::update::NONE_KNOWN)),
                _ => check.bottom,
            };
        }
    };
    let newer = i.os_version.as_deref() != Some(release.version.as_str());
    let title = if newer { texts::update::new_version_named(l, &release.version) } else { l.pick(texts::update::UP_TO_DATE).to_string() };
    let size = match release.length {
        Some(length) => fmt::file_size(length),
        None => fmt::package_size(&release.size),
    };
    let package = texts::update::full_package(l, size);
    let mut y = widgets::list(c, t, f, y, &[Row::new(&title).icon(GREEN, Icon::Layers).subtitle(&package).trail(Trail::None)]).bottom;
    match (&s.download, s.have) {
        (Some(job), _) if job.outcome.is_none() => {
            let (heading, detail, share) = match job.progress {
                None => (l.pick(texts::installing::GETTING_READY), String::new(), None),
                Some(p) => downloaded(l, p, job.rate),
            };
            y = widgets::progress_card(c, t, f, y + t.m.card_gap, heading, &detail, share);
            let label = l.pick(if job.stopping { texts::update::PAUSING } else { texts::update::PAUSE });
            let stop = widgets::list(c, t, f, y + t.m.card_gap, &[Row::new(label).trail(Trail::None)]);
            if !job.stopping {
                out.extend(hits(&stop.rows, &[Action::StopDownload]));
            }
            y = stop.bottom;
        }
        (_, Have::Whole(_)) => {
            let row = Row::new(l.pick(texts::page::INSTALL)).subtitle(l.pick(texts::update::DOWNLOADED_TO_STORAGE));
            let install = widgets::list(c, t, f, y + t.m.card_gap, &[row]);
            out.extend(hits(&install.rows, &[Action::InstallDownloaded]));
            y = install.bottom;
        }
        (job, have) => {
            match job.as_ref().and_then(|j| j.outcome.as_ref()) {
                Some(DownloadEnd::Failed(why)) => y = widgets::footnote(c, t, f, y, &fmt::download_failure(l, why)),
                Some(DownloadEnd::Stopped) => y = widgets::footnote(c, t, f, y, l.pick(texts::update::PAUSED)),
                _ => {}
            }
            let title = l.pick(if matches!(have, Have::Part(_)) { texts::update::RESUME } else { texts::update::DOWNLOAD });
            let had = match have {
                Have::Part(done) => Some(texts::update::downloaded(l, fmt::file_size(done))),
                _ => None,
            };
            let sub = if !s.data.storage_open() {
                Some(l.pick(texts::decrypt::NEEDED))
            } else if !s.online() {
                Some(l.pick(texts::update::WLAN_FIRST))
            } else {
                had.as_deref()
            };
            let row = Row::new(title);
            let download = widgets::list(c, t, f, y + t.m.card_gap, &[sub.map_or(row, |sub| row.subtitle(sub))]);
            out.extend(hits(&download.rows, &[Action::Download]));
            y = download.bottom;
        }
    }
    let saved = texts::update::saved_in(l, DOWNLOADS);
    let points = [l.pick(texts::update::KEEPS_DATA), saved.as_str(), l.pick(texts::update::RESUMABLE), l.pick(texts::update::OPTIMIZES)];
    widgets::notes(c, t, f, y + t.m.card_gap, l.pick(texts::common::NOTES), &points)
}

/// What a progress card says of a download, and the share of it done:
/// what an earlier download left being checked, or the rest coming at
/// `rate` bytes a second.
fn downloaded(l: Lang, progress: Progress, rate: u64) -> (&'static str, String, Option<f32>) {
    match progress {
        Progress::Checking { done, total } => (l.pick(texts::update::CHECKING_PART), format!("{} / {}", fmt::file_size(done), fmt::file_size(total)), Some(share(done, total))),
        Progress::Downloading { done, total } => (
            l.pick(texts::update::DOWNLOADING),
            format!("{} / {} · {}/s", fmt::file_size(done), fmt::file_size(total), fmt::file_size(rate)),
            Some(share(done, total)),
        ),
    }
}

/// Skyward's own update: the build running now; Skyward's newest release
/// on GitHub, and updating to it; then what to note before updating.
pub fn skyward_update(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let mut out = Vec::new();
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::common::THIS_PHONE));
    let y = widgets::info_card(&mut c, t, f, y, &[(l.pick(texts::update::CURRENT_VERSION), VERSION.to_string())]);
    let y = widgets::section(&mut c, t, f, y, l.pick(texts::update::ONLINE));
    let y = skyward_online(&mut c, cx, y, &mut out);
    let points = [l.pick(texts::skyward_update::FROM_GITHUB), l.pick(texts::skyward_update::BOTH_SLOTS), l.pick(texts::skyward_update::REBOOT_TO_USE)];
    let y = widgets::notes(&mut c, t, f, y + t.m.card_gap, l.pick(texts::common::NOTES), &points);
    Frame::new(t, fit(c, t, y), out, l.pick(texts::page::SKYWARD_UPDATE), top)
}

/// Asking GitHub for Skyward's newest release, then updating to it: its
/// build for the phone downloaded and written into the recovery partitions,
/// how far that has come, and how it ended.
fn skyward_online(c: &mut Canvas, cx: &Ctx, y: f32, out: &mut Vec<Hit>) -> f32 {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let gap = t.m.card_gap;
    let newest = match &s.skyward_check {
        SkywardCheck::Asking => return widgets::list(c, t, f, y, &[Row::new(l.pick(texts::update::CHECKING)).trail(Trail::None)]).bottom,
        SkywardCheck::Answered(n) => n,
        other => {
            let mut row = Row::new(l.pick(texts::update::CHECK));
            if !s.online() {
                row = row.subtitle(l.pick(texts::update::WLAN_FIRST));
            }
            let check = widgets::list(c, t, f, y, &[row]);
            out.extend(hits(&check.rows, &[Action::CheckSkyward]));
            return match other {
                SkywardCheck::Failed(why) => widgets::footnote(c, t, f, check.bottom, &fmt::unanswered(l, why)),
                _ => check.bottom,
            };
        }
    };
    let version = &newest.version;
    let (title, detail) = match (newest.newer, &newest.build) {
        (true, Some(build)) => (texts::update::new_version_named(l, version), texts::skyward_update::package(l, fmt::file_size(build.size))),
        (true, None) => (texts::update::new_version_named(l, version), l.pick(texts::skyward_update::NO_PACKAGE).to_string()),
        (false, _) => (l.pick(texts::update::UP_TO_DATE).to_string(), texts::skyward_update::newest_is(l, version)),
    };
    let y = widgets::list(c, t, f, y, &[Row::new(&title).icon(GREEN, Icon::Install).subtitle(&detail).trail(Trail::None)]).bottom;
    if !newest.newer || newest.build.is_none() {
        return y;
    }
    match &s.skyward_job {
        Some(job) if job.outcome.is_none() => {
            let (heading, detail, share) = match job.stage {
                None => (l.pick(texts::installing::GETTING_READY), String::new(), None),
                Some(SkywardStage::Download(p)) => downloaded(l, p, job.rate),
                Some(SkywardStage::Write { done, total }) => {
                    let part = share(done, total);
                    (l.pick(texts::skyward_update::WRITING), texts::common::share_done(l, percent(part)), Some(part))
                }
            };
            let y = widgets::progress_card(c, t, f, y + gap, heading, &detail, share);
            if matches!(job.stage, Some(SkywardStage::Write { .. })) {
                return widgets::footnote(c, t, f, y, l.pick(texts::skyward_update::KEEP_POWER));
            }
            let stop = widgets::list(c, t, f, y + gap, &[Row::new(l.pick(if job.stopping { texts::common::STOPPING } else { texts::common::STOP })).trail(Trail::None)]);
            if !job.stopping {
                out.extend(hits(&stop.rows, &[Action::StopSkywardUpdate]));
            }
            stop.bottom
        }
        Some(SkywardJob { outcome: Some(SkywardEnd::Updated(slots)), .. }) => {
            let detail = match slots.as_slice() {
                [slot] => texts::skyward_update::in_one(l, version, slot.letter()),
                [a, b] => texts::skyward_update::in_two(l, version, a.letter(), b.letter()),
                other => unreachable!("a phone has two slots, and Skyward went into {other:?}"),
            };
            let y = widgets::list(c, t, f, y + gap, &[standing(l.pick(texts::skyward_update::SUCCEEDED), GREEN, Icon::Install, &detail)]).bottom;
            let restart = widgets::list(c, t, f, y + gap, &[Row::new(l.pick(texts::installed::REBOOT_TO_RECOVERY)).icon(ORANGE, Icon::Power)]);
            out.extend(hits(&restart.rows, &[Action::RebootTo(Target::Recovery)]));
            restart.bottom
        }
        job => {
            let mut y = y;
            match job.as_ref().and_then(|j| j.outcome.as_ref()) {
                Some(SkywardEnd::Stopped) => y = widgets::footnote(c, t, f, y, l.pick(texts::skyward_update::STOPPED)),
                Some(SkywardEnd::Download(why)) => y = widgets::footnote(c, t, f, y, &fmt::skyward_download_failure(l, why)),
                Some(SkywardEnd::NotWritten(why)) => {
                    y = widgets::footnote(c, t, f, y, l.pick(texts::skyward_update::NOT_WRITTEN));
                    y = widgets::footnote(c, t, f, y, why);
                }
                Some(SkywardEnd::WriteFailed(slots, why)) => {
                    let warning = match slots.as_slice() {
                        [slot] => texts::skyward_update::partly_one(l, slot.letter()),
                        [a, b] => texts::skyward_update::partly_two(l, a.letter(), b.letter()),
                        other => unreachable!("a phone has two slots, and Skyward was written to {other:?}"),
                    };
                    y = widgets::footnote(c, t, f, y, &warning);
                    y = widgets::footnote(c, t, f, y, why);
                }
                Some(SkywardEnd::Updated(_)) => unreachable!("an update that went in shows how it did"),
                None => {}
            }
            let mut row = Row::new(l.pick(texts::skyward_update::DOWNLOAD_AND_UPDATE));
            if !s.online() {
                row = row.subtitle(l.pick(texts::update::WLAN_FIRST));
            }
            let update = widgets::list(c, t, f, y + gap, &[row]);
            out.extend(hits(&update.rows, &[Action::UpdateSkyward]));
            update.bottom
        }
    }
}

/// The WLAN switch; then the network chosen and how joining it goes, and
/// the networks in range, strongest first.
pub fn wlan(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let w = s.wlan.as_ref().expect("the WLAN page shows on a phone whose WLAN is brought up");
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let on = !matches!(w.power, Power::Off | Power::Failed(_));
    let switch = widgets::list(&mut c, t, f, top, &[Row::new(l.pick(texts::page::WLAN)).trail(Trail::Switch(on))]);
    out.extend(hits(&switch.rows, &[Action::Toggle(Switch::Wlan)]));
    let mut y = switch.bottom;
    match &w.power {
        Power::Off => y = widgets::footnote(&mut c, t, f, y, l.pick(texts::wlan::TURN_ON_NOTE)),
        Power::Failed(why) => y = widgets::footnote(&mut c, t, f, y, &texts::wlan::cant_turn_on(l, why)),
        Power::Starting => y = widgets::footnote(&mut c, t, f, y, l.pick(texts::wlan::TURNING_ON_WLAN)),
        Power::On => {
            let locked = |s: Security| !matches!(s, Security::Open | Security::Owe);
            // The network chosen stands out in the accent, as the stock
            // page shows the one it is on.
            if let Some(j) = &w.joined {
                let found = w.networks.iter().find(|n| n.ssid == j.ssid);
                let (signal, bands) = match (&j.stage, found) {
                    (Joining::Connected(link), found) => (Some(link.signal), found.map_or(Bands::of(link.frequency), |n| n.bands)),
                    (_, Some(n)) => (Some(n.signal), n.bands),
                    (_, None) => (None, Bands::default()),
                };
                let tag = fmt::band_tag(bands);
                let mut row = Row::new(&j.name).fan(signal.map_or(0, fmt::signal_level)).subtitle(fmt::joining(l, &j.stage));
                row = row.trail(Trail::Network { locked: locked(j.security) });
                if let Some(tag) = &tag {
                    row = row.tag(tag);
                }
                let joined = widgets::list_toned(&mut c, t, f, y + t.m.card_gap, &[row], &Tone::lit(t));
                out.extend(hits(&joined.rows, &[Action::OpenJoined]));
                out.push(Hit { rect: widgets::round_button(t, joined.rows[0]), action: Action::OpenJoined });
                y = joined.bottom;
                if let Joining::Connected(link) = &j.stage
                    && let Some(note) = link.reach.as_ref().and_then(|r| fmt::reach(l, r))
                {
                    y = widgets::footnote(&mut c, t, f, y, &note);
                }
            }
            let others: Vec<(usize, &Network)> =
                w.networks.iter().enumerate().filter(|(_, n)| w.joined.as_ref().is_none_or(|j| j.ssid != n.ssid)).collect();
            let (below, rescan) = widgets::section_action(&mut c, t, f, y, l.pick(texts::wlan::NEARBY), l.pick(texts::wlan::REFRESH));
            out.push(Hit { rect: rescan, action: Action::Rescan });
            y = below;
            if others.is_empty() {
                y = widgets::footnote(&mut c, t, f, y - t.m.dp(10.0), l.pick(texts::wlan::LOOKING)) + t.m.card_gap;
            }
            let tags: Vec<Option<String>> = others.iter().map(|(_, n)| fmt::band_tag(n.bands)).collect();
            let mut rows: Vec<Row> = others
                .iter()
                .zip(&tags)
                .map(|((_, n), tag)| {
                    let row = Row::new(&n.name).fan(fmt::signal_level(n.signal)).trail(Trail::Network { locked: locked(n.security) });
                    match tag {
                        Some(tag) => row.tag(tag),
                        None => row,
                    }
                })
                .collect();
            // The list ends with adding a network by hand, as the stock one
            // does.
            rows.push(Row::new(l.pick(texts::wlan::ADD_NETWORK)).tint(t.pal.accent).trail(Trail::None));
            let list = widgets::list(&mut c, t, f, y, &rows);
            let mut picks: Vec<Action> = others.iter().map(|(k, _)| Action::JoinNetwork(*k)).collect();
            picks.push(Action::AddNetwork);
            out.extend(hits(&list.rows, &picks));
            // The round buttons come after the rows they sit on, and a touch
            // finds the last control under it.
            for (rect, (k, _)) in list.rows.iter().zip(&others) {
                out.push(Hit { rect: widgets::round_button(t, *rect), action: Action::NetworkDetails(*k) });
            }
            y = list.bottom;
        }
    }
    Frame::new(t, fit(c, t, y), out, l.pick(texts::page::WLAN), top)
}

/// The password of the network chosen: its name, what is typed, a switch
/// to show it, and the keyboard.
pub fn wlan_password(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let network = s.joining.as_ref().expect("the password page is for a network");
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, &network.name);
    let shown = if s.show_password { s.password.clone() } else { "•".repeat(s.password.chars().count()) };
    let (y, caret) = widgets::text_field(&mut c, t, f, y, &shown);
    let show = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(l.pick(texts::wlan::SHOW_PASSWORD)).trail(Trail::Switch(s.show_password))]);
    out.extend(hits(&show.rows, &[Action::Toggle(Switch::ShowPassword)]));
    keyboard_hits(&mut c, t, f, s, l.pick(texts::wlan::CONNECT), &mut out);
    Frame { caret: Some(caret), ..Frame::new(t, c, out, l.pick(texts::wlan::ENTER_PASSWORD), top) }
}

/// A network added by hand: its name, how it is protected, its password
/// where it takes one with the switch that shows it, and the keyboard,
/// which types into the field touched. The protections open over the page
/// from their row, which alone then answers a touch.
pub fn wlan_add(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let a = s.adding.as_ref().expect("the page adds a network");
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = m.content_top;
    let field = |y: f32| Rect::new(m.margin, y, m.w as f32 - m.margin * 2.0, m.dp(56.0));
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::wlan::NETWORK_NAME));
    let (below, mut caret) = widgets::text_field(&mut c, t, f, y, &a.name);
    out.push(Hit { rect: field(y), action: Action::TypeInto(Field::Name) });
    // Held down while its list is open, as the stock row stays.
    let tone = if a.choosing { Tone::page(&t.pressed()) } else { Tone::page(t) };
    let protection = [Row::new(l.pick(texts::wlan::SECURITY_TITLE)).trail(Trail::Choice(fmt::security(l, a.security)))];
    let security = widgets::list_toned(&mut c, t, f, below + m.card_gap, &protection, &tone);
    out.extend(hits(&security.rows, &[Action::ChooseSecurity]));
    if a.security.password() {
        let y = widgets::section(&mut c, t, f, security.bottom, l.pick(texts::wlan::PASSWORD));
        let shown = if s.show_password { s.password.clone() } else { "•".repeat(s.password.chars().count()) };
        let (below, typed) = widgets::text_field(&mut c, t, f, y, &shown);
        out.push(Hit { rect: field(y), action: Action::TypeInto(Field::Password) });
        if a.field == Field::Password {
            caret = typed;
        }
        let show = widgets::list(&mut c, t, f, below + m.card_gap, &[Row::new(l.pick(texts::wlan::SHOW_PASSWORD)).trail(Trail::Switch(s.show_password))]);
        out.extend(hits(&show.rows, &[Action::Toggle(Switch::ShowPassword)]));
    }
    keyboard_hits(&mut c, t, f, s, l.pick(texts::wlan::CONNECT), &mut out);
    if !a.choosing {
        return Frame { caret: Some(caret), ..Frame::new(t, c, out, l.pick(texts::wlan::ADD_NETWORK), top) };
    }
    // A touch anywhere off the list closes it.
    out.clear();
    out.push(Hit { rect: Rect::new(0.0, 0.0, m.w as f32, m.h as f32), action: Action::CloseChoices });
    let names = ADDABLE.map(|p| fmt::security(l, p));
    let chosen = ADDABLE.iter().position(|&p| p == a.security);
    let held = ADDABLE.iter().position(|&p| s.held == Some(Action::PickSecurity(p)));
    let list = widgets::choices(&mut c, t, f, security.rows[0], &names, chosen, held);
    out.extend(hits(&list, &ADDABLE.map(Action::PickSecurity)));
    Frame::new(t, c, out, l.pick(texts::wlan::ADD_NETWORK), top)
}

/// A network's details, in the stock page's grid: for the one chosen, how
/// joining it goes and what it gave, and forgetting it; for any other, how
/// strong and how protected it is.
pub fn wlan_network(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let network = s.details.as_ref().expect("the details page is of a network");
    let joined = s.wlan.as_ref().and_then(|w| w.joined.as_ref()).filter(|j| j.ssid == network.ssid);
    let y = widgets::section_first(&mut c, t, f, top, &network.name);
    let detail = |icon, title, value: String| Detail { icon, title, value };
    let security = detail(Outline::Shield, l.pick(texts::wlan::SECURITY_TITLE), fmt::security(l, network.security).to_string());
    let signal = l.pick(texts::wlan::SIGNAL_STRENGTH);
    let title = l.pick(texts::wlan::DETAILS);
    let Some(j) = joined else {
        let items = [
            detail(Outline::Wifi, l.pick(texts::wlan::BANDS), fmt::bands(l, network.bands)),
            detail(Outline::Signal, signal, fmt::signal(l, network.signal).to_string()),
            security,
        ];
        let y = widgets::detail_grid(&mut c, t, f, y, &items);
        return Frame::new(t, fit(c, t, y), out, title, top);
    };
    let mut items = vec![detail(Outline::Info, l.pick(texts::wlan::STATUS), fmt::joining(l, &j.stage).to_string())];
    match &j.stage {
        Joining::Connected(link) => {
            if let Some(n) = link.generation {
                items.push(detail(Outline::Wifi, l.pick(texts::wlan::STANDARD), fmt::generation(l, n)));
            }
            if let Some(speed) = link.speed {
                items.push(detail(Outline::Speed, l.pick(texts::wlan::LINK_SPEED), format!("{speed} Mbps")));
            }
            items.push(detail(Outline::Signal, signal, fmt::signal(l, link.signal).to_string()));
            items.push(security);
            items.push(detail(Outline::Address, l.pick(texts::wlan::IP_ADDRESS), link.address.to_string()));
            items.push(detail(Outline::Mask, l.pick(texts::wlan::SUBNET_MASK), link.mask.to_string()));
            if let Some(router) = link.router {
                items.push(detail(Outline::Router, l.pick(texts::wlan::ROUTER), router.to_string()));
            }
        }
        _ => {
            items.push(detail(Outline::Signal, signal, fmt::signal(l, network.signal).to_string()));
            items.push(security);
        }
    }
    let y = widgets::detail_grid(&mut c, t, f, y, &items);
    let forget = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(l.pick(texts::wlan::FORGET)).tint(t.pal.danger).trail(Trail::None)]);
    out.extend(hits(&forget.rows, &[Action::LeaveNetwork]));
    Frame::new(t, fit(c, t, forget.bottom), out, title, top)
}

/// The file manager, every entry of the folder shown, folders first.
pub fn files(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    // An entry picked to copy or move goes to the folder shown, from the
    // bar held at the bottom.
    let dock = s.clip.as_ref().map(|clip| {
        let go = l.pick(match clip.op {
            FileOp::Copy => texts::files::COPY_HERE,
            FileOp::Move => texts::files::MOVE_HERE,
        });
        let (canvas, rects) = widgets::dock_bar(t, f, files::page(t), &[l.pick(texts::common::CANCEL), go]);
        Dock { canvas, hits: hits(&rects, &[Action::Unpick, Action::Paste]) }
    });
    let button = Some((widgets::Glyph::Terminal, Action::TerminalHere));
    let root = l.pick(texts::files::ROOT);
    Frame { dock, button, ..listing(cx, &s.files, root, None, l.pick(texts::files::NONE), Action::OpenEntry, l.pick(texts::page::FILES)) }
}

/// A place's folders and the files in them that can be installed, to
/// choose one. A USB drive's page waits for one to be plugged in, and the
/// drive's files show as soon as it is.
pub fn choose(cx: &Ctx, place: Place) -> Frame {
    let (s, l) = (cx.state, cx.state.lang);
    let unreadable;
    let absent = match (place, &s.usb_drive) {
        (Place::UsbDrive, None) => Some((l.pick(texts::choose::NO_USB_DRIVE), l.pick(texts::choose::CONNECT_USB_DRIVE))),
        (Place::UsbDrive, Some(UsbDrive::Unreadable(why))) => {
            unreadable = texts::choose::usb_drive_format(l, why);
            Some((l.pick(texts::usb_drive::UNREADABLE), unreadable.as_str()))
        }
        _ => None,
    };
    let empty = l.pick(texts::choose::NONE);
    listing(cx, s.folder(Screen::Choose(place)), place_name(l, place), absent, empty, Action::ChooseEntry, l.pick(texts::choose::TITLE))
}

/// A folder laid out as the stock file manager lays one out: the path as
/// capsules from `top`, held under the bar once scrolled there, then the
/// entries in one list on the plain page, each row drawn as it scrolls into
/// view and touched for `action`. In place of the list, `absent` says why
/// nothing of the folder can be shown, and `empty` that it holds nothing.
fn listing(
    cx: &Ctx,
    folder: &Folder,
    top: &str,
    absent: Option<(&str, &str)>,
    empty: &str,
    action: fn(usize) -> Action,
    title: &'static str,
) -> Frame {
    let (t, f) = (cx.t, cx.f);
    let bg = files::page(t);
    let mut c = sheet(cx, t.m.h, bg);
    let rows = match (absent, &folder.entries) {
        (Some((note, why)), _) => {
            files::note(&mut c, t, f, note, Some(why));
            None
        }
        (None, Err(why)) => {
            files::note(&mut c, t, f, cx.state.lang.pick(texts::files::UNREADABLE), Some(why));
            None
        }
        (None, Ok(entries)) if entries.is_empty() => {
            files::note(&mut c, t, f, empty, None);
            None
        }
        (None, Ok(entries)) => Some(files::rows(t, f, entries, action)),
    };
    let pinned = Some(files::strip(t, f, top, &folder.path));
    Frame { bg, rows, pinned, ..Frame::new(t, c, Vec::new(), title, t.m.dp(files::STRIP_TOP_DP)) }
}

/// Row `i` of the page's long list over the whole of `c`, sized for it,
/// drawn as the row comes into view.
pub fn row(screen: Screen, cx: &Ctx, i: usize, c: &mut Canvas) {
    match screen {
        Screen::Files | Screen::Choose(_) => {
            let folder = cx.state.folder(screen);
            let entries = folder.entries.as_ref().expect("a folder listed draws its rows");
            files::row(c, cx.t, cx.f, cx.state.lang, &entries[i], folder.listed, cx.state.time_zone);
        }
        Screen::Installing => widgets::output_row(c, cx.t, cx.f, &cx.state.script_output, i),
        Screen::PartWork => {
            let work = cx.state.part_work.as_ref().expect("the page of partition work shows work");
            widgets::output_row(c, cx.t, cx.f, &work.output, i)
        }
        other => unreachable!("{other:?} draws no list row by row"),
    }
}

/// One entry of a folder, a file or a folder: what it is, and what can be
/// done with it. Deleting asks first, with a swipe.
pub fn file_info(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let e = s.file.as_ref().expect("the file page shows an entry");
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let folder = matches!(e.kind, FileKind::Folder { .. });
    let mut facts = vec![
        (l.pick(texts::files::NAME), e.name.clone()),
        (l.pick(texts::common::LOCATION), Path::new(&s.files.path).join(&e.name).to_string_lossy().into_owned()),
    ];
    let holds = l.pick(texts::files::HOLDS);
    match &e.kind {
        FileKind::Link { target, .. } => facts.push((l.pick(texts::files::POINTS_TO), target.clone())),
        FileKind::Folder { items: Some(n) } => facts.push((holds, texts::files::line::items(l, *n as u64))),
        FileKind::Folder { items: None } => facts.push((holds, l.pick(texts::common::UNREADABLE).to_string())),
        FileKind::File | FileKind::Special => facts.push((l.pick(texts::common::SIZE), texts::files::size(l, e.size, fmt::file_size(e.size)))),
    }
    facts.push((l.pick(texts::files::MODIFIED), fmt::datetime(l, e.modified, s.time_zone)));
    facts.push((l.pick(texts::files::PERMISSIONS), texts::files::owned_by(l, fmt::mode(e.mode), e.uid, e.gid)));
    let y = widgets::info_card(&mut c, t, f, top, &facts);
    let mut rows = Vec::new();
    let mut acts = Vec::new();
    // Images are flashed; packages and zips that install themselves are
    // installed.
    if installs(e) {
        let image = Path::new(&e.name).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("img"));
        rows.push(Row::new(l.pick(if image { texts::files::FLASH } else { texts::page::INSTALL })));
        acts.push(Action::InstallEntry);
    }
    if e.kind == FileKind::File {
        rows.push(Row::new(l.pick(texts::files::EDIT)));
        acts.push(Action::EditEntry);
    }
    rows.extend([Row::new(l.pick(texts::files::COPY)), Row::new(l.pick(texts::files::MOVE_TO)), Row::new(l.pick(texts::files::RENAME))]);
    acts.extend([Action::PickEntry(FileOp::Copy), Action::PickEntry(FileOp::Move), Action::RenameEntry]);
    // A link's own permissions mean nothing; its target is changed where
    // it is.
    if !matches!(e.kind, FileKind::Link { .. }) {
        rows.push(Row::new(l.pick(texts::files::CHANGE_PERMISSIONS)));
        acts.push(Action::OpenPermissions);
    }
    rows.push(Row::new(l.pick(texts::files::DELETE)).tint(t.pal.danger).trail(Trail::None));
    acts.push(Action::AskDelete);
    let actions = widgets::list(&mut c, t, f, y + t.m.card_gap, &rows);
    let mut out = hits(&actions.rows, &acts);
    let mut bottom = actions.bottom;
    if s.asking_delete {
        let what = match folder {
            true => texts::files::ask_delete_folder(l, &e.name),
            false => texts::files::ask_delete(l, &e.name),
        };
        let y = widgets::footnote(&mut c, t, f, actions.bottom, &what);
        let (label, danger) = confirm_label(Confirm::DeleteEntry, cx.state);
        let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
        out.push(Hit { rect: track, action: Action::Confirm(Confirm::DeleteEntry) });
        bottom = track.y + track.h;
    }
    let title = l.pick(if folder { texts::files::FOLDER_INFO } else { texts::files::FILE_INFO });
    Frame::new(t, fit(c, t, bottom), out, title, top)
}

/// The permission bits of the entry the file page shows, a switch each that
/// takes effect at once: reading, writing and running for its owner, its
/// group and everyone else, then the set-ID and sticky bits.
pub fn permissions(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let e = s.file.as_ref().expect("the permissions page shows an entry");
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let mode = e.mode & 0o7777;
    let bits = fmt::mode(e.mode);
    use texts::files::mode as said;
    let facts = [(l.pick(texts::files::NAME), e.name.clone()), (l.pick(texts::files::PERMISSIONS), said::bits(l, bits, format!("{mode:04o}")))];
    let mut y = widgets::info_card(&mut c, t, f, top, &facts);
    let mut out = Vec::new();
    let classes = [(said::OWNER, 6), (said::GROUP, 3), (said::OTHERS, 0)];
    let (read, write, run) = (l.pick(said::READ), l.pick(said::WRITE), l.pick(said::EXECUTE));
    let groups = classes.map(|(title, shift)| (l.pick(title), [(read, 4 << shift), (write, 2 << shift), (run, 1 << shift)]));
    let special = (l.pick(said::SPECIAL), [("SUID", 0o4000), ("SGID", 0o2000), (l.pick(said::STICKY), 0o1000)]);
    for (title, bits) in groups.into_iter().chain([special]) {
        y = widgets::section(&mut c, t, f, y, title);
        let rows = bits.map(|(name, bit)| Row::new(name).trail(Trail::Switch(mode & bit != 0)));
        let list = widgets::list(&mut c, t, f, y, &rows);
        out.extend(hits(&list.rows, &bits.map(|(_, bit)| Action::ModeBit(bit))));
        y = list.bottom;
    }
    Frame::new(t, fit(c, t, y), out, l.pick(texts::files::CHANGE_PERMISSIONS), top)
}

/// A new name for the entry the file page shows, typed on the keyboard.
pub fn rename(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let (y, caret) = widgets::text_field(&mut c, t, f, top, &s.new_name);
    widgets::footnote(&mut c, t, f, y, l.pick(texts::files::RENAME_NOTE));
    keyboard_hits(&mut c, t, f, s, l.pick(texts::common::DONE), &mut out);
    Frame { caret: Some(caret), ..Frame::new(t, c, out, l.pick(texts::files::RENAME), top) }
}

/// The folder an entry goes to already holds its name: what to do.
pub fn clash(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let (clip, into, _) = s.clash.as_ref().expect("the clash page shows a clash");
    let name = Path::new(&clip.path).file_name().map_or_else(|| clip.path.clone(), |n| n.to_string_lossy().into_owned());
    use texts::files::clash as said;
    let skip = l.pick(match clip.op {
        FileOp::Copy => said::SKIP_COPY,
        FileOp::Move => said::SKIP_MOVE,
    });
    let mut c = page(cx);
    let top = t.m.content_top;
    let y = widgets::paragraph(&mut c, t, f, top, &said::there(l, into, name));
    let choices = widgets::list(&mut c, t, f, y + t.m.dp(12.0), &[
        Row::new(l.pick(said::REPLACE)).subtitle(l.pick(said::REPLACE_NOTE)),
        Row::new(l.pick(said::SKIP)).subtitle(skip),
        Row::new(l.pick(said::KEEP_BOTH)).subtitle(l.pick(said::KEEP_BOTH_NOTE)),
    ]);
    let out = hits(&choices.rows, &[
        Action::Resolve(ClashChoice::Replace),
        Action::Resolve(ClashChoice::Skip),
        Action::Resolve(ClashChoice::KeepBoth),
    ]);
    Frame::new(t, c, out, l.pick(said::TITLE), top)
}

/// Copying, moving or deleting: how far it has come and a way to stop it,
/// then, if it did not finish, why.
pub fn file_job(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let job = s.job.as_ref().expect("the job page shows work on files");
    let path = match &job.task {
        FileTask::Copy { path, .. } | FileTask::Move { path, .. } | FileTask::Delete { path } => path,
    };
    use texts::files::job as said;
    let name = Path::new(path).file_name().map_or_else(|| path.clone(), |n| n.to_string_lossy().into_owned());
    let share = |(done, total): (u64, u64)| if total == 0 { 0.0 } else { done as f32 / total as f32 };
    // The bar stays while the work measures, so that the button under it
    // does not move. Deleting, and copying entries that hold no bytes, is
    // counted in entries.
    let (detail, done) = match (job.progress, &job.task) {
        (None, _) => (l.pick(texts::installing::GETTING_READY).to_string(), 0.0),
        (Some((bytes, _)), FileTask::Copy { .. }) if bytes.1 > 0 => (said::copied_of(l, fmt::file_size(bytes.0), fmt::file_size(bytes.1)), share(bytes)),
        (Some((bytes, _)), FileTask::Move { .. }) if bytes.1 > 0 => (said::moved_of(l, fmt::file_size(bytes.0), fmt::file_size(bytes.1)), share(bytes)),
        (Some((_, entries)), FileTask::Copy { .. }) => (said::copied_items(l, entries.0, entries.1), share(entries)),
        (Some((_, entries)), FileTask::Move { .. }) => (said::moved_items(l, entries.0, entries.1), share(entries)),
        (Some((_, entries)), FileTask::Delete { .. }) => (said::deleted_items(l, entries.0, entries.1), share(entries)),
    };
    let mut c = page(cx);
    let top = t.m.content_top;
    let y = widgets::progress_card(&mut c, t, f, top, &name, &detail, Some(done));
    let mut out = Vec::new();
    match &job.outcome {
        None => {
            let label = l.pick(if job.stopping { texts::common::STOPPING } else { texts::common::STOP });
            let stop = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(label).trail(Trail::None)]);
            if !job.stopping {
                out.extend(hits(&stop.rows, &[Action::StopJob]));
            }
        }
        Some(end) => {
            let why = match (end, &job.task) {
                (JobEnd::Stopped, FileTask::Copy { .. }) => l.pick(said::COPY_STOPPED).to_string(),
                (JobEnd::Stopped, FileTask::Move { .. }) => l.pick(said::MOVE_STOPPED).to_string(),
                (JobEnd::Stopped, FileTask::Delete { .. }) => l.pick(said::DELETE_STOPPED).to_string(),
                (JobEnd::Failed(why), FileTask::Copy { .. }) => said::copy_failed(l, why),
                (JobEnd::Failed(why), FileTask::Move { .. }) => said::move_failed(l, why),
                (JobEnd::Failed(why), FileTask::Delete { .. }) => said::delete_failed(l, why),
                (JobEnd::Done, _) => unreachable!("work done leaves its page"),
            };
            let y = widgets::footnote(&mut c, t, f, y, &why);
            let finish = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(l.pick(texts::common::DONE)).trail(Trail::None)]);
            out.extend(hits(&finish.rows, &[Action::Back]));
        }
    }
    let title = l.pick(match (&job.task, job.outcome.is_none()) {
        (FileTask::Copy { .. }, true) => said::COPYING,
        (FileTask::Copy { .. }, false) => said::COPY_INCOMPLETE,
        (FileTask::Move { .. }, true) => said::MOVING,
        (FileTask::Move { .. }, false) => said::MOVE_INCOMPLETE,
        (FileTask::Delete { .. }, true) => said::DELETING,
        (FileTask::Delete { .. }, false) => said::DELETE_INCOMPLETE,
    });
    Frame { back: job.outcome.is_some(), ..Frame::new(t, c, out, title, top) }
}

/// A shell on a terminal: the screen it writes, the keys a phone keyboard
/// lacks, and the keyboard, every key of which goes to the shell.
pub fn terminal(cx: &Ctx) -> Frame {
    let (t, f, s) = (cx.t, cx.f, cx.state);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    keyboard_hits(&mut c, t, f, s, s.lang.pick(texts::keyboard::ENTER), &mut out);
    let l = term::layout(t, f, top);
    term::draw(&mut c, t, f, &l, cx.term);
    let held = cx.term.is_some_and(|term| term.ctrl);
    let keys = term::keys(&mut c, t, f, &l, &term::TERM_KEYS, held);
    out.extend(keys.into_iter().map(|(rect, key)| Hit { rect, action: Action::TermKey(key) }));
    Frame { scrollback: Some((l.card, l.pitch)), ..Frame::new(t, c, out, s.lang.pick(texts::page::TERMINAL), top) }
}

/// A text file being edited: where it is, its lines wrapped on a card, the
/// keys a phone keyboard lacks for moving the cursor, and the keyboard.
/// Once it is changed, the round button saves it.
pub fn editor(cx: &Ctx) -> Frame {
    let (t, f, s) = (cx.t, cx.f, cx.state);
    let ed = s.editor.as_ref().expect("the editor page shows a file");
    let mut c = page(cx);
    let mut out = Vec::new();
    keyboard_hits(&mut c, t, f, s, s.lang.pick(texts::keyboard::NEW_LINE), &mut out);
    editor::caption(&mut c, t, f, &ed.path);
    let l = editor::layout(t, f);
    let caret = editor::draw(&mut c, t, f, &l, ed);
    let keys = term::keys(&mut c, t, f, &l, &editor::EDITOR_KEYS, false);
    out.extend(keys.into_iter().map(|(rect, key)| Hit { rect, action: Action::TermKey(key) }));
    let button = ed.changed.then_some((widgets::Glyph::Done, Action::SaveText));
    Frame { scrollback: Some((l.card, l.pitch)), caret, button, ..Frame::new(t, c, out, s.lang.pick(texts::files::EDIT), t.m.content_top) }
}

/// The languages the pages speak, each named in itself, to speak another.
pub fn language(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let rows: Vec<Row> = Lang::ALL.iter().map(|&k| Row::new(k.name()).trail(Trail::Check(k == l))).collect();
    let list = widgets::list(&mut c, t, f, top, &rows);
    let out = hits(&list.rows, &Lang::ALL.map(Action::Language));
    Frame::new(t, fit(c, t, list.bottom), out, l.pick(texts::page::LANGUAGE), top)
}

/// The time zone the status bar's clock and the times on the pages keep.
pub fn time_zone(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let rows: Vec<Row> = TIME_ZONES
        .iter()
        .enumerate()
        .map(|(k, &(name, cities, _))| Row::new(l.pick(name)).subtitle(l.pick(cities)).trail(Trail::Check(k == s.time_zone)))
        .collect();
    let list = widgets::list(&mut c, t, f, top, &rows);
    let picks: Vec<Action> = (0..TIME_ZONES.len()).map(Action::TimeZone).collect();
    let out = hits(&list.rows, &picks);
    let bottom = widgets::footnote(&mut c, t, f, list.bottom, l.pick(texts::time_zone::NOTE));
    Frame::new(t, fit(c, t, bottom), out, l.pick(texts::page::TIME_ZONE), top)
}

/// The file a ramdisk path ends in, without its extension: the key the names
/// file goes by.
fn file_key(path: &str) -> String {
    Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or(path).to_string()
}

/// What the names file calls a wallpaper, or the file's own name where it
/// says nothing.
fn wallpaper_name_of(l: Lang, name: &skyward_config::WallpaperName, key: &str) -> String {
    let picked = if l == Lang::En { &name.en } else { &name.zh };
    if picked.is_empty() { key.to_string() } else { picked.clone() }
}

/// The name of the sound `path` points to, as the names file says it in the
/// language the pages speak; where it says nothing, the file's own name.
fn sound_name(l: Lang, names: &skyward_config::SoundNames, kind: SoundKind, path: &str) -> String {
    let file = file_key(path);
    let named = match kind {
        SoundKind::Ring => names.ring.get(&file),
        SoundKind::Notf => names.notf.get(&file),
    };
    match named {
        Some(n) if l == Lang::En => n.en.clone(),
        Some(n) => n.zh.clone(),
        None => file,
    }
}

/// Which of the two sounds a name is for: the ring the phone rings with or
/// the sound a notification makes.
#[derive(Clone, Copy)]
enum SoundKind {
    Ring,
    Notf,
}

/// One of the four things the personalization page sets: the two sounds and
/// the two wallpapers. Each opens a page listing the names the ramdisk
/// carries for it, as the page of timer sounds lists its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersonalPick {
    Ring,
    Notf,
    Desk,
    Lock,
}

impl PersonalPick {
    /// The words over the page that lists this one's names.
    fn title(self) -> Text {
        match self {
            PersonalPick::Ring => RINGTONE,
            PersonalPick::Notf => SOUND_EFFECTS,
            PersonalPick::Desk => DESK_WALLPAPER,
            PersonalPick::Lock => LOCK_WALLPAPER,
        }
    }

    /// The key the phone is set to now: what the list marks as in use.
    /// Sounds still come from the fixed ramdisk file; a wallpaper comes
    /// from the state's effective key — a saved choice, or the names file's
    /// first entry when none was saved.
    fn in_use(self, s: &State) -> String {
        match self {
            PersonalPick::Ring => file_key(skyward_config::ramdisk::RINGTONE),
            PersonalPick::Notf => file_key(skyward_config::ramdisk::NOTIFICATION),
            PersonalPick::Desk => s.wallpaper_key(true).to_string(),
            PersonalPick::Lock => s.wallpaper_key(false).to_string(),
        }
    }

    /// What the names file calls `key` in `l`, or the key itself where the
    /// file says nothing.
    fn name(self, l: Lang, s: &State, key: &str) -> String {
        match self {
            PersonalPick::Ring => s.sound_names.ring.get(key).map(|n| sound_name_of(l, n, key)),
            PersonalPick::Notf => s.sound_names.notf.get(key).map(|n| sound_name_of(l, n, key)),
            PersonalPick::Desk => s.wallpaper_names.desk.get(key).map(|n| wallpaper_name_of(l, n, key)),
            PersonalPick::Lock => s.wallpaper_names.lock.get(key).map(|n| wallpaper_name_of(l, n, key)),
        }
        .unwrap_or_else(|| key.to_string())
    }
}

/// The keys `pick` can be set to, in the order the page shows them: the
/// wallpapers in the order the names file writes them, so the first one it
/// names is the default and the one at the top; the sounds sorted, since
/// their table has no order of its own. Where the file holds nothing the
/// one in use stands alone, so the page is never blank. The app uses the
/// same order to map a touch back to the key it names.
pub(super) fn pick_keys(s: &State, pick: PersonalPick) -> Vec<String> {
    let mut keys: Vec<String> = match pick {
        PersonalPick::Ring => {
            let mut keys: Vec<String> = s.sound_names.ring.keys().cloned().collect();
            keys.sort();
            keys
        }
        PersonalPick::Notf => {
            let mut keys: Vec<String> = s.sound_names.notf.keys().cloned().collect();
            keys.sort();
            keys
        }
        PersonalPick::Desk => s.wallpaper_names.desk.keys().cloned().collect(),
        PersonalPick::Lock => s.wallpaper_names.lock.keys().cloned().collect(),
    };
    if keys.is_empty() {
        keys.push(pick.in_use(s));
    }
    keys
}

/// The names `pick` can be set to, each with whether it is the one in use.
fn picks(l: Lang, s: &State, pick: PersonalPick) -> Vec<(String, bool)> {
    let now = pick.in_use(s);
    pick_keys(s, pick).into_iter().map(|key| {
        let name = pick.name(l, s, &key);
        (name, key == now)
    }).collect()
}

/// What one of the personalization page's four rows opens: the names that
/// thing can be set to, the one in use ticked. A wallpaper row answers a
/// touch and changes the picture; the sound rows only show what there is,
/// until there is a sound layer to play them with.
pub fn personal_pick(cx: &Ctx, pick: PersonalPick) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let top = t.m.content_top;
    let picked = picks(l, s, pick);
    let rows: Vec<Row> = picked
        .iter()
        .map(|(name, now)| Row::new(name.as_str()).trail(Trail::Check(*now)))
        .collect();
    let list = widgets::list(&mut c, t, f, top, &rows);
    let (acts, note): (Vec<Action>, Text) = match pick {
        PersonalPick::Desk => ((0..picked.len()).map(|at| Action::Wallpaper { desk: true, at }).collect(), WALLPAPER_PICK_NOTE),
        PersonalPick::Lock => ((0..picked.len()).map(|at| Action::Wallpaper { desk: false, at }).collect(), WALLPAPER_PICK_NOTE),
        // The sound layer does not exist, so these rows are for looking at.
        PersonalPick::Ring | PersonalPick::Notf => (Vec::new(), SOUND_PICK_NOTE),
    };
    let bottom = widgets::footnote(&mut c, t, f, list.bottom, l.pick(note));
    Frame::new(t, fit(c, t, bottom), hits(&list.rows, &acts), l.pick(pick.title()), top)
}

/// Sound, ring and wallpaper picked the way the user likes them: a place
/// the four gather while each of them is not built yet. Each row opens the
/// names its thing can be set to, and none of those names can be set.
pub fn personalization(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    // The ring and the notification name the sound each is set to, and the
    // two wallpapers theirs; a touch on any of them opens the list of names
    // it can be set to.
    let y = widgets::section_first(&mut c, t, f, top, l.among("声音", "聲音", "Sound"));
    let ring = sound_name(l, &s.sound_names, SoundKind::Ring, skyward_config::ramdisk::RINGTONE);
    let notf = sound_name(l, &s.sound_names, SoundKind::Notf, skyward_config::ramdisk::NOTIFICATION);
    let sound = widgets::list(&mut c, t, f, y, &[
        Row::new(l.pick(RINGTONE)).icon(BLUE, Icon::Music).trail(Trail::Value(ring.as_str())),
        Row::new(l.pick(SOUND_EFFECTS)).icon(GREEN, Icon::Music).trail(Trail::Value(notf.as_str())),
    ]);
    out.extend(hits(&sound.rows, &[Action::Open(Screen::Personal(PersonalPick::Ring)), Action::Open(Screen::Personal(PersonalPick::Notf))]));
    let y = widgets::section(&mut c, t, f, sound.bottom, l.pick(WALLPAPER_TITLE));
    let desk = PersonalPick::Desk.name(l, s, s.wallpaper_key(true));
    let lock = PersonalPick::Lock.name(l, s, s.wallpaper_key(false));
    let wallpapers = widgets::list(&mut c, t, f, y, &[
        Row::new(l.pick(DESK_WALLPAPER)).icon(PURPLE, Icon::Phone).trail(Trail::Value(desk.as_str())),
        Row::new(l.pick(LOCK_WALLPAPER)).icon(ORANGE, Icon::Lock).trail(Trail::Value(lock.as_str())),
    ]);
    out.extend(hits(&wallpapers.rows, &[Action::Open(Screen::Personal(PersonalPick::Desk)), Action::Open(Screen::Personal(PersonalPick::Lock))]));
    let note = l.among("声音还不能播放，点开只能看到有哪些可选；壁纸点一下就能换。", "聲音還不能播放，點開只能看到有哪些可選；桌布點一下就能換。", "Sound cannot play yet — opening a row only shows what there is to choose from; a wallpaper changes with one tap.");
    let bottom = widgets::footnote(&mut c, t, f, wallpapers.bottom, note);
    Frame::new(t, fit(c, t, bottom), out, l.pick(PERSONALIZATION), top)
}

/// Saving the logs, as a bug report asks for them.
pub fn logs(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::logs::CONTENTS));
    let parts = widgets::list(&mut c, t, f, y, &[
        Row::new(l.pick(texts::logs::RECOVERY_LOG)).subtitle(l.pick(texts::logs::RECOVERY_LOG_NOTE)).trail(Trail::Check(s.logs[0])),
        Row::new(l.pick(texts::logs::KERNEL_LOG)).subtitle(l.pick(texts::logs::KERNEL_LOG_NOTE)).trail(Trail::Check(s.logs[1])),
    ]);
    out.extend(hits(&parts.rows, &[Action::LogPart(0), Action::LogPart(1)]));
    let y = widgets::section(&mut c, t, f, parts.bottom, l.pick(texts::logs::SAVE_TO));
    let storage = l.pick(match cx.state.data.storage_open() {
        true => texts::logs::FOLDER,
        false => texts::decrypt::NEEDED,
    });
    let places = widgets::list(&mut c, t, f, y, &[
        Row::new(l.pick(texts::logs::TEMPORARY)).subtitle(l.pick(texts::logs::TEMPORARY_NOTE)).trail(Trail::Check(s.logs_to == 0)),
        Row::new(place_name(l, Place::Storage)).subtitle(storage).trail(Trail::Check(s.logs_to == 1)),
    ]);
    out.extend(hits(&places.rows, &[Action::LogsTo(0), Action::LogsTo(1)]));
    let y = widgets::footnote(&mut c, t, f, places.bottom, l.pick(texts::logs::NOTE));
    let export = widgets::button(&mut c, t, f, y + t.m.dp(24.0), l.pick(texts::logs::EXPORT));
    out.push(Hit { rect: export, action: Action::ExportLogs });
    Frame::new(t, c, out, l.pick(texts::page::LOGS), top)
}

/// The words the licence gives for a program's notice, which every
/// language but English reads its own rendering ahead of.
const LICENSE_ORIGINAL: &str = "Skyward\nCopyright (C) 2026 Kirk Lin\n\n\
This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.\n\n\
This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.\n\n\
You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.";

/// Skyward's own licence.
pub fn license(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let y = widgets::info_card(&mut c, t, f, top, &[(l.pick(texts::license::TERMS_TITLE), l.pick(texts::license::TERMS).to_string())]);
    let bottom = match l.pick(texts::license::ORIGINAL_ALONE) {
        true => widgets::paragraph(&mut c, t, f, y + t.m.dp(4.0), LICENSE_ORIGINAL),
        false => {
            let y = widgets::paragraph(&mut c, t, f, y + t.m.dp(4.0), l.pick(texts::license::NOTICE));
            let y = widgets::section(&mut c, t, f, y, l.pick(texts::license::ORIGINAL));
            widgets::paragraph(&mut c, t, f, y - t.m.dp(16.0), LICENSE_ORIGINAL)
        }
    };
    Frame::new(t, fit(c, t, bottom), Vec::new(), l.pick(texts::page::LICENSE), top)
}

/// The monospaced typeface's licence, which names the copyright holders on
/// its first lines; the image carries the same file beside the font.
const MONO_LICENSE: &str = include_str!("../../../resources/fonts/KirkMono-OFL.txt");

/// The brand typeface's licence, which names its copyright holders on its
/// first line; the image carries the same file beside the font.
const BRAND_LICENSE: &str = include_str!("../../../resources/fonts/LexendDeca-OFL.txt");

/// The typefaces, as they name themselves, and the terms each is used under.
pub fn font_license(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let (typeface, version, copyright) = (l.pick(texts::fonts::TYPEFACE), l.pick(texts::fonts::VERSION), l.pick(texts::fonts::COPYRIGHT));
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::fonts::INTERFACE_TEXT));
    let y = widgets::info_card(&mut c, t, f, y, &[
        (typeface, f.info.family.clone()),
        (version, f.info.version.clone()),
        (copyright, f.info.copyright.clone().expect("MiSans names its copyright holder")),
    ]);
    let y = widgets::paragraph(&mut c, t, f, y + t.m.dp(4.0), l.pick(texts::fonts::MISANS));
    let y = widgets::section(&mut c, t, f, y, l.among("品牌字标", "品牌字標", "Brand wordmark"));
    let brand_holders: Vec<&str> = BRAND_LICENSE.lines().take_while(|l| l.starts_with("Copyright")).collect();
    let y = widgets::info_card(&mut c, t, f, y, &[
        (typeface, f.brand_info.family.clone()),
        (version, f.brand_info.version.clone()),
        (copyright, brand_holders.join("\n")),
        (l.pick(texts::license::TERMS_TITLE), l.pick(texts::fonts::OFL).to_string()),
    ]);
    let brand = l.among(
        "关于页的字标使用 Lexend Deca 字体，依据 SIL 开源字体许可证 1.1 版免费使用。字体文件未经修改，仅随本镜像提供，不单独分发。\n\n许可证全文位于 /skyward/LexendDeca-OFL.txt，也可在 openfontlicense.org 查看。",
        "關於頁的字標使用 Lexend Deca 字型，依據 SIL 開放字型授權條款 1.1 版免費使用。字型檔案未經修改，僅隨本映像檔提供，不單獨散布。\n\n授權條款全文位於 /skyward/LexendDeca-OFL.txt，也可在 openfontlicense.org 查看。",
        "The About page's wordmark is set in Lexend Deca, used free of charge under the SIL Open Font License, version 1.1. The font file is unchanged, and comes with this image alone, never on its own.\n\nThe license in full is at /skyward/LexendDeca-OFL.txt, and on openfontlicense.org.",
    );
    let y = widgets::paragraph(&mut c, t, f, y + t.m.dp(4.0), brand);
    let y = widgets::section(&mut c, t, f, y, l.pick(texts::fonts::TERMINAL_TEXT));
    let holders: Vec<&str> = MONO_LICENSE.lines().take_while(|l| l.starts_with("Copyright")).collect();
    let y = widgets::info_card(&mut c, t, f, y, &[
        (typeface, f.mono_info.family.clone()),
        (version, f.mono_info.version.clone()),
        (copyright, holders.join("\n")),
        (l.pick(texts::license::TERMS_TITLE), l.pick(texts::fonts::OFL).to_string()),
    ]);
    let bottom = widgets::paragraph(&mut c, t, f, y + t.m.dp(4.0), l.pick(texts::fonts::MONO));
    Frame::new(t, fit(c, t, bottom), Vec::new(), l.pick(texts::page::FONTS), top)
}

/// Which slot the phone starts from next, as the boot control holds it,
/// and what it holds of each; picking the other and swiping switches.
/// While an update is in flight the slot stays as the update set it.
pub fn slot(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let title = l.pick(texts::slot::BOOT_SLOT);
    let Some(current) = i.slot else {
        widgets::list(&mut c, t, f, top, &[Row::new(l.pick(texts::slot::NO_SLOTS)).trail(Trail::None)]);
        return Frame::new(t, c, out, title, top);
    };
    let slots = match &s.boot_slots {
        Some(Ok(slots)) => *slots,
        Some(Err(why)) => {
            let list = widgets::list(&mut c, t, f, top, &[Row::new(l.pick(texts::slot::UNREADABLE)).trail(Trail::None)]);
            widgets::footnote(&mut c, t, f, list.bottom, why);
            return Frame::new(t, c, out, title, top);
        }
        None => return Frame::new(t, c, out, title, top),
    };
    let chosen = s.next_slot.unwrap_or(slots.active);
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::reboot::NEXT_BOOT));
    let titles = [Slot::A, Slot::B].map(|slot| texts::slot::named(l, slot.letter()));
    let subs = [Slot::A, Slot::B].map(|slot| {
        let k = slot.number() as usize;
        l.pick(match (slot == current, slots.bootable[k], slots.successful[k]) {
            (true, false, _) => texts::slot::RUNNING_UNBOOTABLE,
            (true, true, false) => texts::slot::RUNNING_UNSTARTED,
            (true, true, true) => texts::slot::RUNNING,
            (false, false, _) => texts::slot::OTHER_UNBOOTABLE,
            (false, true, false) => texts::slot::OTHER_UNSTARTED,
            (false, true, true) => texts::slot::OTHER,
        })
    });
    let rows: Vec<Row> =
        [Slot::A, Slot::B].iter().enumerate().map(|(k, &slot)| Row::new(&titles[k]).subtitle(subs[k]).trail(Trail::Check(slot == chosen))).collect();
    let list = widgets::list(&mut c, t, f, y, &rows);
    out.extend(hits(&list.rows, &[Slot::A, Slot::B].map(Action::PickSlot)));
    if let Some(note) = fmt::snapshot_slot_note(l, &i.snapshot) {
        widgets::footnote(&mut c, t, f, list.bottom, note);
        return Frame::new(t, c, out, title, top);
    }
    let y = widgets::footnote(&mut c, t, f, list.bottom, l.pick(texts::slot::SWITCH_NOTE));
    let (label, danger) = confirm_label(Confirm::Slot, cx.state);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit { rect: track, action: Action::Confirm(Confirm::Slot) });
    Frame::new(t, c, out, title, top)
}

/// The two systems as big cards one over the other, laid out as the boot
/// slot page lays out its slots: the main system, in the phone's own
/// partitions, and the second, whose files lie in the data partition, or
/// the place to install one, the one that starts next checked. Picking the
/// other and swiping writes its boot images into the slot that starts
/// next, and restarts into it. Under them, the second system, and removing
/// it.
pub fn systems(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let title = l.pick(texts::page::DUAL_BOOT);
    // While the boot partitions are written, or the second system removed,
    // the page only says so: of a switch the system asked for, perhaps
    // before the systems are read.
    if let Some(work) = s.systems_work {
        let (heading, detail) = match work {
            SystemsWork::Switching(to) => {
                let name = match (&s.systems, to) {
                    (Some(Ok(systems)), Who::Installed) => Some(systems.installed.name.as_str()),
                    (Some(Ok(systems)), Who::Other) => systems.other.as_ref().map(|o| o.name.as_str()),
                    _ => None,
                };
                let heading = match name {
                    Some(name) => texts::systems::switching_to(l, name),
                    None => l.pick(texts::systems::SWITCHING).to_string(),
                };
                (heading, l.pick(texts::systems::WRITING_BOOT_IMAGES))
            }
            SystemsWork::Removing => (l.pick(texts::systems::REMOVING).to_string(), l.pick(texts::systems::GIVING_ROOM_BACK)),
        };
        let y = widgets::progress_card(&mut c, t, f, top, &heading, detail, None);
        let y = widgets::footnote(&mut c, t, f, y, l.pick(texts::systems::KEEP_POWER));
        return Frame::new(t, fit(c, t, y), out, title, top);
    }
    let systems = match &s.systems {
        Some(Ok(systems)) => systems,
        Some(Err(why)) => {
            let list = widgets::list(&mut c, t, f, top, &[Row::new(l.pick(texts::systems::UNREADABLE)).trail(Trail::None)]);
            let y = widgets::footnote(&mut c, t, f, list.bottom, why);
            return Frame::new(t, fit(c, t, y), out, title, top);
        }
        None => return Frame::new(t, fit(c, t, top), out, title, top),
    };
    let starts = starting(systems);
    let chosen = s.system_pick.unwrap_or(starts);
    let second_role = l.pick(texts::systems::SECOND);
    // Boot partitions that hold a second system's image nothing is kept of
    // start neither system the page knows.
    let main = SystemCard {
        role: l.pick(texts::systems::MAIN),
        name: &systems.installed.name,
        detail: systems.installed.version.as_deref().unwrap_or_default(),
        mark: SystemMark::Check(chosen == Who::Installed && systems.held != Held::Unrecorded),
    };
    let second = match &systems.other {
        Some(other) => SystemCard { role: second_role, name: &other.name, detail: &other.version, mark: SystemMark::Check(chosen == Who::Other) },
        None => SystemCard {
            role: second_role,
            name: l.pick(texts::systems::INSTALL),
            detail: l.pick(texts::systems::CHOOSE_PACKAGE),
            mark: SystemMark::Opens,
        },
    };
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::reboot::NEXT_BOOT));
    let shown = widgets::system_cards(&mut c, t, f, y, &[main, second]);
    let second_opens = match systems.other {
        Some(_) => Action::PickSystem(Who::Other),
        None => Action::Open(Screen::Install),
    };
    out.extend(hits(&shown.rows, &[Action::PickSystem(Who::Installed), second_opens]));
    let mut y = shown.bottom + t.m.dp(12.0);
    if let Some(why) = &s.system_notice {
        y = widgets::info_card(&mut c, t, f, y, &[(l.pick(texts::systems::SECOND_FAILED), why.clone())]);
    }
    if chosen != starts {
        match (chosen, fmt::snapshot_system_note(l, &i.snapshot)) {
            (Who::Other, Some(note)) => y = widgets::footnote(&mut c, t, f, y, note),
            _ => {
                let (label, danger) = confirm_label(Confirm::SwitchSystem, s);
                let track = widgets::confirm(&mut c, t, f, y + t.m.dp(12.0), label, danger, s.confirm);
                out.push(Hit { rect: track, action: Action::Confirm(Confirm::SwitchSystem) });
                y = track.y + track.h;
            }
        }
    }
    y = widgets::footnote(&mut c, t, f, y, l.pick(texts::systems::WHAT));
    match systems.held {
        Held::Recorded => {}
        Held::Rewritten => y = widgets::footnote(&mut c, t, f, y, l.pick(texts::systems::REWRITTEN)),
        Held::Unrecorded => y = widgets::footnote(&mut c, t, f, y, l.pick(texts::systems::UNRECORDED)),
    }
    if systems.other.is_some() && systems.changed {
        y = widgets::footnote(&mut c, t, f, y, l.pick(texts::systems::CHANGED));
    }
    if let Some(other) = &systems.other {
        y = widgets::section(&mut c, t, f, y, second_role);
        let size = fmt::bytes(other.size);
        let facts = widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::systems::SPACE_USED)).trail(Trail::Text(&size))]);
        let mounted = matches!(s.data, Data::Open | Data::Locked { .. });
        let remove = Row::new(l.pick(texts::systems::REMOVE)).tint(t.pal.danger);
        let (remove, act) = match mounted {
            true => (remove.trail(Trail::None), Action::AskDelete),
            false => (remove.subtitle(l.pick(texts::decrypt::NEEDED)), Action::Open(Screen::Decrypt)),
        };
        let removing = widgets::list(&mut c, t, f, facts.bottom + t.m.card_gap, &[remove]);
        out.extend(hits(&removing.rows, &[act]));
        y = removing.bottom;
        if s.asking_delete && mounted {
            let ask = l.pick(match starts {
                Who::Other => texts::systems::ASK_REMOVE_RUNNING,
                Who::Installed => texts::systems::ASK_REMOVE,
            });
            let y2 = widgets::footnote(&mut c, t, f, y, ask);
            let (label, danger) = confirm_label(Confirm::RemoveSystem, s);
            let track = widgets::confirm(&mut c, t, f, y2 + t.m.dp(24.0), label, danger, s.confirm);
            out.push(Hit { rect: track, action: Action::Confirm(Confirm::RemoveSystem) });
            y = track.y + track.h;
        }
    }
    Frame::new(t, fit(c, t, y), out, title, top)
}

/// The phone's own fastbootd serving the computer: whether it runs, how far
/// the computer has the port, and where to go from here, chosen with the
/// keys and lit where they are.
pub fn fastboot(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = page(cx);
    let top = t.m.content_top;
    let running = l.pick(if s.fastboot.running { texts::fastboot::RUNNING } else { texts::fastboot::NOT_RUNNING });
    let link = l.pick(match s.fastboot.link {
        UsbLink::Connected => texts::fastboot::CONNECTED,
        UsbLink::Connecting => texts::fastboot::CONNECTING,
        UsbLink::Unplugged => texts::fastboot::NO_COMPUTER,
    });
    let slot = i.slot.map(|s| s.letter());
    let mut facts = vec![
        Row::new("fastbootd").trail(Trail::Text(running)),
        Row::new("USB").trail(Trail::Text(link)),
        Row::new(l.pick(texts::specs::CODENAME)).trail(Trail::Text(&i.codename)),
    ];
    if let Some(letter) = slot {
        facts.push(Row::new(l.pick(texts::slot::CURRENT)).trail(Trail::Text(letter)));
    }
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::fastboot::STATUS));
    let state = widgets::list(&mut c, t, f, y, &facts);
    let y = widgets::footnote(&mut c, t, f, state.bottom, l.pick(texts::fastboot::NOTE));
    let y = widgets::section(&mut c, t, f, y, l.pick(texts::fastboot::LEAVE));
    let rows: Vec<Row> = FASTBOOT_CHOICES.iter().map(|&(title, _)| Row::new(l.pick(title)).trail(Trail::None)).collect();
    let choices = widgets::list(&mut c, t, f, y, &rows);
    let actions: Vec<Action> = FASTBOOT_CHOICES.iter().map(|&(_, action)| action).collect();
    let out = hits(&choices.rows, &actions);
    widgets::footnote(&mut c, t, f, choices.bottom, l.pick(texts::fastboot::KEYS));
    Frame { back: false, ..Frame::new(t, c, out, "Fastboot", top) }
}

/// Where to go once what the system asked for has stopped or cannot be
/// done, in the order the pages list them.
const AFTERWARDS: [(Text, Action); 3] = [
    (texts::installed::REBOOT_TO_SYSTEM, Action::RebootTo(Target::System)),
    (texts::afterwards::ENTER_RECOVERY, Action::Dismiss),
    (texts::reboot::target::POWER_OFF, Action::RebootTo(Target::PowerOff)),
];

/// The list of [`AFTERWARDS`] at `y`, and where it ends. Work the user
/// chose in the recovery goes back to its home page, rather than into it.
fn afterwards(c: &mut Canvas, t: &Theme, f: &Fonts, l: Lang, y: f32, asked: bool, out: &mut Vec<Hit>) -> f32 {
    let rows: Vec<Row> = AFTERWARDS
        .iter()
        .map(|&(title, action)| Row::new(l.pick(if action == Action::Dismiss && !asked { texts::afterwards::BACK_HOME } else { title })).trail(Trail::None))
        .collect();
    let choices = widgets::list(c, t, f, y, &rows);
    out.extend(hits(&choices.rows, &AFTERWARDS.map(|(_, action)| action)));
    choices.bottom
}

/// Erasing: each part and how far it has come, and once erasing has
/// stopped, why. What the system asked for leaves on its own once it is
/// done, and offers where to go only if it stops; what the user chose
/// offers where to go once it ends either way.
pub fn erase(cx: &Ctx) -> Frame {
    use std::cmp::Ordering;
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let e = &s.erase;
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let failed = matches!(e.outcome, Some(Err(_)));
    // Only a part that went wrong was under way; whatever else stopped
    // erasing stopped it before any part began.
    let begun = !failed || matches!(e.outcome, Some(Err(EraseStop::Failed(..))));
    let heading = l.pick(match &e.outcome {
        None => texts::wipe::WIPING,
        Some(Ok(())) => texts::wipe::WIPED,
        Some(Err(_)) => texts::wipe::INCOMPLETE,
    });
    let mut y = widgets::section_first(&mut c, t, f, top, heading);
    if !e.parts.is_empty() {
        let states: Vec<String> = (0..e.parts.len())
            .map(|k| match (k.cmp(&e.done), failed) {
                (Ordering::Less, _) => l.pick(texts::wipe::WIPED).into(),
                (Ordering::Equal, false) => match e.progress {
                    Some((done, total)) if total > 0 => texts::wipe::wiping_share(l, percent(share(done, total))),
                    _ => l.pick(texts::wipe::WIPING).into(),
                },
                (Ordering::Equal, true) if begun => l.pick(texts::common::UNFINISHED).into(),
                (Ordering::Greater, false) => l.pick(texts::common::WAITING).into(),
                (Ordering::Equal | Ordering::Greater, true) => l.pick(texts::wipe::NOT_WIPED).into(),
            })
            .collect();
        let rows: Vec<Row> = e.parts.iter().zip(&states).map(|(&part, state)| Row::new(fmt::part(l, part)).trail(Trail::Text(state))).collect();
        y = widgets::list(&mut c, t, f, y, &rows).bottom;
    }
    let notes: Vec<String> = match (&e.outcome, e.asked, e.reset) {
        (Some(Err(stop)), _, _) => {
            let (what, detail) = fmt::erase_stop(l, stop);
            std::iter::once(what).chain(detail).collect()
        }
        (_, true, true) => {
            let leaving = match s.asked.shutdown_after {
                true => texts::erase::ALL_THEN_POWER_OFF,
                false => texts::erase::ALL_THEN_REBOOT,
            };
            vec![l.pick(leaving).into()]
        }
        (_, true, false) => vec![l.pick(if s.asked.shutdown_after { texts::parts::leaving::POWER_OFF } else { texts::parts::leaving::REBOOT }).into()],
        (None, false, _) => vec![l.pick(texts::wipe::KEEP_POWER).into()],
        (Some(Ok(())), false, true) => vec![l.pick(texts::erase::MADE_ANEW).into()],
        (Some(Ok(())), false, false) => Vec::new(),
    };
    for note in &notes {
        y = widgets::footnote(&mut c, t, f, y, note);
    }
    if let (true, Some(reason)) = (e.asked, &s.asked.reason) {
        y = widgets::info_card(&mut c, t, f, y + t.m.dp(12.0), &[(l.pick(texts::installing::REASON), reason.clone())]);
    }
    if failed || (!e.asked && e.outcome.is_some()) {
        let below = widgets::section(&mut c, t, f, y, l.pick(texts::common::ACTIONS));
        y = afterwards(&mut c, t, f, l, below, e.asked, &mut out);
    }
    let title = l.pick(match (e.reset, e.asked) {
        (true, _) => texts::wipe::RESET,
        (false, true) => texts::erase::WIPE_CACHE,
        (false, false) => texts::page::WIPE,
    });
    Frame { back: false, ..Frame::new(t, fit(c, t, y), out, title, top) }
}

/// The system could not start Android and asks whether to erase all user
/// data: start it again, look around the recovery first, or erase.
pub fn cant_start(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let mut y = widgets::paragraph(&mut c, t, f, top, l.pick(texts::cant_start::WHAT));
    if let Some(reason) = &s.asked.reason {
        y = widgets::info_card(&mut c, t, f, y + t.m.dp(12.0), &[(l.pick(texts::installing::REASON), reason.clone())]);
    }
    y = widgets::section(&mut c, t, f, y, l.pick(texts::common::ACTIONS));
    let rows = [
        Row::new(l.pick(texts::common::TRY_AGAIN)).subtitle(l.pick(texts::cant_start::STARTS_AGAIN)).trail(Trail::None),
        Row::new(l.pick(texts::afterwards::ENTER_RECOVERY)).subtitle(l.pick(texts::cant_start::WIPES_NOTHING)).trail(Trail::None),
    ];
    let choices = widgets::list(&mut c, t, f, y, &rows);
    out.extend(hits(&choices.rows, &[Action::RebootTo(Target::System), Action::Dismiss]));
    let y = widgets::footnote(&mut c, t, f, choices.bottom, l.pick(texts::cant_start::CANT_UNDO));
    let (label, danger) = confirm_label(Confirm::FactoryReset, cx.state);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit { rect: track, action: Action::Confirm(Confirm::FactoryReset) });
    Frame { back: false, ..Frame::new(t, c, out, l.pick(texts::cant_start::TITLE), top) }
}

/// Something the system asked of the recovery that it does not do: what
/// was asked, and where to go instead.
pub fn unhandled(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let mut items = vec![(l.pick(texts::unhandled::REQUEST), s.asked.unhandled.clone().unwrap_or_default())];
    if let Some(reason) = &s.asked.reason {
        items.push((l.pick(texts::installing::REASON), reason.clone()));
    }
    let y = widgets::info_card(&mut c, t, f, top, &items);
    let y = widgets::footnote(&mut c, t, f, y, l.pick(texts::unhandled::NOTE));
    let y = widgets::section(&mut c, t, f, y, l.pick(texts::common::ACTIONS));
    afterwards(&mut c, t, f, l, y, true, &mut out);
    Frame { back: false, ..Frame::new(t, c, out, l.pick(texts::unhandled::TITLE), top) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::BackupRefusal;

    fn job(restore: bool, checks: bool) -> BackupJob {
        let plan = vec![("boot".into(), 200), ("system".into(), 800)];
        BackupJob { restore, place: Place::Storage, checks, plan, at: None, outcome: None, stopping: false }
    }

    fn at(item: usize, done: u64, of: u64, checking: bool) -> BackupAt {
        BackupAt { item, done, of, checking }
    }

    /// The hand carries Xiaomi's blue only while the watch runs; standing
    /// still it is a charcoal, the same grey the marks are cut in. Sampling
    /// a point partway down the fine needle, away from the numerals and the
    /// counterweight, tells the two apart without depending on the exact
    /// shade: the running hand is plainly blue, the standing one is not.
    #[test]
    fn the_hand_is_blue_only_while_the_watch_runs() {
        let t = Theme::light(1080, 2400, 2.624);
        let f = crate::font::bundled();
        let (cx, cy, deg) = (540.0, t.m.dp(DIAL_CY), 228.0);
        // A point 60% of the way from the hub toward the tip, on the needle.
        let a = deg * std::f32::consts::PI / 180.0;
        let (ux, uy) = (a.sin(), -a.cos());
        let px = cx + ux * t.m.dp(HAND_TIP) * 0.6;
        let py = cy + uy * t.m.dp(HAND_TIP) * 0.6;
        let i = (py as usize * 1080 + px as usize) * 4;
        let ink = |tint: f32| {
            let mut c = Canvas::new(1080, 2400);
            clock_dial(&mut c, &t.m, &t, &f, cx, cy, deg, tint);
            [c.data[i], c.data[i + 1], c.data[i + 2]]
        };
        let running = ink(1.0);
        let still = ink(0.0);
        assert!(running[2] > running[0] + 60, "a running hand must read blue, got {running:?}");
        assert!((still[0] as i32 - still[2] as i32).abs() < 20, "a standing hand must be grey, got {still:?}");
    }

    /// The timer's hand is the stopwatch's needle drawn thick — a hairline
    /// for a second, a rod for a minute. Three dp off the centre line the
    /// needle has left the face bare while the hour hand still covers it,
    /// which is what tells the two drawings apart.
    #[test]
    fn the_timer_s_hand_is_a_heavier_rod_than_the_second_hand() {
        let t = Theme::light(1080, 2400, 2.624);
        let f = crate::font::bundled();
        let (w, h) = (1080u32, 2400u32);
        let (cx, cy, deg) = (540.0, t.m.dp(DIAL_CY), 0.0);
        // Sixty per cent of the way out to the hour hand's tip, four dp off
        // its centre line: past the needle's edge (1.2 dp each side) and
        // well inside the hour hand's (4.5 dp each side), so only the
        // needle's blown-out shadow reaches the point.
        let px = cx + t.m.dp(4.0);
        let py = cy - t.m.dp(HOUR_HAND.tip) * 0.6;
        let i = (py as usize * w as usize + px as usize) * 4;
        // The face is pale and the hand's ink is dark, so the sum of the
        // three channels tells them apart without pinning either's exact
        // value, which the shadow under the hand is free to thin a little.
        let at = |c: &Canvas| c.data[i] as u32 + c.data[i + 1] as u32 + c.data[i + 2] as u32;
        let mut second = Canvas::new(w, h);
        clock_dial(&mut second, &t.m, &t, &f, cx, cy, deg, 0.0);
        let mut hour = Canvas::new(w, h);
        timer_dial(&mut hour, &t.m, &t, &f, cx, cy, deg, 0.0);
        assert!(at(&second) > at(&hour) + 200, "the hour hand covers four dp off its line where the needle leaves the face bare");
    }

    /// Smartisan's hour hand ends at the hub. Only the stopwatch's needle
    /// runs on behind its centre and widens into a counterweight, so a tail
    /// on the timer's rod would read as a second, smaller hand.
    #[test]
    fn the_timer_s_hour_hand_carries_no_tail_behind_its_hub() {
        let t = Theme::light(1080, 2400, 2.624);
        let (cx, cy) = (540.0, t.m.dp(DIAL_CY));
        let (hour, _) = hand_geom(&t.m, cx, cy, 0.0, &HOUR_HAND);
        assert_eq!(hour.len(), 1, "the rod and nothing behind it");
        let (needle, _) = hand_geom(&t.m, cx, cy, 0.0, &SECOND_HAND);
        assert_eq!(needle.len(), 3, "the needle keeps its line and its weight");
        // At twelve the rod runs up from the hub, so its far end is above
        // the centre. A tail would have put a stroke below it.
        assert!(hour[0].3 < cy, "the hour hand's one stroke runs up, not down");
    }

    /// The clock is laid out on the 411.6 × 914 dp panel the pages were
    /// drawn on. A shorter one keeps its count and its keys where they are
    /// and takes the difference out of the paper between them: the entries
    /// get fewer, and never run over the keys or off the screen.
    #[test]
    fn a_short_panel_takes_the_room_from_the_paper_not_from_the_keys() {
        let tall = Theme::light(1080, 2400, 2.624);
        let short = Theme::light(1080, 1800, 2.624);
        assert!(clock_paper_room(&short.m) < clock_paper_room(&tall.m), "a shorter panel holds fewer entries");
        for h in [2400u32, 2200, 2000, 1800, 1600, 1400, 1200] {
            let m = &Theme::light(1080, h, 2.624).m;
            let (foot, room) = (clock_paper_foot(m), clock_paper_room(m));
            let used = m.dp(PAPER_TOP) + room as f32 * m.dp(PAPER_ROW);
            if room > 0 {
                assert!(used <= foot + 0.01, "at {h} px the paper ends {} dp past its foot", (used - foot) / m.px_per_dp);
            }
            assert!(foot < m.h as f32 - m.margin - m.dp(26.0), "at {h} px the paper reaches into the keys");
        }
    }

    #[test]
    fn a_script_shows_its_last_line_until_skyward_goes_back() {
        let said = ["- Flashing boot".to_string(), "  ".to_string()];
        let l = Lang::Hans;
        assert_eq!(script_stage(l, None, &[]), ("正在运行安装脚本", None));
        assert_eq!(script_stage(Lang::En, None, &[]), ("Running the install script", None));
        assert_eq!(script_stage(l, Some(&Stage::Script { thousandths: 250 }), &said), ("- Flashing boot", Some(0.25)), "past a blank line");
        assert_eq!(script_stage(l, Some(&Stage::Reflashing { done: 1, total: 4 }), &said), ("正在重新刷入 Skyward", Some(0.25)));
        assert_eq!(update_stage(Lang::En, Some(&Stage::Reflashing { done: 3, total: 4 })), ("Flashing Skyward again".to_string(), Some(0.75)));
        assert_eq!(update_stage(Lang::Hant, Some(&Stage::Writing { done: 1, total: 4 })).0, "第 1 步，共 3 步：寫入新系統 25%");
    }

    #[test]
    fn a_backup_checks_its_parts_last_and_a_restore_first() {
        // Halfway through writing system, 600 of the 1000 bytes: the first
        // of two passes, then the second.
        assert_eq!(job_progress(&job(false, true), &at(1, 400, 800, false)), (0.3, 600));
        assert_eq!(job_progress(&job(false, true), &at(1, 400, 800, true)), (0.8, 600));
        assert_eq!(job_progress(&job(false, false), &at(1, 400, 800, false)), (0.6, 600), "without a check, writing is all");
        assert_eq!(job_progress(&job(true, true), &at(0, 100, 200, true)), (0.05, 100));
        assert_eq!(job_progress(&job(true, true), &at(0, 100, 200, false)), (0.55, 100));
    }

    #[test]
    fn each_part_says_where_it_stands() {
        let states = |j: &BackupJob| (0..2).map(|k| part_state(Lang::Hans, j, k)).collect::<Vec<_>>();
        let mut b = job(false, true);
        assert_eq!(states(&b), ["等待中", "等待中"]);
        b.at = Some(at(1, 400, 800, false));
        assert_eq!(states(&b), ["已备份", "正在备份 50%"]);
        b.at = Some(at(0, 50, 200, true));
        assert_eq!(states(&b), ["正在校验 25%", "已备份"]);
        b.outcome = Some(BackupEnd::Failed("the backup does not hold what was stored".into()));
        assert_eq!(states(&b), ["未完成", "未校验"]);
        b.outcome = Some(BackupEnd::Saved("20261021-141639".into()));
        assert_eq!(states(&b), ["已校验", "已校验"]);
        let mut r = job(true, true);
        r.at = Some(at(1, 0, 800, true));
        assert_eq!(states(&r), ["已校验", "正在校验 0%"]);
        r.at = Some(at(0, 200, 400, false));
        assert_eq!(states(&r), ["正在恢复 50%", "已校验"]);
        r.outcome = Some(BackupEnd::Failed("write /dev/block/by-name/boot_b".into()));
        assert_eq!(states(&r), ["未完成", "未恢复"]);
        r.at = None;
        r.outcome = Some(BackupEnd::Refused(BackupRefusal::DataReset));
        assert_eq!(states(&r), ["未开始", "未开始"]);
    }

    #[test]
    fn each_part_says_where_it_stands_in_the_language() {
        let mut b = job(false, true);
        b.at = Some(at(1, 400, 800, false));
        assert_eq!((0..2).map(|k| part_state(Lang::En, &b, k)).collect::<Vec<_>>(), ["Backed up", "Backing up 50%"]);
        let mut r = job(true, true);
        r.at = Some(at(0, 200, 400, false));
        assert_eq!((0..2).map(|k| part_state(Lang::Hant, &r, k)).collect::<Vec<_>>(), ["正在還原 50%", "已驗證"]);
    }
}
