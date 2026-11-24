//! The recovery's pages as one machine: what it holds, what touches, keys
//! and the clock do to it, and what it asks of the runtime in return. Each
//! page's own types, methods and tests are in the module named for it.

use crate::canvas::{Canvas, Run};
use crate::charge;
use crate::color::Rgba;
use crate::editor::{self, Editor};
use crate::files::{self, FileType};
use crate::fling::{Fling, Tracker};
use crate::fmt;
use crate::focus::{self, Group, Spot, Stop};
use crate::font::Fonts;
use crate::geom::Rect;
use crate::glow::{self, GlowClock};
use crate::icons;
use crate::lang::{same, tri, Lang, Text};
use crate::motion::{self, Spring};
use crate::pages::{self, Band, Ctx, PersonalPick};
use crate::sprite::{Blurs, Sprite};
use crate::search;
use crate::settings::Settings;
use crate::term::{self, Term, TermKey};
use crate::texts;
use crate::theme::Theme;
use crate::widgets::{self, Bar, Bolt, Cap, Layer, WlanMark};
use skyward_info::DeviceInfo;
use skyward_install::{Event, ImageKind, Outcome, Place as FlashPlace, Reflash, Refusal, Stage, Target as FlashTarget};
use skyward_net::download::{Failure, Progress};
use skyward_net::github::{Build, Newest};
use skyward_net::release::{Release, Unanswered};
use skyward_net::supplicant::Security;
use skyward_net::wlan::{Joined, Network, Power, Stage as Joining, Traffic, Wlan};
use skyward_systems::{Overview, Who};
use skyward_vab::{BootSlots, Hold, Slot, SnapshotState, Standing};

mod act;
mod backup;
mod battery;
mod calc;
mod clock;
mod decrypt;
mod dialog;
mod draw;
mod desktop;
mod entry;
mod filesystem;
mod foot;
mod frame;
mod input;
mod install;
mod keys;
mod lock;
mod manager;
mod mount;
mod music;
mod nav;
mod notify;
mod power;
mod shade;
mod prefs;
mod reboot;
mod relabel;
mod scroll;
mod skyward;
mod state;
mod store;
mod systems;
mod tasks;
mod terminal;
#[cfg(test)]
mod testing;
mod update;
mod volume;
mod wipe;
mod wlan;

pub use backup::*;
pub use calc::*;
pub use clock::*;
pub use decrypt::*;
use desktop::*;
pub use entry::*;
pub use filesystem::*;
pub use foot::*;
pub use frame::*;
pub use install::*;
pub use manager::*;
pub use mount::*;
pub use music::*;
pub use notify::*;
pub use prefs::*;
pub use shade::*;
pub use reboot::*;
pub use relabel::*;
pub use skyward::*;
pub use state::*;
pub use store::*;
pub use systems::*;
pub use tasks::*;
pub use terminal::*;
pub use update::*;
pub use wipe::*;
pub use wlan::*;

/// The pages of the recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Decrypt,
    /// The simple desktop Project Aether starts on: a plain wallpaper with
    /// the apps laid over it, each an icon that opens its page.
    Desktop,
    /// The calculator the desktop opens.
    Calculator,
    /// The clock the desktop opens: the time counted up and the time counted
    /// down, one face at a time, switched by the bar at the foot of the page.
    Clock,
    /// The clock's settings, opened by the gear at the page's top right: the
    /// sound its timer's end makes, and whether it ticks.
    ClockSettings,
    /// Which of the sounds the ramdisk carries the timer's end makes.
    ClockSound,
    /// The player the desktop opens: a page standing empty while nothing
    /// comes out of it yet, rather than an app that stops the system.
    Music,
    /// The tasks page the desktop opens: the processes the machine runs and
    /// the performance it runs them at, one face at a time. Neither is read
    /// off the phone yet, so both say where they stand.
    Tasks,
    /// The Aether store the desktop opens: what it puts in front, all it
    /// lists and what the phone already has, one face at a time. Nothing is
    /// fetched yet, so each face says where it stands.
    Store,
    Home,
    Settings,
    /// Sound, ring and wallpaper picked the way the user likes them: a
    /// temporary page while each of those is not built yet.
    Personalization,
    /// The names one of the personalization page's four things can be set
    /// to, the one in use ticked. None of them can be set yet, so the page
    /// only lists what there is.
    Personal(PersonalPick),
    Reboot,
    Wipe,
    /// Mounting and unmounting the slot's partitions.
    Mount,
    Install,
    /// A place's folders and the files in them that can be installed, to
    /// choose one.
    Choose(Place),
    /// Installing a package, which the computer sends with `adb sideload`
    /// or which is a file chosen in a place, and how it went.
    Installing,
    About,
    Specs,
    /// How long the screen stays on with nothing touched.
    ScreenOff,
    /// Finding a feature or partition by name, typed on the screen's own
    /// keyboard.
    Search,
    Backup,
    /// The backups found, to restore one.
    Restore,
    /// One backup, and what to restore from it.
    RestoreBackup,
    /// Backing up or restoring under way, and how it ended.
    BackupJob,
    /// Tools for the experienced: files, a terminal, the logs.
    Advanced,
    Files,
    /// One file, and what can be done with it.
    FileInfo,
    /// The permission bits of the entry the file page shows, a switch each.
    Permissions,
    /// A text file, changed on the screen's keyboard and then saved.
    Editor,
    Terminal,
    /// Giving the data partition's files the SELinux labels the installed
    /// system's rules give them: what it does and a swipe to start, then
    /// how far it has come and how it ended.
    Relabel,
    /// The partitions whose filesystems can be repaired or made afresh as
    /// another.
    Filesystems,
    /// One of them: what it holds, and what to do to it, with a swipe to
    /// do it.
    Filesystem,
    /// Work on whole partitions under way, and how it ended: the
    /// filesystem page's, or what the system asked for before the recovery
    /// started.
    PartWork,
    Language,
    /// Whether the clock keeps the installed system's time zone, and the
    /// parts of the world to choose one from.
    TimeZone,
    /// The time zones of one part of the world.
    TimeZones(Region),
    /// Saving the logs, as a bug report asks for them.
    Logs,
    /// Where an update the system installed stands, and merging it into
    /// the slot it went to.
    Update,
    /// Skyward's newest release, and updating to it: its build for the
    /// phone downloaded and written into the recovery partitions.
    SkywardUpdate,
    /// Skyward's own licence.
    License,
    /// The interface typeface and the terms it is used under.
    FontLicense,
    /// Which slot the phone starts from next.
    Slot,
    /// The installed system and the one Skyward installed beside it: which
    /// of them starts, switching between them, and taking the other away.
    Systems,
    /// The phone's own fastbootd serving the computer's fastboot, and where
    /// to go from it. Its only page, worked with the keys: the partitions
    /// the touchscreen runs from are left out while fastbootd rewrites
    /// super.
    Fastboot,
    /// Erasing what the system asked to erase before the recovery started,
    /// all user data or the cache alone: how far it has come, and where to
    /// go if it stops.
    Erase,
    /// The system could not start Android and asks whether to erase all
    /// user data, which may be what keeps it from starting.
    CantStart,
    /// Something the system asked of the recovery that it does not do.
    Unhandled,
    /// A new name for the entry the file page shows, typed on the screen's
    /// keyboard.
    Rename,
    /// The folder an entry is copied or moved to already holds that name;
    /// what to do is the user's to choose.
    Clash,
    /// Copying, moving or deleting under way, and how it ended.
    FileJob,
    /// The WLAN switch, the network chosen and how joining it goes, and
    /// the networks in range.
    Wlan,
    /// The password of the network chosen, typed on the screen's keyboard.
    WlanPassword,
    /// The network joined, and the address and the rest it gave.
    WlanNetwork,
    /// A network added by hand: its name, how it is protected and its
    /// password, typed on the screen's keyboard.
    WlanAdd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confirm {
    Reboot,
    Wipe,
    Flash,
    Backup,
    Restore,
    /// Start next from the slot chosen on the slot page.
    Slot,
    /// Erase all user data, as the system offered when it could not start.
    FactoryReset,
    /// Delete the entry the file page shows.
    DeleteEntry,
    /// Delete the backup the restore page shows.
    DeleteBackup,
    /// Install the update that waits to be agreed to.
    Install,
    /// Install an update that erases all user data after it.
    InstallWipe,
    /// Run the install script of the zip that waits to be agreed to.
    RunScript,
    /// Merge the update that waits in the slot the phone runs from.
    Merge,
    /// Give the data partition's files the SELinux labels the installed
    /// system's rules give them.
    Relabel,
    /// Do to the partition the filesystem page shows what it chose.
    Filesystem,
    /// Switch to the system picked on the systems page, and restart into
    /// it.
    SwitchSystem,
    /// Take the other system away.
    RemoveSystem,
    /// Install the system the installer holds with the room chosen.
    InstallSystem,
}

/// What a control does when touched.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Open(Screen),
    /// A desktop icon whose app is not built yet: the system stops rather
    /// than pretending, and the panel says so. The word is the one the panel
    /// looks the app up by.
    NotReady(&'static str),
    /// Leave the decrypt page for the page it was opened from; the page
    /// the recovery started on, for the home page without keeping it
    /// behind.
    Skip,
    /// Try opening the data partition again, after it did not open.
    OpenData,
    Back,
    Theme { dark: bool },
    Toggle(Switch),
    RebootTarget(usize),
    /// Choose or leave out what the wipe page erases.
    WipeItem(Wipe),
    /// A slider's track: touching or dragging it sets the slider's level.
    Slider(Slider),
    /// Mount or unmount the mount page's partition at this index.
    MountToggle(usize),
    /// A swipe-to-confirm track; dragging the knob to the far end confirms.
    Confirm(Confirm),
    /// The pattern grid on the unlock page.
    Pattern,
    /// Start receiving a package over USB.
    Sideload,
    /// Send the image to one of the partitions it fits.
    FlashPartition(usize),
    /// Send the image to these slots.
    FlashSlots(Pick),
    /// Leave the recovery for this target at once.
    RebootTo(Target),
    /// Leave for this target from the power menu's confirmation: the card
    /// falls back and the page under it goes black first, and the phone is
    /// asked to leave once the black is whole.
    PowerExit(Target),
    /// Stop fastboot and bring the recovery's own pages up, without
    /// restarting.
    EnterRecovery,
    /// Leave what the system asked of the recovery for the recovery's own
    /// pages.
    Dismiss,
    /// Keep the screen on for this choice in [`SCREEN_OFF`].
    ScreenOff(usize),
    /// Keep the time zone at this place in [`TIME_ZONES`].
    TimeZone(usize),
    /// Speak this language.
    Language(Lang),
    /// Tick or untick a part of the backup, by its place in
    /// [`BACKUP_PARTS`].
    BackupPart(usize),
    /// Send the backup to this place.
    BackupTo(Place),
    /// Open a backup found, by its place in the list, to restore it.
    OpenBackup(usize),
    /// Tick or untick a part of the backup being restored.
    RestorePart(usize),
    /// Stop backing up, or checking a backup before it is restored.
    StopBackup,
    /// Tick or untick a log to export: the recovery's, the kernel's.
    LogPart(usize),
    /// Save the logs to the temporary folder, or to internal storage.
    LogsTo(usize),
    /// Save the logs ticked where they are to go.
    ExportLogs,
    /// Pick this slot to start next; a swipe then switches to it.
    PickSlot(Slot),
    /// Pick this system to start next; a swipe then switches to it.
    PickSystem(Who),
    /// Give the root of the system being installed the size at this place
    /// in those offered.
    SystemSize(usize),
    /// A key of the on-screen keyboard.
    Type(Cap),
    /// A key of the calculator.
    Calc(CalcKey),
    /// Show this face of the calculator: the sum, or the units page.
    CalcFace(CalcFace),
    /// Show this face of the player: the songs, or the one being played.
    MusicFace(MusicFace),
    /// Show this shelf of the player's list: all the songs, or the albums,
    /// the artists or the folders they are kept by.
    MusicFilter(MusicFilter),
    /// Show this face of the tasks page: the processes, or the performance.
    TasksFace(TasksFace),
    /// Show this face of the Aether store: the front, everything it lists,
    /// or what the phone already has.
    StoreFace(StoreFace),
    /// Show this face of the clock: the stopwatch, or the timer.
    ClockFace(ClockFace),
    /// Start the face shown counting, or stop it where it is.
    ClockGo,
    /// Put the face shown back: the stopwatch to nothing with its paper
    /// thrown away, the timer to the length it is set for.
    ClockReset,
    /// Take an entry on the stopwatch's paper.
    ClockLap,
    /// Put the timer-end notice away and take the timer back to nothing.
    TimerDoneOk,
    /// An entry of the power menu was taken. Power off, restart and suspend
    /// ask for a sheet to confirm them; cancel only puts the menu away.
    PowerMenuChoice(power::PowerChoice),
    /// The cancelling key of a sheet: put it away and leave the page as it
    /// was, whatever was under it — desktop, app or lock screen.
    SheetCancel,
    /// Set the timer for this many minutes, from zero to the ruler's three
    /// hours.
    ClockSpan(u64),
    /// Make the sound at this place in the sounds the timer's end can make.
    ClockSound(usize),
    /// Pick one of the personalization page's wallpapers: which of the two
    /// pictures to set, and the place of the choice in the page's list.
    Wallpaper { desk: bool, at: usize },
    /// A key of the row above the keyboard, the terminal's or the editor's.
    TermKey(TermKey),
    /// Empty the search field.
    ClearSearch,
    /// Leave the search, by the round button beside its field.
    CloseSearch,
    /// Open the file manager's entry at this place in the folder: a folder
    /// to look in, a text file to edit, or what can be done with any other.
    OpenEntry(usize),
    /// Open the terminal in the folder the file manager shows.
    TerminalHere,
    /// Open the entry at this place in a place's folder: a folder to look
    /// in, or the file to install.
    ChooseEntry(usize),
    /// Go up to the folder this many levels below the top of the list
    /// shown, by its place in the path shown over the list.
    OpenAncestor(usize),
    /// Pick the entry the file page shows to copy or move, and look for
    /// where it goes.
    PickEntry(FileOp),
    /// Copy or move the entry picked into the folder shown.
    Paste,
    /// Put the entry picked down again.
    Unpick,
    /// Give the entry the file page shows a new name.
    RenameEntry,
    /// Open the file the file page shows to edit it.
    EditEntry,
    /// Install the file the file page shows.
    InstallEntry,
    /// Write the file the editor page shows as it now stands.
    SaveText,
    /// Open the page of the permissions of the entry the file page shows.
    OpenPermissions,
    /// Turn this permission bit of the entry the file page shows on or off.
    ModeBit(u32),
    /// Ask to delete the entry the file page shows, or the backup the
    /// restore page shows; a swipe then confirms.
    AskDelete,
    /// How to go on when a name is taken where an entry goes.
    Resolve(ClashChoice),
    /// Stop giving the data partition's files their labels.
    StopRelabel,
    /// Open the page of the filesystem page's partition at this place.
    OpenFilesystem(usize),
    /// Choose what to do to the partition that page shows.
    ChooseFs(FsChoice),
    /// Stop copying, moving or deleting.
    StopJob,
    /// Join the network at this place in the list of those in range, or
    /// open the page of the one joined.
    JoinNetwork(usize),
    /// Open the details of the network at this place in the list of those
    /// in range.
    NetworkDetails(usize),
    /// The network chosen: its password again once refused, its details
    /// otherwise.
    OpenJoined,
    /// Leave the network joined, and forget it.
    LeaveNetwork,
    /// Look for the networks in range again.
    Rescan,
    /// Open the page that adds a network by hand.
    AddNetwork,
    /// Type into this field of the page that adds a network.
    TypeInto(Field),
    /// Open the protections a network added by hand can have.
    ChooseSecurity,
    /// Take this protection, and close the list.
    PickSecurity(Security),
    /// Close the list open over the page, choosing nothing.
    CloseChoices,
    /// Ask the update server for the newest system.
    CheckUpdate,
    /// Download the newest system's full package, or carry on downloading
    /// it.
    Download,
    StopDownload,
    /// Install the full package downloaded.
    InstallDownloaded,
    /// Ask GitHub for Skyward's newest release.
    CheckSkyward,
    /// Download the newest release's build for the phone and write it into
    /// the recovery partitions.
    UpdateSkyward,
    /// Stop downloading the build.
    StopSkywardUpdate,
    /// A control whose work is not wired up yet; touching it says so.
    Unavailable(&'static str),
    /// A control drawn where it belongs but with nothing behind it yet: the
    /// settings gear on the pages whose settings page is not built. It takes
    /// the press — the chip darkens under the finger — and lets it go
    /// without a word, where [`Action::Unavailable`] would say one.
    Idle,
}

/// One finger, in screen pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Touch {
    Down(f32, f32),
    Move(f32, f32),
    Up,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Power,
    VolumeUp,
    VolumeDown,
    Back,
}

/// Where a screenshot went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shot {
    /// Into the gallery's screenshots, in the owner's shared storage.
    Album,
    /// Into the recovery's own memory, to go into the gallery's once the
    /// owner's storage opens.
    Held,
}

/// The exit code of a system that stopped because an app on its desktop is
/// not built yet.
///
/// Chosen rather than found: the codes around it are a process's — 101 for a
/// panic, 128 plus a signal for one that was killed, 139 for a bad pointer —
/// and none of them means "this was asked for and is not there yet". One code
/// for all such apps: which app it was is said in words, and a number cannot
/// carry a name.
pub const NOT_READY: i32 = 67;

/// Work the runtime carries out on the UI's behalf.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Reboot(Target),
    /// Stop the system where it stands, with this exit code, because
    /// something was asked of it that it cannot do.
    ///
    /// Nothing is drawn and nothing is put right here: the process ends, and
    /// the panel that watched it says what happened. `reason` is the word for
    /// the app, where there is one, which travels past the exit code in a
    /// note rather than in it — a code cannot carry a name.
    Stop { code: i32, reason: Option<&'static str> },
    /// A short tap of the vibration motor.
    Haptic,
    /// The timer's notice: start the long vibration while the notice stands,
    /// and stop it when the notice is put away.
    TimerAlarm { on: bool },
    /// The motor's two quick ticks as the power key turns the screen off,
    /// as strong as the phone's own system plays them whatever the
    /// vibration settings say.
    Ticks,
    /// Read the wallpaper named `key` from the ramdisk and draw the desk
    /// with it, or the lock when `desk` is false. The picture is the
    /// runtime's to decode, so the pages only carry the name.
    Wallpaper { desk: bool, key: String },
    /// Save what the panel shows as a picture: into the gallery's
    /// screenshots in the owner's shared storage where `storage` says it is
    /// open, and otherwise held until it is, telling how it went with
    /// [`App::screenshot_saved`].
    Screenshot { storage: bool },
    /// Offer internal storage to a computer over USB, or stop.
    FileTransfer(bool),
    /// Start receiving a package over USB.
    Sideload,
    /// Start installing the file at this path from the top of a place,
    /// telling how it goes as [`Effect::Sideload`] does.
    InstallFile(Place, String),
    /// Start installing the file at this full path, opened in the file
    /// manager, telling how it goes as [`Effect::Sideload`] does.
    InstallPath(String),
    /// Write the image the installer holds to `partition`, which lies in
    /// `place`, in the slots `pick` means there.
    Flash { partition: String, place: FlashPlace, pick: Pick },
    /// Stop waiting for the computer, for where an image goes, or for an
    /// update to be agreed to.
    Cancel,
    /// Read the slot's partitions for the mount page, which has just opened.
    ReadMounts,
    /// Read what the boot control holds of the slots, telling it with
    /// [`App::set_boot_slots`].
    ReadSlots,
    /// Have the bootloader start this slot next, telling how it went with
    /// [`App::slot_switched`].
    SetSlot(Slot),
    /// Read the systems for the systems page, telling them with
    /// [`App::set_systems`].
    ReadSystems,
    /// Write this system's boot set into the boot partitions and restart
    /// into it, telling of a switch that did not go with
    /// [`App::systems_work_failed`].
    SwitchSystem(Who),
    /// Take the other system away, telling how it went with
    /// [`App::set_systems`] or [`App::systems_work_failed`].
    RemoveSystem,
    /// Install the system the installer holds with a root of `size` bytes.
    InstallSystem { size: u64 },
    /// Mount the slot's partition with this base name, such as `system`.
    Mount(String),
    /// Unmount the slot's partition with this base name.
    Unmount(String),
    /// List the folder at this path for the file manager.
    ReadDir(String),
    /// List the folder at this path from the top of a place, to choose a
    /// file to install in, telling what it holds with [`App::set_place`].
    ReadPlace(Place, String),
    /// Open the data partition as far as it opens with nothing a user
    /// knows, telling what waits with [`App::data_waiting`] or why it did
    /// not open with [`App::data_failed`].
    OpenData,
    /// Open the own files of `user` with what they entered at their screen
    /// lock, telling what still waits with [`App::data_waiting`] or why it
    /// did not open with [`App::unlock_failed`].
    Unlock { user: u32, code: String },
    /// Hand the USB port back from fastboot to adb, bring up what the
    /// recovery runs from the slot's partitions, then show its pages with
    /// [`App::enter_recovery`].
    EnterRecovery,
    /// Erase all user data, as a factory reset does, telling how it goes
    /// with [`App::erase_at`] and [`App::erase_ended`].
    FactoryReset,
    /// Erase what the wipe page has chosen, [`Wipe::All`] as a factory
    /// reset does, telling how it goes with [`App::erase_at`],
    /// [`App::erase_progress`] and [`App::erase_ended`].
    Wipe(Vec<Wipe>),
    /// Install the update held for the user to agree to.
    Install,
    /// Forget what the system asked of the recovery, so that the next start
    /// goes where its reboot reason says.
    Dismiss,
    /// Start a shell for the terminal page in this folder, on a screen of
    /// the size [`App::term_size`] gives, telling of it with
    /// [`App::term_started`].
    StartShell(String),
    /// Have the terminal page's shell go to this folder if it waits for a
    /// command; one running a command is left to it, and [`App::report`]
    /// says so.
    ShellTo(String),
    /// Send these bytes to the shell, as typed.
    TermInput(Vec<u8>),
    /// Copy or move an entry into a folder; with no choice made yet, a name
    /// taken there is told with [`App::file_clash`] instead of going on.
    Paste { op: FileOp, path: String, into: String, clash: Option<ClashChoice> },
    /// Give an entry a new name; how it went comes back with
    /// [`App::renamed`].
    Rename { path: String, name: String },
    /// Give an entry these permission bits; how it went comes back with
    /// [`App::mode_set`].
    SetMode { path: String, mode: u32 },
    /// Read the file at this path to edit it, as much as
    /// [`editor::MAX_BYTES`] and one byte more, telling what it holds with
    /// [`App::text_opened`].
    OpenText(String),
    /// Write `text` as the contents of the file at `path`, which keeps what
    /// it has beside them; how it went comes back with
    /// [`App::text_saved`].
    SaveText { path: String, text: String },
    /// Delete an entry, folders whole.
    Delete(String),
    /// Stop the work on files under way.
    StopJob,
    /// Give the data partition's files the SELinux labels the installed
    /// system's rules give them, telling how it goes with
    /// [`App::relabel_started`], [`App::relabel_progress`] and
    /// [`App::relabel_ended`].
    Relabel,
    /// Stop giving the data partition's files their labels.
    StopRelabel,
    /// Read the partitions for the filesystem page, which has just opened,
    /// telling them with [`App::set_filesystems`].
    ReadFilesystems,
    /// Do `action` to the filesystem page's partition by its name, telling
    /// how it goes with [`App::part_work_started`], [`App::part_listed`],
    /// [`App::part_at`], [`App::part_ended`] and [`App::part_work_ended`];
    /// making the data partition afresh as another filesystem is a factory
    /// reset, which tells as erasing tells.
    Filesystem { name: String, action: FsAction },
    /// Save the logs chosen, the recovery's own and the kernel's, into a
    /// folder of their own in internal storage or the temporary folder,
    /// saying where with [`App::report`].
    ExportLogs { recovery: bool, kernel: bool, storage: bool },
    /// Back up the parts ticked, by their place in [`BACKUP_PARTS`], into
    /// a folder of their own in `place`, telling how it goes with
    /// [`App::backup_started`], [`App::backup_plan`], [`App::backup_at`]
    /// and [`App::backup_ended`].
    Backup { parts: [bool; 5], place: Place, compress: bool, verify: bool },
    /// Restore the parts ticked of the backup in the folder `name` of
    /// `place`, telling how it goes as a backup does.
    Restore { place: Place, name: String, parts: [bool; 5] },
    /// Look for backups in the places that can be read, telling what is
    /// found with [`App::set_backups`].
    FindBackups { storage: bool, usb: bool },
    /// Delete the backup in the folder `name` of `place`, telling how it
    /// went with [`App::backup_deleted`].
    DeleteBackup { place: Place, name: String },
    /// Stop the backup under way, or the check before a restore.
    StopBackup,
    /// Read where the update stands, telling it with [`App::set_update`].
    ReadUpdate,
    /// Turn the WLAN on or off; how it stands comes back with
    /// [`App::set_wlan`].
    Wlan(bool),
    ScanWlan,
    JoinWlan { ssid: Vec<u8>, name: String, security: Security, password: Secret },
    LeaveWlan,
    /// Ask the update server for the newest system, telling what it
    /// answered with [`App::update_checked`].
    CheckUpdate,
    /// Download `release`'s full package into [`DOWNLOADS`], telling how it
    /// goes with [`App::download_progress`] and [`App::download_ended`].
    Download(Release),
    StopDownload,
    /// Ask GitHub for Skyward's newest release, telling what it answered
    /// with [`App::skyward_checked`].
    CheckSkyward,
    /// Download `build` and write it into the recovery partitions, telling
    /// how it goes with [`App::skyward_progress`] and
    /// [`App::skyward_ended`].
    UpdateSkyward(Build),
    StopSkywardUpdate,
    /// Merge the update that waits in the slot the phone runs from,
    /// telling how it goes with [`App::set_update`].
    Merge,
}

enum Drag {
    Idle,
    /// Finger down and not yet past the touch slop. `edge` is set when the
    /// press began at a screen edge (true for the left one).
    /// `at` is when it came down: held long enough over an entry of the file
    /// manager, it opens what can be done with that entry.
    Press { x: f32, y: f32, action: Option<Action>, edge: Option<bool>, at: u64 },
    /// A press held long enough to have done its work; nothing it does
    /// counts until it lifts.
    Held,
    Scroll { last: f32 },
    /// Swiping in from a screen edge, the finger now `x` across; the cue
    /// keeps to height `cy`, set by where the swipe began. Past the
    /// threshold, lifting goes back.
    Edge { start: f32, from_left: bool, armed: bool, x: f32, cy: f32 },
    /// Pulling up from the foot of the screen, from where the finger came
    /// down at `from`: the page up off the screen, or the tasks up.
    Foot { from: f32 },
    /// Dragging the desktop's icons sideways, from where the finger came
    /// down at `start`. Letting go past a quarter of the screen turns it to
    /// the page beside the one it was on; short of that, it goes back.
    Page { start: f32 },
    /// Faces of a tabbed page, drawn whole and moved with the finger, from
    /// the x where it first went over the slop. The faces follow it until it
    /// lifts, then spring to the one it settled on.
    Face { from: f32 },
    /// A finger pulling the timer's ruler. `last_x` is where it was on the
    /// last move; `direction` remembers the way the scale last travelled so
    /// letting go can settle between two minutes on that side.
    Ruler { last_x: f32, direction: i32 },
    Slider(Slider, Rect),
    /// A finger dragging one of the volume panel's tracks: the card stands
    /// still while the track moves under it, so the track's own place is
    /// kept rather than looked up again each move.
    Panel(Slider, Rect),
    Confirm { which: Confirm, track: Rect, grab: f32 },
    /// Scrolling a terminal's screen back through what the shell wrote, or
    /// the editor's text, `carry` being the part of a row not yet scrolled.
    History { last: f32, carry: f32, pitch: f32 },
    Pattern,
    /// A finger that landed just after the panel lit; nothing it does
    /// counts until it lifts.
    Waking,
    /// A finger on the lock, pulling it up from where it came down at
    /// `from`: past the mark it goes away, short of it, back down. How quick
    /// it was going when it lifted is read from the same tracker the foot of
    /// the screen reads a pull with, so a flick means the same thing in
    /// either place.
    Lock { from: f32 },
    /// A finger on the status bar, or on the panel it has out, pulling it
    /// down or back up from where it came down. `last` is where it was on
    /// the move before, so a pull on a list already out scrolls the list
    /// rather than moving the panel; `bar` says the finger came down on the
    /// status bar itself, and so works the panel rather than the list
    /// wherever it goes — pulled out from there, pushed back from there.
    Shade { last: f32, bar: bool },
}

/// Where the keys are on a page.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Focus {
    /// Nowhere yet: the first control in view lights once the keys work
    /// the page.
    Unset,
    /// On no control, none showing where the keys scrolled the page.
    Off,
    /// On this control, as it was when the page was last drawn.
    On(Spot),
}

/// A button held down, and what holding it does.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Press {
    /// A volume key, moving the light down the page or up it, and again at
    /// `next`.
    Volume { down: bool, next: u64 },
    /// The power key on the control lit, from `at`, which it works as it
    /// comes up; held long enough over an entry of the file manager, it
    /// opens what can be done with the entry, as a long press does.
    Power { spot: Spot, at: u64 },
    /// The power key on a swipe-to-confirm control, whose knob slides along
    /// its track at `track` from `at` while the key is held; let go once
    /// it has slid far enough, the control confirms.
    Slide { which: Confirm, track: Rect, at: u64 },
    /// The power key held with nothing lit: let go before it has been held
    /// long enough, the panel goes off as a press of the key has always
    /// turned it off; held longer, the power menu comes up.
    PowerIdle { at: u64 },
    /// The power key that lit a dark panel, `at`: the panel stays lit
    /// whenever it is let go, and held long enough the power menu comes up
    /// over the lock, as a long press does with the panel already on.
    PowerWake { at: u64 },
    /// Done with: nothing more comes of it until it is let go.
    Spent,
}

const TOAST_MS: u64 = 2000;
/// The volume panel stays up this long after the last volume key.
const VOLUME_BAR_MS: u64 = 1500;
/// The volume's span, in steps: the panel's tracks and the volume keys both
/// count the same fifteen. Shared with the settings file so its saved levels
/// are checked against the same span the panel shows.
pub(crate) const VOLUME_STEPS: u8 = 15;
/// How long a text field's cursor shows and then hides, as the stock
/// fields blink it.
const BLINK_MS: u64 = 500;
/// The spring the home page's search bar moves to the search page's field
/// on, and back: critically damped, fitted to frames of the stock settings
/// on a Xiaomi 17 Ultra.
const MORPH_PERIOD: f32 = 0.292;
/// Along the move, from 0 at the home page to 1 at the search page: where
/// the large title has faded away, and the stretch over which the button
/// leaving the search comes in.
const MORPH_TITLE_GONE: f32 = 0.6;
const MORPH_CANCEL: (f32, f32) = (0.45, 0.95);
/// How blurred that button is as it starts to show, in dp.
const MORPH_CANCEL_BLUR_DP: f32 = 8.0;
/// When what the search page shows under its field comes in after the move
/// starts, and how long it takes, in ms.
const MORPH_BODY_MS: (u64, u64) = (380, 120);
/// A row a finger lands on starts darkening this long after, as the stock
/// settings' rows do; a finger moving on before then scrolls the page, and
/// the row never shows pressed.
const ROW_PRESS_DELAY_MS: u64 = 90;
/// The spring a row held down darkens on, and fades back on once let go:
/// critically damped, fitted to frames of the stock settings on a Xiaomi
/// 17 Ultra.
const ROW_PRESS_PERIOD: f32 = 0.18;
/// A row let go before it showed pressed shows pressed from the lift this
/// long, as Android shows a quick tap.
const QUICK_TAP_MS: u64 = 64;
/// How dark a desktop icon goes under a finger, at full: deeper than a
/// row's own darkening, which lies under words on a card, because an icon
/// is a lit tile with nothing over it and reads faint at the row's depth.
const ICON_PRESS_DIM: f32 = 0.22;
/// A page opening slides in on a critically damped spring of this period,
/// set off already moving this fast, in screen widths a second; a page
/// going back slides off on a spring of this period, from rest. Both fitted
/// to frames of the stock settings on a Xiaomi 17 Ultra.
const OPEN_PERIOD: f32 = 0.37;
const OPEN_SPEED: f32 = 2.94;
const BACK_PERIOD: f32 = 0.393;
/// A page growing out of a desktop icon, or shrinking back into it, moves
/// on a spring from rest: unlike a page sliding in it has no rush to catch
/// up with, so the size and the fade come up evenly together.
const ZOOM_PERIOD: f32 = 0.3;
/// While a page in front covers it wholly, the page behind stands this far
/// left, in screen widths, darkened this much.
const BEHIND_SHIFT: f32 = 0.25;
const BEHIND_DIM: f32 = 0.47;
/// The most lines of what an install script shows that its page keeps,
/// the latest; the log keeps them all.
const SCRIPT_LINES: usize = 500;
/// How long a finger rests on an entry before it counts as held, as the
/// system's long press does.
const LONG_PRESS_MS: u64 = 400;
/// A finger lifted slower than this, in dp per second, leaves the page
/// where it is.
const MIN_FLING_DP: f32 = 50.0;
/// The fastest a page starts gliding, in dp per second.
const MAX_FLING_DP: f32 = 8000.0;
/// A glide running into a page's end faster than this, in dp per second,
/// swings out on the slower spring.
const HARD_BOUNCE_DP: f32 = 1667.0;
/// Scroll, in dp, over which a large title fades out.
const TITLE_FADE_DP: f32 = 20.0;
/// Blur, in dp, of a title faded out completely.
const TITLE_BLUR_DP: f32 = 25.0 / 3.0;
/// A search bar folding away: its icon and words fade out over the first
/// this many dp of the fold, then the bar over as many more, gone however
/// faint once its foot reaches its top. A page let go with the bar folded
/// this share of the way or further settles folded, and unfolded short of
/// it, on a critically damped spring of this period set off from rest.
/// Fitted to frames of the stock settings on a Xiaomi 17 Ultra.
const FOLD_FADE_DP: f32 = 26.0;
const FOLD_SETTLE: f32 = 0.5;
const FOLD_PERIOD: f32 = 0.354;
/// How far, in dp, a swipe from an edge has to come in to go back.
const BACK_SWIPE_DP: f32 = 56.0;
/// The back cue sits this far, in dp, above where the finger came down,
/// and stays at that height however the finger then wanders.
const CUE_LIFT_DP: f32 = 9.0;
/// On the About page, where the wordmark's area ends, in dp from the top:
/// the version has faded out by the time it passes under the bar.
const ABOUT_LOGO_DP: f32 = 160.0;
/// Scroll, in dp, over which the wordmark then shrinks and fades.
const ABOUT_LOGO_FADE_DP: f32 = 40.0;
/// Scroll, in dp, over which the About page's field fades out; the field's
/// discs sit in the band this far below the bar.
const ABOUT_FIELD_DP: f32 = 389.0;
/// The field is worked out every this many pixels and filled in between.
const FIELD_CELL: usize = 24;
/// Before the panel goes off it stays dim this long, in milliseconds, or
/// a fifth of the screen-off time when that is shorter.
const DIM_MS: u64 = 7000;
/// The dim level is this much below the brightness set, no brighter than
/// the ceiling, and never under the slider's floor unless the brightness
/// set is.
const DIM_DROP: f32 = 0.04;
const DIM_CEILING: f32 = 0.05;
/// The lowest brightness the slider sets.
const MIN_BRIGHTNESS: f32 = 0.02;
/// How much of a slider's span a volume key moves it.
const SLIDER_STEP: f32 = 0.1;
/// Where both sliders start: one step of the keys from their left end.
pub const SLIDER_START: f32 = SLIDER_STEP;
/// While the backlight eases, it steps once a frame of a 120 Hz panel.
const BACKLIGHT_STEP_MS: u64 = 8;
/// Just after the panel lights, a touch or key only keeps it awake: the
/// second tap of a wake habit, or the hand reaching for a phone whose
/// screen an install just lit, should not act. Android's double-tap window.
const WAKE_GUARD_MS: u64 = 400;
/// A volume key held this long moves the light again, then once every
/// [`REPEAT_MS`], as the system repeats a key held.
const REPEAT_DELAY_MS: u64 = 400;
const REPEAT_MS: u64 = 50;
/// The power key and volume down pressed within this long of each other
/// take a screenshot, as the system's key combination does; the first of
/// the two waits this long, or until it is let go, before it does what it
/// does alone.
const CHORD_MS: u64 = 150;
/// How far along its track the knob of a swipe-to-confirm control has to
/// be let go to confirm.
const SLID: f32 = 0.9;
/// How long the power key held takes the knob the whole way along.
const SLIDE_MS: u64 = 700;
/// The response, in seconds, of the spring the page scrolls on to show
/// the control the keys moved to.
const SEEK_PERIOD: f32 = 0.35;
/// How strongly the light tints the control the keys are on, and the one
/// the power key is held down on.
const LIGHT_ALPHA: u8 = 36;
const LIGHT_HELD_ALPHA: u8 = 72;
/// How far past the screen, in screens, a long list keeps its rows drawn.
const NEAR_SCREENS: f32 = 0.5;

/// How the screen's content goes dark as the panel goes off, and comes up
/// as it lights, as the phone's own system moves it: to black in 365 ms
/// slowing as the cube of the time left, from black over 520 ms slowing as
/// its square.
const DARKEN: Ease = Ease::Out(365, 3);
const BRIGHTEN: Ease = Ease::Out(520, 2);
/// The charging screen comes over the page as the phone's own does, over
/// 420 ms slowing as the square of the time left, shows this long from
/// when the cable went in, and goes in 300 ms.
const CHARGE_SHOW: Ease = Ease::Out(420, 2);
const CHARGE_HIDE: Ease = Ease::Out(300, 2);
const CHARGE_MS: u64 = 4000;

/// How a part of the screen eases between shown and hidden.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Ease {
    /// A critically damped spring of this period, in seconds.
    Spring(f32),
    /// A straight run over this many milliseconds.
    Linear(u64),
    /// A run over this many milliseconds that slows to a stop, what is
    /// left of the way going as this power of the time left.
    Out(u64, i32),
}

impl Ease {
    fn progress(self, ms: u64) -> f32 {
        match self {
            Ease::Spring(period) => motion::settle(ms, period),
            Ease::Linear(span) => (ms as f32 / span as f32).min(1.0),
            Ease::Out(span, power) => 1.0 - (1.0 - (ms as f32 / span as f32).min(1.0)).powi(power),
        }
    }
}

/// The charging screen: when the cable went in, how far it shows over the
/// page, and whether the panel was dark then and goes dark again after.
struct Charging {
    since: u64,
    shown: Fade,
    dark_after: bool,
}

/// A part of the screen fading in or out: where it is headed, since when,
/// how far in it was then, and how it eases each way.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Fade {
    on: bool,
    since: u64,
    from: f32,
    show: Ease,
    hide: Ease,
}

impl Fade {
    fn hidden(show: Ease, hide: Ease) -> Fade {
        Fade { on: false, since: 0, from: 0.0, show, hide }
    }

    fn shown(show: Ease, hide: Ease) -> Fade {
        Fade { on: true, since: 0, from: 1.0, show, hide }
    }

    /// How far in, 0 to 1, at `now`.
    fn level(&self, now: u64) -> f32 {
        let (target, ease) = if self.on { (1.0, self.show) } else { (0.0, self.hide) };
        self.from + (target - self.from) * ease.progress(now.saturating_sub(self.since))
    }

    fn set(&mut self, on: bool, now: u64) {
        if on != self.on {
            self.from = self.level(now);
            self.since = now;
            self.on = on;
        }
    }

    /// Stand fully in or out at once.
    fn jump(&mut self, on: bool) {
        self.on = on;
        self.from = if on { 1.0 } else { 0.0 };
    }

    fn moving(&self, now: u64) -> bool {
        self.level(now) != if self.on { 1.0 } else { 0.0 }
    }
}

/// How the bar's title comes and goes: sharpening in on a spring and
/// blurring away on a quicker one, or on the About page, fading in a
/// straight line together with the bar.
fn title_fade(hero: bool) -> Fade {
    if hero {
        Fade::hidden(Ease::Linear(150), Ease::Linear(150))
    } else {
        Fade::hidden(Ease::Spring(0.3), Ease::Spring(0.15))
    }
}

fn bar_fade(hero: bool) -> Fade {
    if hero {
        Fade::hidden(Ease::Linear(150), Ease::Linear(150))
    } else {
        Fade::hidden(Ease::Linear(100), Ease::Linear(100))
    }
}

/// The panel at full brightness: it dims over half a second and comes
/// back quickly when touched.
fn full_brightness() -> Fade {
    Fade::shown(Ease::Linear(150), Ease::Linear(500))
}

/// The level the panel dims to shortly before it goes off, for the
/// brightness set.
fn dim_level(full: f32) -> f32 {
    (full - DIM_DROP).clamp(MIN_BRIGHTNESS, DIM_CEILING).min(full)
}

/// How long the panel stays dim before going off, for a screen-off time.
fn dim_span(after: u64) -> u64 {
    DIM_MS.min(after / 5)
}

/// Where a pinned strip shows on a page scrolled to `offset`: with the page,
/// or just under the bar once the page has carried it that far.
fn pinned_top(t: &Theme, p: &Pinned, offset: f32) -> f32 {
    (p.top - offset).max(widgets::bar_bottom(t))
}

/// Whether `r` reaches into the band of the screen from `top` to `bottom`.
fn within(r: Rect, (top, bottom): (f32, f32)) -> bool {
    r.y < bottom && r.y + r.h > top
}

/// Lay the sprite `blurs` keeps, moved by (`dx`, `dy`), in `color` at
/// `level` of its strength, blurred by `blur` pixels.
fn draw_faded(out: &mut Canvas, blurs: &mut Blurs, dx: f32, dy: f32, color: Rgba, level: f32, blur: f32) {
    if level > 0.0 {
        blurs.get(blur).draw(out, dx, dy, color, level);
    }
}

/// The home page's search bar moving up to become the search page's
/// field, or back down, as the stock settings move it: the home page's
/// content goes at once on the way up and comes along under the bar on
/// the way down; the large title fades and blurs as it goes up with the
/// bar; the button leaving the search fades and sharpens in; the keyboard
/// rises with the bar; and what the search page shows under its field
/// comes in last.
struct Morph {
    /// Up to the search page.
    into: bool,
    /// How far along it is, from 0 at the home page to 1 at the search
    /// page, as a spring in thousandths settling on the end it goes to.
    spring: Spring,
    start: u64,
    /// How far the home page was scrolled, where its bar stood.
    scroll: f32,
    /// The search page's keyboard, and where it rests.
    keyboard: Option<(Canvas, i32)>,
    /// What the search page shows between its field and the keyboard, and
    /// where.
    body: Option<(Canvas, i32)>,
    /// The home page's large title, and the button's disc and cross.
    title: Option<Blurs>,
    cancel: Option<(Blurs, Blurs)>,
}

impl Morph {
    /// How far along it is at `now`, and whether it has settled.
    fn at(&self, now: u64) -> (f32, bool) {
        let (x, done) = self.spring.at(now);
        (if self.into { 1.0 } else { 0.0 } + x / 1000.0, done)
    }
}

/// A control under a finger, darkening behind it a moment after the finger
/// lands: a row of the page behind its words and icon, or one of the
/// desktop's icons in the square it is drawn on. Where it lies on the
/// screen, when it starts darkening, and when it starts fading back.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Darken {
    rect: Rect,
    rise: u64,
    fall: Option<u64>,
}

impl Darken {
    /// How dark it is at `now`, from 0 to 1.
    fn level(&self, now: u64) -> f32 {
        if now < self.rise {
            return 0.0;
        }
        let up = |t: u64| motion::settle(t - self.rise, ROW_PRESS_PERIOD);
        match self.fall {
            Some(fall) if now >= fall => up(fall) * (1.0 - motion::settle(now - fall, ROW_PRESS_PERIOD)),
            _ => up(now),
        }
    }

    /// Whether it has faded back after being let go.
    fn gone(&self, now: u64) -> bool {
        self.fall.is_some_and(|fall| now >= fall && motion::settle(now - fall, ROW_PRESS_PERIOD) == 1.0)
    }

    /// Whether it is darkening or fading back at `now`.
    fn changing(&self, now: u64) -> bool {
        let dark = self.fall.is_none() && now >= self.rise && motion::settle(now - self.rise, ROW_PRESS_PERIOD) == 1.0;
        now >= self.rise && !dark && !self.gone(now)
    }

    /// The finger lifted at `now`: a row that had not shown pressed yet
    /// shows it a moment.
    fn lift(&mut self, now: u64) {
        if now < self.rise {
            self.rise = now;
            self.fall = Some(now + QUICK_TAP_MS);
        } else {
            self.fall = Some(now);
        }
    }
}

/// How a page showed on a frame: how far scrolled, and how far past an end
/// with that; how far its bar and the bar's title had come in; where the
/// keys' light was; whether the cursor showed; how dark the row a finger
/// held was; and the button a finger held.
#[derive(Clone, Copy, Debug, PartialEq)]
struct View {
    scroll: f32,
    offset: f32,
    bar: f32,
    title: f32,
    light: Option<(Group, Rect, f32, Rgba)>,
    caret: bool,
    row: f32,
    pressing: Option<Action>,
}

/// How long a clock face takes to give way to the other, and how far the
/// one coming in starts to the right of where it comes to rest. The pages
/// of a tab strip switch under a finger at once; these are two faces of one
/// page, and the swap reads as a change rather than an errand, so it is
/// short and it moves only a little.
const FACE_MS: u64 = 180;
const FACE_SLIDE_DP: f32 = 12.0;
/// How long a face strip takes to settle once the finger lifts.
const FACE_PERIOD: f32 = 0.28;
/// Past this share of a screen's width a pull commits to the face it pulled
/// in; short of it, the strip goes back where it came from.
const FACE_COMMIT: f32 = 0.32;
/// A flick faster than this many screen widths a second commits to the face
/// it was thrown towards, however far the finger travelled.
const FACE_FLING: f32 = 1.2;

/// A clock face giving way to the other: the page as the face left showed
/// it, and when the change was asked for. The face coming in is drawn over
/// this one, a little to the right of where it ends up, fading up as it
/// goes — the page is one page, and only what it says is changing.
struct FaceTurn {
    since: u64,
    left: Canvas,
}

/// A strip of faces under a finger, going where the finger goes. The face
/// left stands on the page beside the one it gives way to, and the two are
/// moved together by `shift` — the finger's own travel — until it lifts:
/// then a spring carries them to the face it settled on or back to the one
/// it came from. At either end the neighbour is empty, so pulling past the
/// last face shows the page's ground rather than a face that is not there.
struct FaceSwipe {
    /// Which face was showing when the finger came down, and which one it
    /// pulls in. The order is the one the words across the top name.
    from: usize,
    /// The one the finger pulls in, one past `from` in either direction, or
    /// `from` itself when there is none that way.
    to: usize,
    /// Which way the finger pulled: left for the face to the right of this
    /// one. Kept even at the ends, where there is no `to` to tell it by.
    onward: bool,
    /// How far the finger has dragged the strip, in pixels: negative when
    /// it pulled the next face in from the right. While the finger is down
    /// this is its travel; once it lifts, a spring carries it on.
    shift: f32,
    /// The spring settling the strip on letting go, and the shift at which
    /// it comes to rest: nought for the face it came from, a screen's width
    /// for the one it pulled in.
    spring: Option<Spring>,
    target: f32,
    /// The two faces have been drawn whole, once, where the fonts and device
    /// were at hand. Until the first frame of the drag they are empty.
    drawn: bool,
    /// The page as it showed under the finger, drawn whole once so the
    /// faces can be moved without drawing them again each frame.
    left: Canvas,
    /// The face pulled in, drawn whole once; empty at either end.
    right: Canvas,
}

impl FaceSwipe {
    /// Whether the strip is still being carried by a spring after the finger
    /// lifted, and so wants a frame at a time.
    fn moving(&self) -> bool {
        self.spring.is_some()
    }
}

/// A page sliding over another as it opens or goes back, as the stock
/// settings slide them. A page opened comes in from the right edge over
/// the page it opens from, which shifts a quarter of the way left and
/// darkens; going back, the page left slides off to the right over the
/// page gone back to, which comes back from the left and brightens. The
/// page in front has its left corners rounded as the screen's are.
/// Where a page stands while it moves over another, at one instant.
#[derive(Clone, Copy, Debug)]
enum Motion {
    /// Settled; nothing is over anything.
    None,
    /// Sliding across the screen or up it: the offset of the front page
    /// from its rest in pixels, how far the page behind is pushed, and how
    /// dark that is. Which axis is read from the turn itself.
    Move { at: i32, shift: i32, dim: f32 },
    /// Out of a desktop icon: the page's rect on screen and how much of it
    /// shows. Both grow together so the page never changes shape.
    Zoom { rect: Rect, alpha: f32 },
}

struct Turn {
    /// Going back: the page in front is the page left; else the page
    /// opened.
    back: bool,
    /// The move is up and down rather than across: a page opened from the
    /// desktop rises from the foot of the screen, and sinks back to it.
    rise: bool,
    /// A page coming out of a desktop icon, or going back into it, grows
    /// from this rect instead of sliding: kept while the move lasts so a
    /// page going back finds the same icon it came out of.
    icon: Option<Rect>,
    /// The page left, as it last showed.
    left: Canvas,
    /// How far from the left edge the page in front stands, in thousandths
    /// of the screen's width, as a spring settling where it goes; set off
    /// on the first frame of the slide.
    spring: Option<Spring>,
}

/// Whether a control is a row: as wide as the cards it lies on.
fn row_wide(t: &Theme, r: Rect) -> bool {
    let m = &t.m;
    (r.x - m.margin).abs() < 0.5 && (r.w - (m.w as f32 - m.margin * 2.0)).abs() < 0.5
}

/// The whole pixels of the page's `canvas` a held row's `rect` covers:
/// left, top, width and height.
fn row_part(r: Rect, canvas: &Canvas) -> (u32, u32, u32, u32) {
    let (x0, y0) = (r.x.floor().max(0.0) as u32, r.y.floor().max(0.0) as u32);
    let (x1, y1) = (((r.x + r.w).ceil() as u32).min(canvas.w), ((r.y + r.h).ceil() as u32).min(canvas.h));
    (x0, y0, x1 - x0, y1 - y0)
}

/// The keyboard at the foot of a typing page's `frame`, and where it rests.
fn keyboard_strip(t: &Theme, frame: &Frame) -> (Canvas, i32) {
    let top = (t.m.h as f32 - widgets::keyboard_height(t)).floor() as i32;
    let mut strip = Canvas::new(frame.canvas.w, (frame.canvas.h as i32 - top).max(0) as u32);
    strip.put_rows(&frame.canvas, top, frame.canvas.h as i32, 0);
    (strip, top)
}

pub struct App {
    pub state: State,
    /// The pages open, the one shown last: each with how far it is
    /// scrolled and where the keys are on it.
    stack: Vec<(Screen, f32, Focus)>,
    frame: Option<Frame>,
    /// The controls of the page drawn, in the order the keys go through
    /// them.
    spots: Option<Vec<Spot>>,
    /// The keys are working the pages: the control they are on is lit. A
    /// touch puts the light out, and a volume key brings it back.
    keys: bool,
    /// The button held down, and what holding it does.
    button: Option<(Key, Press)>,
    /// The first of the power key and volume down gone down, waiting for
    /// the other to take a screenshot, and when it went down.
    chord: Option<(Key, u64)>,
    /// The page scrolling of its own accord, to show the control the keys
    /// moved to or to fold its search bar whole or not at all: where it
    /// comes to rest, and the spring taking it there.
    seek: Option<(f32, Spring)>,
    /// A swipe-to-confirm control whose knob the power key moved, to draw
    /// again with the page.
    knob: Option<(Confirm, Rect)>,
    /// The slider lit takes the volume keys, until the power key lets it
    /// go.
    adjusting: bool,
    /// How tall the page was when last drawn: one being drawn again, as
    /// work under way tells how far it has come, scrolls as far meanwhile.
    height: f32,
    /// The rows of the page's long list drawn so far, by their place in it;
    /// those far off screen are let go, their canvases kept spare for the
    /// next rows to be drawn on.
    drawn: Vec<(usize, Canvas)>,
    spare: Vec<Canvas>,
    /// The current page's large and bar titles, for their words and the
    /// theme, with their blurred versions as they are needed. A page can
    /// change its title in place, as work on files does once it ends.
    titles: Option<((&'static str, bool), Blurs, Blurs)>,
    /// The icon and words of the current page's folding search bar, for
    /// its words and the theme.
    search_words: Option<((&'static str, bool), Canvas)>,
    /// The `<` chip for the theme (dark or not), and where it goes.
    chip: Option<(bool, Canvas, i32, i32)>,
    /// The chip behind the round button at the right end of the bar, as
    /// for `<`.
    button_chip: Option<(bool, Canvas, i32, i32)>,
    /// The folder the terminal page's shell last started in, where Enter
    /// starts it again once it has ended.
    shell_folder: String,
    /// When leaving the editor with its changes unsaved was last warned
    /// of: going back again while the warning shows leaves without them.
    unsaved_warned: Option<u64>,
    /// The About page field's dither, made for the screen once.
    dither: Option<glow::Dither>,
    toast: Option<(String, u64)>,
    /// The volume panel showing until this time, after a volume key moved
    /// it while the keys turn the volume.
    volume_bar: Option<u64>,
    drag: Drag,
    tracker: Tracker,
    /// The page gliding on after a flick.
    fling: Option<Fling>,
    /// A terminal's screen or the editor's text gliding on after a flick:
    /// the glide, a row's height, and how many rows it has scrolled so far.
    glide: Option<(Fling, f32, isize)>,
    /// How far a dragging finger has pulled the page past an end:
    /// negative past the top, positive past the bottom.
    pull: f32,
    /// The page springing back after being pulled or thrown past an end.
    bounce: Option<Spring>,
    /// The back cue shrinking away after a swipe that did not go back:
    /// its edge, where it sits, and its depth.
    cue: Option<(bool, f32, Spring)>,
    title: Fade,
    bar: Fade,
    /// A page was put up, or the file manager's folder changed: once it is
    /// rendered, the bar and its title stand as its scroll has them,
    /// without easing in or out.
    snap: bool,
    /// The About page's moving field, while that page shows.
    glow: Option<GlowClock>,
    /// The terminal page's shell, from when it is first opened; it goes on
    /// while other pages show.
    term: Option<Term>,
    display_on: bool,
    /// The time last told: the pages read the clock's counts by it, so a
    /// count that goes on while the page is elsewhere shows where it is.
    now: u64,
    /// When the screen was last touched or a key pressed; set on the first
    /// tick, so the time before the UI started does not count.
    active: Option<u64>,
    /// When the panel last lit from dark, for the brief guard that keeps a
    /// stray touch or key from acting on a page just put up.
    woke_at: Option<u64>,
    /// The panel at full brightness, or easing to the dim level it takes
    /// shortly before going off.
    bright: Fade,
    /// The screen's content against black: it goes dark as the panel goes
    /// off, under the backlight held then, and comes up as the panel lights.
    curtain: Fade,
    /// The power menu, over the page while the phone is being asked to
    /// restart or go off.
    power_menu: Option<power::PowerMenu>,
    /// The page the power menu came over, softened: its ground, kept for as
    /// long as the menu stands, so blurring the page once is enough.
    power_blur: Option<Canvas>,
    /// The phone being seen off from the power menu's confirmation: the page
    /// going black under the falling card, and where it is going.
    power_exit: Option<power::PowerExit>,
    /// The sheet over the page: one card at the foot, its ground only
    /// darkened, and the key that puts it away. What the timer ended with
    /// is written into its words, so no page holds a notice of its own.
    sheet: Option<dialog::Sheet>,
    /// The lock screen, over whatever the panel shows from the moment it
    /// goes dark: the page it came over kept soft behind it, and the swipe
    /// up that puts it away.
    lock: Option<lock::Lock>,
    /// The panel pulled down from the status bar: the control centre and the
    /// notices, standing over the page without taking it away. About only
    /// while a finger has it out or it is settling, so it costs nothing the
    /// rest of the time.
    shade: Option<shade::Shade>,
    /// A notice that has just come, standing over the page for a few
    /// seconds before it slips back into the panel's list.
    banner: Option<shade::Banner>,
    /// The notices the pages and the system have put, held for the panel.
    /// They belong to the panel, not to whoever put them: a page going away
    /// does not take them with it, and nothing runs out on its own.
    notifier: Notifier,
    /// Where the clock's two counts stand with the panel while its page is
    /// left behind: no notice put, one standing, or one a finger took away.
    clock_stopwatch: notify::Shown,
    clock_timer: notify::Shown,
    /// How far the battery has been told to be running down: none, or the
    /// step of `battery::BATTERY_LEVELS` it last said. It only climbs, so a
    /// battery sitting at a level is told once, and it climbs back to none
    /// only once the charger has taken it above `battery::RECOVERED`.
    battery_told: u8,
    /// What the battery has left to say and has not yet: the level it ran
    /// down to while the panel was busy — a card already up, the phone on
    /// the lock, work under way — and the card waits for the way to clear
    /// rather than covering what is on the panel.
    battery_pending: Option<u8>,
    /// The page the panel showed before it went dark, a pixel of every ten,
    /// kept sharp: the lock's ground when there is no wallpaper. None until
    /// a page has been shown.
    behind: Option<Canvas>,
    darkening: Option<f32>,
    /// The charging screen, from when a cable goes in until it has gone,
    /// with the level and what the charger does drawn for it, and the
    /// screen it is drawn on while it comes or goes over the page.
    charging: Option<Charging>,
    charge_art: Option<charge::Art>,
    charge_canvas: Option<Canvas>,
    /// While a text field's cursor is up: the page, how many edits had
    /// been typed, and when it came up whole on them. It shows for the
    /// first half of each blink from then on, so it is whole again at each
    /// edit and on each page it comes up on.
    blink: Option<(Screen, u64, u64)>,
    /// Edits typed into any text field so far.
    edits: u64,
    /// Whether the frame last composed showed the cursor.
    caret_shown: bool,
    /// The home page's search bar on its way to the search page's field,
    /// or back.
    morph: Option<Morph>,
    /// The row a finger holds down, or let go of while it fades back, and
    /// the part of the page it lies on as drawn and as drawn held down.
    row_press: Option<Darken>,
    row_art: Option<(Canvas, Canvas)>,
    /// The desktop's icon a finger holds down, or let go of while it fades
    /// back: the square it is drawn on, darkened where it stands.
    icon_press: Option<Darken>,
    /// How the page showed on the last frame, while it can be drawn so
    /// again: until it is rendered anew.
    shown: Option<View>,
    /// A page sliding in over the page left, or off over the page gone
    /// back to.
    turn: Option<Turn>,
    /// The rect of the desktop icon the page was opened from, kept so going
    /// back puts it away into the same icon. Only a page opened from the
    /// desktop uses this; the others just slide.
    desk_icon: Option<Rect>,
    /// A clock face giving way to the other, and the canvas the face coming
    /// in is drawn on while it does.
    face_turn: Option<FaceTurn>,
    face_page: Option<Canvas>,
    /// Faces drawn whole under a finger and moved with it, on the pages
    /// whose top words are tabs.
    face_swipe: Option<FaceSwipe>,
    /// The pill of the bar at the foot of a page, on its way from the face
    /// it was behind to the one just touched.
    foot_slide: Option<FootSlide>,
    /// A finger at the foot of the screen pulling, and what it pulls.
    foot: Option<Foot>,
    /// While a page slides: the page shown drawn on its own, and how it
    /// showed, to slide it again as it was while it shows the same.
    layer: Option<(View, Canvas)>,
    /// Bands of rows of the page drawn that show what has changed since,
    /// to draw again.
    stale: Vec<(u32, u32)>,
    /// How many columns from the left edge of the page's rows drawn are
    /// drawn; a page opened is drawn across as it slides in.
    cols: u32,
    /// The canvas the last page left was drawn on, to draw the next on.
    backdrop: Option<Canvas>,
    /// How round the screen's corners are, in pixels.
    corners: f32,
    dirty: bool,
    w: u32,
    h: u32,
    px_per_dp: f32,
    /// The picture the desktop is drawn on, already cropped and scaled to
    /// fill the screen; a desktop black where none was read.
    wallpaper: Option<Canvas>,
    /// The picture the lock screen stands over, its own rather than the
    /// desktop's; where none was read the lock falls back to the desktop's.
    lock_wallpaper: Option<Canvas>,
}

impl App {
    pub fn new(start: Screen, w: u32, h: u32, px_per_dp: f32) -> Self {
        App {
            state: State::default(),
            stack: vec![(start, 0.0, Focus::Unset)],
            frame: None,
            spots: None,
            // The touchscreen may run from the partitions fastboot leaves
            // out, so its page starts worked with the keys.
            keys: start == Screen::Fastboot,
            button: None,
            chord: None,
            power_menu: None,
            power_blur: None,
            power_exit: None,
            sheet: None,
            lock: None,
            shade: None,
            banner: None,
            notifier: Notifier::default(),
            clock_stopwatch: notify::Shown::No,
            clock_timer: notify::Shown::No,
            battery_told: 0,
            battery_pending: None,
            behind: None,
            seek: None,
            knob: None,
            adjusting: false,
            height: 0.0,
            drawn: Vec::new(),
            spare: Vec::new(),
            titles: None,
            search_words: None,
            chip: None,
            button_chip: None,
            shell_folder: "/".into(),
            unsaved_warned: None,
            dither: None,
            toast: None,
            volume_bar: None,
            drag: Drag::Idle,
            tracker: Tracker::default(),
            fling: None,
            glide: None,
            pull: 0.0,
            bounce: None,
            cue: None,
            title: title_fade(start == Screen::About),
            bar: bar_fade(start == Screen::About),
            snap: false,
            glow: None,
            term: None,
            display_on: true,
            now: 0,
            active: None,
            woke_at: None,
            bright: full_brightness(),
            curtain: Fade::shown(BRIGHTEN, DARKEN),
            darkening: None,
            charging: None,
            charge_art: None,
            charge_canvas: None,
            blink: None,
            edits: 0,
            caret_shown: false,
            morph: None,
            row_press: None,
            row_art: None,
            icon_press: None,
            shown: None,
            turn: None,
            desk_icon: None,
            face_turn: None,
            face_page: None,
            face_swipe: None,
            foot_slide: None,
            foot: None,
            layer: None,
            stale: Vec::new(),
            cols: 0,
            backdrop: None,
            corners: 0.0,
            dirty: true,
            w,
            h,
            px_per_dp,
            wallpaper: None,
            lock_wallpaper: None,
        }
    }

    /// Draw the desktop on `picture`, which is cropped and scaled to fill
    /// the screen: wider than the screen it loses its sides, taller its top
    /// and bottom, so it is never squeezed.
    pub fn set_wallpaper(&mut self, picture: Canvas) {
        self.wallpaper = Some(Canvas::covering(&picture, self.w, self.h));
    }

    /// The picture the desktop is drawn on, filling the screen.
    pub fn wallpaper(&self) -> Option<&Canvas> {
        self.wallpaper.as_ref()
    }

    /// Draw the lock on `picture`, cropped and scaled to fill the screen as
    /// the desktop's is: the lock stands over its own wallpaper rather than
    /// the desktop's.
    pub fn set_lock_wallpaper(&mut self, picture: Canvas) {
        self.lock_wallpaper = Some(Canvas::covering(&picture, self.w, self.h));
    }

    /// The picture the lock stands over, its own where one was read.
    pub fn lock_wallpaper(&self) -> Option<&Canvas> {
        self.lock_wallpaper.as_ref()
    }

    /// The screen's corners are rounded to `radius` pixels: so are the
    /// leading corners of a page sliding over another.
    pub fn set_screen_corners(&mut self, radius: f32) {
        self.corners = radius;
    }

    pub fn theme(&self) -> Theme {
        if self.state.dark {
            Theme::dark(self.w, self.h, self.px_per_dp)
        } else {
            Theme::light(self.w, self.h, self.px_per_dp)
        }
    }

    pub fn screen(&self) -> Screen {
        self.top().0
    }

    /// The time last told, as the pages read it: the clock's counts are
    /// worked out by it, so a page drawing on its own reads the same time
    /// as the live one.
    pub fn now(&self) -> u64 {
        self.now
    }

    fn top(&self) -> &(Screen, f32, Focus) {
        self.stack.last().expect("the page stack always holds a page")
    }

    fn top_mut(&mut self) -> &mut (Screen, f32, Focus) {
        self.stack.last_mut().expect("the page stack always holds a page")
    }

    /// Whether the panel needs a new frame since the last call.
    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    /// When time next has to pass through [`App::tick`]: at once while
    /// something on the lit screen moves on its own, a frame on while the
    /// backlight eases, else when a shown message should disappear or the
    /// panel should dim or go off.
    pub fn deadline(&self, now: u64) -> Option<u64> {
        let sliding = matches!(self.button, Some((_, Press::Slide { .. }))) && self.state.confirm < 1.0;
        let moving = self.display_on
            && (self.fling.is_some()
                || self.glide.is_some()
                || self.bounce.is_some()
                || self.seek.is_some()
                || sliding
                || self.cue.is_some()
                || self.title.moving(now)
                || self.bar.moving(now)
                || self.morph.is_some()
                || self.turn.is_some()
                || self.foot.as_ref().is_some_and(|f| f.spring.is_some())
                || self.row_press.is_some_and(|p| p.changing(now) || p.gone(now))
                || self.icon_press.is_some_and(|p| p.changing(now) || p.gone(now))
                || self.glowing());
        // The content going dark or coming up, and the charging screen's
        // tide, move a frame at a time while the panel is lit.
        let lit = self.panel(now).is_some();
        if moving || (lit && (self.curtain.moving(now) || self.charging.is_some())) {
            return Some(0);
        }
        // What is laid over the page, and a clock face turning to the
        // other, move by their own clocks too. Without this they were woken
        // once at the end of their run and drawn that once: the card went
        // from off the panel to where it was going between two frames, and
        // the phone held the panel's pace turning nothing out in between.
        if self.overlay_moving(now) {
            return Some(0);
        }
        let darkens = self.row_press.map(|p| p.rise).filter(|&rise| rise > now);
        let easing = self.bright.moving(now).then_some(now + BACKLIGHT_STEP_MS);
        let held = match self.drag {
            Drag::Press { action: Some(Action::OpenEntry(_)), at, .. } => Some(at + LONG_PRESS_MS),
            _ => None,
        };
        let repeat = match self.button {
            Some((_, Press::Volume { next, .. })) => Some(next),
            // A power key on the light wakes a file's entry with a long
            // press, and anywhere else it brings up the power menu: both
            // need the tick at the hold's end, so all of them wake here.
            Some((_, Press::Power { at, .. } | Press::PowerIdle { at } | Press::PowerWake { at })) => Some(at + LONG_PRESS_MS),
            _ => None,
        };
        let blink = self.blink_at(now).filter(|_| self.display_on).map(|(_, next)| next);
        let chord = self.chord.map(|(_, at)| at + CHORD_MS);
        // The clock's counts go on of their own accord, and the stopwatch
        // reads to the hundredth: the page is told to draw again a frame at
        // a time while the count on it runs, rather than waiting for the
        // fallback below, which is a whole second.
        // The ruler's way home from a reset asks for frames of its own: it
        // is not a count, so `running` does not see it.
        // The second hand's colour fade and its sweep home on a reset are
        // their own motion: after a pause or a reset the count stands still,
        // so `running` does not see them. Without them here the loop woke
        // only at the one-second fallback and the hand jumped to its end.
        // The timer's hand has its own sweep home for the same reason.
        let busy = self.state.clock.running()
            || self.state.clock.ruler_sweeping(now)
            || self.state.clock.hand.moving(now)
            || self.state.clock.timer_hand.moving(now);
        let ticking = (self.display_on && busy).then(|| now + clock::step(&self.state.clock, now));
        [self.toast.as_ref().map(|t| t.1), self.volume_bar, easing, self.idle_due(), held, repeat, blink, darkens, chord, ticking]
            .into_iter()
            .flatten()
            .min()
    }

    /// Whether what stands over the page is on the move: the power menu
    /// rising, a sheet rising or falling, the lock going away or back down
    /// as its spring carries it, the black-out a confirmed 关机 or 重新启动
    /// leaves, and a clock face turning to the other. None of these is a
    /// count that ticks by the clock or a scroll that follows a finger, so
    /// each has to ask for the frames it is made of — and the panel has to
    /// hear that it did, or the whole motion happens between two frames.
    fn overlay_moving(&self, now: u64) -> bool {
        self.sheet.as_ref().is_some_and(|s| s.moving(now))
            || self.power_menu.as_ref().is_some_and(|m| now <= m.since + dialog::RISE_MS)
            || self.power_exit.as_ref().is_some_and(|e| !e.fired)
            || self.lock.as_ref().is_some_and(|l| l.spring.is_some())
            || self.shade_moving(now)
            // A card standing over the page shows, holds and goes by its own
            // clock: it is drawn frame after frame until it has gone.
            || self.banner.is_some()
            || self.face_moving(now)
            || self.foot_moving(now)
    }

    /// Whether a clock face is giving way to the other. The frame it lands
    /// on counts: that is the one that puts the turn away.
    fn face_moving(&self, now: u64) -> bool {
        self.face_turn.as_ref().is_some_and(|t| now <= t.since + FACE_MS)
            || self.face_swipe.as_ref().is_some_and(|s| s.moving())
    }

    /// The tab faces a page holds, where the face shown stands among them,
    /// and how to show a given one. Every page whose top words are tabs
    /// answers these three, so a new one needs no more than this to take
    /// the same turn and the same swipe as the rest. `None` where a page
    /// has no tabs at all.
    fn face_slots(&self) -> Option<(usize, usize)> {
        match self.screen() {
            Screen::Clock => Some((if self.state.clock.face == ClockFace::Stopwatch { 0 } else { 1 }, 2)),
            Screen::Calculator => Some((if self.state.calc_face == CalcFace::Sum { 0 } else { 1 }, 2)),
            Screen::Music => Some((if self.state.music_face == MusicFace::List { 0 } else { 1 }, 2)),
            Screen::Tasks => Some((TasksFace::ALL.iter().position(|f| *f == self.state.tasks_face).unwrap_or(0), TasksFace::ALL.len())),
            Screen::Store => Some((StoreFace::ALL.iter().position(|f| *f == self.state.store_face).unwrap_or(0), StoreFace::ALL.len())),
            _ => None,
        }
    }

    /// Show the face at `at`, counting from the leftmost word across the
    /// top. The caller has already settled which face that is.
    fn show_face(&mut self, at: usize) {
        match self.screen() {
            Screen::Clock => self.state.clock.face = if at == 0 { ClockFace::Stopwatch } else { ClockFace::Timer },
            Screen::Calculator => self.state.calc_face = if at == 0 { CalcFace::Sum } else { CalcFace::Convert },
            Screen::Music => self.state.music_face = if at == 0 { MusicFace::List } else { MusicFace::Play },
            Screen::Tasks => self.state.tasks_face = TasksFace::ALL[at.min(TasksFace::ALL.len() - 1)],
            Screen::Store => self.state.store_face = StoreFace::ALL[at.min(StoreFace::ALL.len() - 1)],
            _ => {}
        }
        self.refresh();
    }

    /// Carry a face strip on after the finger lifted: the spring moves it
    /// towards the face it settled on, and once it lands the state shows
    /// that face and the strip is put away.
    fn settle_face_swipe(&mut self, now: u64) {
        let Some(swipe) = self.face_swipe.as_ref() else { return };
        let Some(spring) = swipe.spring else { return };
        let (off, settled) = spring.at(now);
        let (to, target) = (swipe.to, swipe.target);
        if settled {
            self.face_swipe = None;
            self.show_face(to);
            self.dirty = true;
        } else {
            if let Some(swipe) = self.face_swipe.as_mut() {
                swipe.shift = off + target;
            }
            self.dirty = true;
        }
    }

    /// The page as it stands, kept for the face coming in to be drawn over.
    /// Nothing is kept of a page that is not drawn whole on screen: the turn
    /// is a nicety, and the face changes either way.
    fn face_snapshot(&self, now: u64) -> Option<FaceTurn> {
        let fr = self.frame.as_ref()?;
        let (w, h) = (self.w, self.h);
        let y = self.top().1.max(0.0).round() as u32;
        let (held, cols) = (fr.canvas.held(), fr.canvas.cols());
        if cols.start > 0 || cols.end < w || held.start > y || held.end < y + h {
            return None;
        }
        Some(FaceTurn { since: now, left: fr.canvas.crop(0, y, w, h) })
    }

    /// Whether a text field's cursor shows at `now`, and when it next turns;
    /// `None` while no cursor is up.
    fn blink_at(&self, now: u64) -> Option<(bool, u64)> {
        let (_, _, since) = self.blink?;
        let n = now.saturating_sub(since) / BLINK_MS;
        Some((n.is_multiple_of(2), since + (n + 1) * BLINK_MS))
    }

    /// When sitting idle next changes the lit panel: when it dims, or once
    /// it is dimming, when it goes off.
    fn idle_due(&self) -> Option<u64> {
        let (active, after) = (self.active?, SCREEN_OFF[self.state.screen_off]?);
        if !self.display_on {
            return None;
        }
        Some(if self.bright.on { active + after - dim_span(after) } else { active + after })
    }

    /// Let time pass: the panel dims and goes off once left alone long
    /// enough, a gliding page moves on, a page past its end springs back,
    /// the bar and its title follow the scroll, and a message past its
    /// deadline goes away.
    pub fn tick(&mut self, now: u64) -> Vec<Effect> {
        let mut fx = Vec::new();
        self.now = now;
        // A sheet whose key was pressed leaves once it has fallen to the
        // foot, letting the page brighten behind it.
        self.settle_sheet(now);
        // The panel pulled down stays where it was let go, or goes back
        // whole and is let go itself.
        self.settle_shade(now);
        // A card that has stood its few seconds is let go; the notice it
        // spoke for is still in the list.
        if self.banner.as_ref().is_some_and(|b| b.gone(now)) {
            self.banner = None;
            self.dirty = true;
        }
        self.settle_face_swipe(now);
        // The card has fallen and the page under it is black: the phone is
        // asked to leave now, not the moment the key was pressed. The
        // black-out stands while it goes, so the screen never comes back.
        if let Some(exit) = self.power_exit.as_mut()
            && !exit.fired
            && now >= exit.since + power::BLACK_MS + power::BLACK_HOLD_MS
        {
            exit.fired = true;
            fx.push(Effect::Reboot(exit.target));
        }
        // The key waiting for its partner in a screenshot, left alone long
        // enough, does what it does alone.
        if self.chord.is_some_and(|(_, at)| now >= at + CHORD_MS) {
            fx.extend(self.let_chord_go());
        }
        // An entry of the file manager held long enough, under a finger or
        // the power key, opens what can be done with it, a folder as much as
        // a file, with a tap of the motor.
        if let Drag::Press { action: Some(Action::OpenEntry(i)), at, .. } = self.drag
            && now >= at + LONG_PRESS_MS
        {
            self.drag = Drag::Held;
            fx.extend(self.held_entry(i));
        }
        if let Some((key, Press::Power { spot: Spot { stop: Stop::Hit(Action::OpenEntry(i)), .. }, at })) = self.button
            && now >= at + LONG_PRESS_MS
        {
            self.button = Some((key, Press::Spent));
            fx.extend(self.held_entry(i));
        }
        // The power key held brings the power menu up, with nothing lit or
        // on a control — a short press still takes the control, but holding
        // it is always the menu. A file's entry above is the one exception:
        // its own long press opens what can be done with the entry.
        if let Some((key, Press::Power { at, .. } | Press::PowerIdle { at } | Press::PowerWake { at })) = self.button
            && now >= at + LONG_PRESS_MS
        {
            self.button = Some((key, Press::Spent));
            fx.extend(self.show_power_menu(now));
        }
        let active = *self.active.get_or_insert(now);
        if let (true, Some(after)) = (self.display_on, SCREEN_OFF[self.state.screen_off]) {
            let idle = now.saturating_sub(active);
            if idle >= after {
                self.sleep(now);
            } else if idle >= after - dim_span(after) {
                self.bright.set(false, now);
            }
        }
        // The charging screen goes once it has shown long enough: on a
        // panel lit for it, with the panel, else for the page.
        if self.display_on && self.charging.as_ref().is_some_and(|c| c.shown.on && now >= c.since + CHARGE_MS) {
            let c = self.charging.as_mut().expect("the charging screen shows");
            if c.dark_after {
                self.sleep(now);
            } else {
                c.shown.set(false, now);
            }
        }
        if self.charging.as_ref().is_some_and(|c| !c.shown.on && !c.shown.moving(now)) {
            self.end_charging();
        }
        // Once the content is dark, the panel is off, and what showed goes
        // with it.
        if !self.display_on && self.darkening.is_some() && self.curtain.level(now) <= 0.0 {
            self.darkening = None;
            self.end_charging();
        }
        if self.curtain.moving(now) || self.charging.is_some() {
            self.dirty = true;
        }
        // What stands over the page moves a frame at a time, and the panel
        // is only drawn again when it is told something changed: asking for
        // the frames is not enough, the drawing has to be asked for too.
        if self.overlay_moving(now) {
            self.dirty = true;
        }
        if self.toast.as_ref().is_some_and(|m| now >= m.1) {
            self.toast = None;
            self.dirty = true;
        }
        if self.volume_bar.is_some_and(|m| now >= m) {
            self.volume_bar = None;
            self.dirty = true;
        }
        if self.blink_at(now).is_some_and(|(on, _)| on != self.caret_shown) || self.morph.is_some() || self.turn.is_some() {
            self.dirty = true;
        }
        // A reset sweep ends on its own: once the hand lands, give its
        // angle back to the count and start fading it to charcoal.
        self.state.clock.settle_hand(now);
        self.state.clock.settle_timer_hand(now);
        self.state.clock.land_ruler(now);
        // The clock's counts go on of their own accord: while either goes,
        // the rows the reading is drawn in are drawn again each frame so the
        // reading follows. The hand's own sweep and fade need the same
        // frames, so the dial keeps up. A timer reaching its end stops there
        // with a tap of the motor.
        if self.state.clock.running() || self.state.clock.hand.moving(now) || self.state.clock.timer_hand.moving(now) {
            if self.display_on {
                match self.frame.as_ref().and_then(|fr| fr.live) {
                    // One band of rows, drawn again once however many ticks
                    // come before the next frame: the same rows queued twice
                    // would have them drawn twice over.
                    Some((first, end)) => {
                        if !self.stale.contains(&(first, end)) {
                            self.refresh_rows(first, end);
                        }
                    }
                    // A page that has not said which of its rows move: all
                    // of it is drawn again.
                    None => self.dirty = true,
                }
            }
            if self.state.clock.arrive(now) {
                let minutes = self.state.clock.span / 60_000;
                self.show_timer_done(minutes, now);
                self.refresh();
                fx.push(Effect::TimerAlarm { on: true });
            }
        }
        // A count that goes on with the clock's page left behind says so in
        // the panel until the page is back or the count stops. Read after
        // the counts have been advanced, so a timer that has just ended puts
        // its notice away in the same frame the notice of its end goes up.
        self.sync_clock_notices();
        // A battery that ran down while the panel had something else to say
        // says it as soon as the way is clear.
        self.say_battery(now);
        // A pull at the foot of the screen, let go, carries on to where it
        // leads or back where it came from.
        fx.extend(self.settle_foot(now));
        // The lock, let go, goes away or back down the same way.
        fx.extend(self.settle_lock(now));
        if let Some(p) = self.row_press {
            if p.gone(now) {
                self.row_press = None;
                self.row_art = None;
                self.dirty = true;
            } else if p.changing(now) {
                self.dirty = true;
            }
        }
        if let Some(fling) = self.fling {
            let (y, done) = fling.at(now);
            let next = y.clamp(0.0, self.max_scroll());
            let top = self.top_mut();
            if next != top.1 {
                top.1 = next;
                self.dirty = true;
            }
            if next != y {
                // The glide ran into an end: the speed it had left throws
                // the page past it, and it springs back.
                let speed = fling.velocity(now);
                let period = if speed.abs() <= self.theme().m.dp(HARD_BOUNCE_DP) { 0.4 } else { 0.55 };
                self.bounce = Some(Spring::new(now, 0.0, speed, period));
                self.fling = None;
            } else if done {
                self.fling = None;
                self.settle_fold(now);
            }
        }
        if let Some((glide, pitch, scrolled)) = self.glide {
            let (y, done) = glide.at(now);
            let rows = (y / pitch).trunc() as isize;
            if rows != scrolled {
                self.scroll_rows(rows - scrolled);
            }
            self.glide = if done { None } else { Some((glide, pitch, rows)) };
        }
        if let Some(b) = self.bounce {
            self.dirty = true;
            if b.at(now).1 {
                self.bounce = None;
            }
        }
        if let Some((target, spring)) = self.seek {
            let (off, done) = spring.at(now);
            let at = if done { target } else { (target + off).clamp(0.0, self.max_scroll()) };
            let top = self.top_mut();
            if top.1 != at {
                top.1 = at;
                self.dirty = true;
            }
            if done {
                self.seek = None;
            }
        }
        // The power key held on a swipe-to-confirm control slides its knob.
        if let Some((_, Press::Slide { which, track, at })) = self.button {
            let along = (now.saturating_sub(at) as f32 / SLIDE_MS as f32).min(1.0);
            if along != self.state.confirm {
                self.state.confirm = along;
                self.knob = Some((which, track));
                self.dirty = true;
            }
        }
        // A volume key held moves the light on and on, once the page it
        // last moved on has been drawn.
        if let Some((key, Press::Volume { down, next })) = self.button
            && now >= next
            && self.frame.is_some()
        {
            self.button = Some((key, Press::Volume { down, next: now + REPEAT_MS }));
            fx.extend(self.volume(down, now));
        }
        if let Some((_, _, s)) = self.cue {
            self.dirty = true;
            if s.at(now).1 {
                self.cue = None;
            }
        }
        if self.screen() == Screen::About {
            let look = self.theme().hero.glow;
            match &mut self.glow {
                Some(g) if std::ptr::eq(g.look(), look) => g.advance(now),
                _ => self.glow = Some(GlowClock::new(look, now)),
            }
            if self.glowing() {
                self.dirty = true;
            }
        }
        self.follow_scroll(now);
        fx
    }

    /// Whether the About page's field is on screen, moving.
    fn glowing(&self) -> bool {
        self.glow.is_some() && self.display_on && self.top().1 < self.theme().m.dp(ABOUT_FIELD_DP)
    }

    /// Whether the bar's title and the bar belong over the rendered page
    /// where it is scrolled: the title once the large one has faded, the bar
    /// once the content slides under it. On the About page both come once
    /// the version has faded out.
    fn bar_wanted(&self) -> Option<(bool, bool)> {
        let fr = self.frame.as_ref()?;
        let t = self.theme();
        let scroll = self.top().1;
        Some(if fr.hero.is_some() {
            let gone = scroll >= t.m.dp(ABOUT_LOGO_DP) - widgets::bar_bottom(&t);
            (gone, gone)
        } else {
            let under = (fr.content_top - widgets::bar_bottom(&t) - t.m.dp(20.0)).max(0.0);
            (fr.header && fr.title_lift(scroll) >= t.m.dp(TITLE_FADE_DP), scroll > under)
        })
    }

    /// Bring the bar and its title in or out as the page scrolls.
    fn follow_scroll(&mut self, now: u64) {
        let Some((title, bar)) = self.bar_wanted() else { return };
        self.title.set(title, now);
        self.bar.set(bar, now);
        if self.title.moving(now) || self.bar.moving(now) {
            self.dirty = true;
        }
    }

    /// How far past its end the page shows at `now`: negative past the
    /// top, positive past the bottom.
    fn overscroll(&self, now: u64) -> f32 {
        if self.pull != 0.0 {
            motion::rubber(self.pull, self.h as f32)
        } else {
            self.bounce.map_or(0.0, |b| b.at(now).0)
        }
    }

    /// Something laid over the page changed (the clock): compose again. The
    /// lock's picture keeps a clock of its own, so it is made again too.
    pub fn redraw(&mut self) {
        self.dirty = true;
        self.lock_sheet_gone();
    }

    /// The settings the pages show, as they are kept between starts.
    pub fn settings(&self) -> Settings {
        Settings::of(&self.state)
    }

    /// Take the settings `kept` from the starts before, and speak the
    /// language chosen then, or else the one the system `named`, or else
    /// the one the pages last spoke.
    pub fn take_settings(&mut self, kept: Option<&Settings>, named: Option<Lang>) {
        if let Some(kept) = kept {
            kept.apply(&mut self.state);
        }
        if !self.state.language_chosen
            && let Some(lang) = named.or(kept.and_then(Settings::spoken))
        {
            self.state.lang = lang;
        }
        self.refresh();
    }

    /// The page's readings changed (battery, storage): render it again.
    pub fn refresh(&mut self) {
        self.frame = None;
        self.stale.clear();
        self.spots = None;
        self.knob = None;
        self.row_art = None;
        self.shown = None;
        self.layer = None;
        self.spare.extend(self.drawn.drain(..).map(|(_, c)| c));
        self.dirty = true;
    }

    /// What the page shows changed only in rows `first..end`: draw those
    /// again, and the rest stays as drawn.
    fn refresh_rows(&mut self, first: u32, end: u32) {
        if self.frame.is_none() {
            return;
        }
        self.stale.push((first, end));
        self.spots = None;
        self.shown = None;
        self.layer = None;
        self.dirty = true;
    }
}
