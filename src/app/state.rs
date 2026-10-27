use crate::features::appearance::Setting as AppearanceOption;
use crate::features::apps::management::{self, Command as ManagementCommand, RemovalTarget, Route};
use crate::features::home::{GreetingMode, Setting as HomeOption, StartupFocus};
use crate::features::shortcuts::{self, Action as ShortcutAction, Button as ShortcutButton};
use crate::i18n::Language;
use crate::model::{
    App, BACKGROUNDS, CATEGORIES, INPUT_NAME_LIMIT, InputMode, Inventory, Preferences,
    clean_input_name,
};
use crate::text_rendering::TextRendering;
use crate::weather::{self, Unit};
use serde::Serialize;
use std::time::{Duration, Instant};

pub const WEATHER_SECTION: usize = 2;
pub const INPUTS_SECTION: usize = 3;
pub const SYSTEM_SECTION: usize = 4;
pub const SECTION_COUNT: usize = 5;
/// Automatic inventory retries before the error shows: with the 6.5 s Luna timeout about 50 s.
pub const INVENTORY_RETRIES: u32 = 5;
pub const INVENTORY_RETRY_DELAY: Duration = Duration::from_secs(2);
pub const HOME_STRIP_LEFT: f32 = 84.;
// Every density fits its full cards inside this strip; the next card peeks into
// the fade at the right, while a focused card stays fully visible.
pub const HOME_STRIP_RIGHT: f32 = 1748.;
#[cfg(test)]
pub const HOME_TILE_PITCH: f32 = 281.6;
pub const HOME_FADE_LEFT: f32 = 8.;
pub const HOME_FADE_RIGHT: f32 = 1910.;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum Page {
    Home,
    Apps,
    Settings,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Target {
    CatalogRefresh,
    CatalogFilter,
    CatalogPrevious,
    CatalogNext,
    CatalogUndo,
    CatalogRestoreAll,
    CatalogApp(String),
    CatalogBack,
    CatalogToggle,
    CatalogLgHome,
    CatalogRemove,
    ControlCenter,
    CloseControlCenter,
    Profile,
    ProfileName,
    ProfileClearName,
    ProfileAvatar,
    CloseProfile,
    Quick(crate::features::control_center::Action),
    SleepSlider,
    ClockAvatar,
    ClockSide,
    AvatarStyle,
    DisplayHD,
    DisplayUHD,
    DisplayKeep,
    RotationMode,
    RotationShuffle,
    RotationNext,
    RotationInclude(String),
    PhotoFolder,
    PhotoUrl,
    PhotoFit,
    PhotoImport,
    PhotoStock,
    PhotoClearSources,
    Photo(usize),
    ControlApply,
    ControlRestore,
    Nav(usize),
    TvSettings,
    Recent(String),
    RecentEmpty,
    RecentRetry,
    App(String),
    Input(String),
    Browse,
    Filter(usize),
    ClearFilter,
    Search,
    ClearSearch,
    Sidebar(usize),
    Setting(usize),
    SystemRefresh,
    SystemField(usize),
    PrivacyRefresh,
    PrivacyField(usize),
    GreetingName,
    GreetingNameClear,
    HomeOption(HomeOption),
    AppearanceOption(AppearanceOption),
    ShortcutsEnabled,
    ShortcutBinding(ShortcutButton),
    ShortcutsReset,
    StartupFocus(StartupFocus),
    AppNamesHome,
    AppNamesApps,
    ClockFormat,
    ClockTimeFormat,
    Background(usize),
    InputMode(String),
    InputRename,
    InputNameReset,
    InputXboxPreset,
    Choice(usize),
    CloseDialog,
    Retry,
    DismissNotice,
    Option(usize),
    RemovalCancel,
    RemovalConfirm,
    RemovalCheck,
    Key(usize),
    WeatherConfigure,
    WeatherToggle,
    WeatherLocation,
    WeatherTemperature,
    WeatherQuery,
    WeatherSearch,
    WeatherClear,
    WeatherResult(usize),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
    pub fn center(self) -> (f32, f32) {
        (self.x + self.w / 2., self.y + self.h / 2.)
    }
    pub fn inset(self, n: f32) -> Self {
        Self::new(self.x + n, self.y + n, self.w - 2. * n, self.h - 2. * n)
    }
}

#[derive(Clone, Debug)]
pub struct Control {
    pub target: Target,
    pub rect: Rect,
    pub scrollable: bool,
}

#[derive(Clone, Debug)]
pub enum Overlay {
    Profile,
    ControlCenter {
        restore: Target,
    },
    ControlConfirm {
        command: crate::features::controls::Command,
        restore: Target,
    },
    Options {
        id: String,
        restore: Target,
        removal: Option<RemovalTarget>,
    },
    RemovalConfirm {
        target: RemovalTarget,
        title: String,
        restore: Target,
    },
    RemovalStatus {
        restore: Target,
    },
    Recents {
        restore: Target,
    },
    Keyboard,
    SystemKeyboard,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum Subpage {
    AppManagement,
    ManagedApp(String),
    Display,
    Rotation,
    Wallpapers,
    Shortcuts,
    Appearance,
    Home,
    StartupFocus,
    Privacy,
    SystemInfo,
    AppNames,
    Greeting,
    Clock,
    ClockFormat,
    TimeFormat,
    Input(String),
    WeatherLocation,
    Temperature,
    Language,
    TextRendering,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum TextField {
    ProfileName,
    PhotoFolder,
    PhotoUrl,
    Apps,
    Location,
    Name,
    InputName,
}

#[derive(Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, PartialEq)]
pub enum Effect {
    AppCatalog {
        generation: u64,
    },
    Control {
        generation: u64,
        command: crate::features::controls::Command,
    },
    Save,
    Manage {
        generation: u64,
        command: ManagementCommand,
    },
    Privacy {
        generation: u64,
    },
    SystemInfo {
        generation: u64,
    },
    /// Read the app inventory again, keeping decoded icons of unchanged apps.
    Refresh,
    /// Refresh apps: read the inventory and every app icon again.
    Reload,
    Launch(String),
    Recent {
        generation: u64,
        command: crate::recents::Command,
    },
    Quit,
    Weather(weather::Command),
}

pub const KEYS: [&str; 40] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "Space",
    "Delete", "Clear", "Done",
];

pub struct State {
    pub sleep_draft: usize,
    pub cc_timer: bool,
    pub display_change: Option<bool>,
    pub display_keep: bool,
    pub display_trial: bool,
    pub display_report: String,
    pub screensaver: crate::features::screensaver::Session,
    pub rotation: crate::features::wallpapers::rotation::Session,
    pub wallpapers: crate::features::wallpapers::Session,
    pub photo_draft: String,
    pub controls: crate::features::controls::Controls,
    pub page: Page,
    pub focus: Target,
    pub inventory: Inventory,
    pub preferences: Preferences,
    pub query: String,
    pub filter: usize,
    pub section: usize,
    pub subpage: Option<Subpage>,
    pub scroll: f32,
    pub home_scroll: f32,
    pub overlay: Option<Overlay>,
    pub moving: Option<String>,
    pub loading: bool,
    /// Set by Refresh apps: the next inventory also reloads every app icon.
    pub reload_artwork: bool,
    pub launching: bool,
    pub notice: Option<(String, Instant)>,
    pub error: Option<String>,
    /// Inventory loads that failed in a row while no apps were shown yet.
    pub inventory_failures: u32,
    pub dirty: bool,
    pub pointer: bool,
    pub pressed: Option<Target>,
    pub previous_background: Option<(String, Instant)>,
    pub panel_started: Option<Instant>,
    pub weather: weather::Weather,
    pub keyboard_field: TextField,
    pub greeting_name_draft: String,
    pub greeting_from_home: bool,
    pub input_name: String,
    pub recents: crate::recents::Recents,
    pub system: crate::features::system::Information,
    pub privacy: crate::features::privacy::Privacy,
    pub management: management::Management,
    pub catalog: crate::features::apps::catalog::Catalog,
    pub inventory_generation: u64,
    pub removed_apps: std::collections::HashSet<String>,
    pub webos: bool,
    pub shortcut_session: shortcuts::Session,
}

impl State {
    pub fn new(preferences: Preferences) -> Self {
        Self {
            controls: crate::features::controls::Controls::default(),
            wallpapers: crate::features::wallpapers::Session::default(),
            photo_draft: String::new(),
            sleep_draft: 0,
            cc_timer: false,
            display_change: None,
            display_keep: false,
            display_trial: false,
            display_report: String::new(),
            screensaver: crate::features::screensaver::Session::default(),
            rotation: crate::features::wallpapers::rotation::Session::default(),
            page: Page::Home,
            focus: Target::Nav(0),
            inventory: Inventory::default(),
            preferences,
            query: String::new(),
            filter: 0,
            section: 0,
            subpage: None,
            scroll: 0.,
            home_scroll: 0.,
            overlay: None,
            moving: None,
            loading: false,
            reload_artwork: false,
            launching: false,
            notice: None,
            error: None,
            inventory_failures: 0,
            dirty: true,
            pointer: false,
            pressed: None,
            previous_background: None,
            panel_started: None,
            weather: weather::Weather::default(),
            keyboard_field: TextField::Apps,
            greeting_name_draft: String::new(),
            greeting_from_home: false,
            input_name: String::new(),
            recents: crate::recents::Recents::default(),
            system: crate::features::system::Information::default(),
            privacy: crate::features::privacy::Privacy::default(),
            management: management::Management::default(),
            catalog: crate::features::apps::catalog::Catalog::default(),
            inventory_generation: 0,
            removed_apps: std::collections::HashSet::new(),
            webos: false,
            shortcut_session: shortcuts::Session::default(),
        }
    }

    pub fn notify(&mut self, text: impl Into<String>) {
        self.notice = Some((text.into(), Instant::now()));
        self.dirty = true;
    }

    pub fn tick(&mut self) {
        let recents_open = self.recents_open();
        if recents_open || self.recents.visibility > 0. {
            self.dirty |= self.recents.animate(
                recents_open,
                self.preferences.reduced_motion,
                self.pointer,
                Instant::now(),
            );
        }
        if let Some(start) = self.panel_started {
            self.dirty = true;
            let duration = if self.overlay.is_some() { 120 } else { 150 };
            if self.preferences.reduced_motion || start.elapsed() >= Duration::from_millis(duration)
            {
                self.panel_started = None;
            }
        }
        if self
            .notice
            .as_ref()
            .is_some_and(|(_, start)| start.elapsed() < Duration::from_millis(120))
            && !self.preferences.reduced_motion
        {
            self.dirty = true;
        }
        if let Some((_, start)) = &self.previous_background {
            self.dirty = true;
            if self.preferences.reduced_motion || start.elapsed() >= Duration::from_millis(400) {
                self.previous_background = None;
            }
        }
        if self
            .notice
            .as_ref()
            .is_some_and(|(_, start)| start.elapsed() > Duration::from_secs(5))
        {
            self.notice = None;
            self.dirty = true;
        }
    }

    pub fn page_index(&self) -> usize {
        match self.page {
            Page::Home => 0,
            Page::Apps => 1,
            Page::Settings => 2,
        }
    }

    pub fn animating(&self) -> bool {
        ((self.recents_open() || self.recents.visibility > 0.)
            && (self.recents.visibility != if self.recents_open() { 1. } else { 0. }
                || self.recents.cards_animating(self.pointer)))
            || self.previous_background.is_some()
            || self.panel_started.is_some()
            || (!self.preferences.reduced_motion
                && self
                    .notice
                    .as_ref()
                    .is_some_and(|(_, start)| start.elapsed() < Duration::from_millis(120)))
    }

    pub fn open(&mut self, index: usize) {
        self.weather.cancel_search();
        self.page = match index {
            1 => Page::Apps,
            2 => Page::Settings,
            _ => Page::Home,
        };
        self.focus = Target::Nav(self.page_index());
        self.scroll = 0.;
        if self.page == Page::Home {
            self.scroll_favorites(0.);
        }
        self.query.clear();
        self.filter = 0;
        self.overlay = None;
        self.moving = None;
        self.subpage = None;
        self.panel_started = None;
        self.dirty = true;
    }

    pub fn apps(&self) -> Vec<&App> {
        let ids = self.preferences.favorites(&self.inventory.apps);
        if self.page == Page::Home {
            return ids
                .iter()
                .filter(|id| !self.preferences.hidden.contains(id))
                .filter_map(|id| self.inventory.apps.iter().find(|app| &app.id == id))
                .collect();
        }
        let query = self.query.to_lowercase();
        let mut apps: Vec<_> = self
            .inventory
            .apps
            .iter()
            .filter(|app| {
                self.preferences.hidden.contains(&app.id) == (self.filter == 5)
                    && (self.filter == 0
                        || self.filter == 5
                        || app.category == CATEGORIES[self.filter - 1])
                    && app.title.to_lowercase().contains(&query)
            })
            .collect();
        apps.sort_by(|a, b| {
            let rank = |app: &App| {
                ids.iter()
                    .position(|id| id == &app.id)
                    .unwrap_or(usize::MAX)
            };
            rank(a)
                .cmp(&rank(b))
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
        });
        apps
    }

    pub fn content_viewport(&self) -> Rect {
        let bottom = if self.page == Page::Home && self.moving.is_none() {
            1053.6
        } else {
            994.4
        };
        Rect::new(64., 108.8, 1792., bottom - 108.8)
    }

    pub fn home_names_visible(&self) -> bool {
        self.preferences.show_home_names && !self.dock_enabled()
    }

    /// Disabled rows remain focusable so the reason can be read, but cannot mutate.
    pub fn target_enabled(&self, target: &Target) -> bool {
        match target {
            Target::CatalogLgHome => false,
            Target::CatalogUndo => self.catalog.can_undo(),
            Target::CatalogRestoreAll => !self.preferences.hidden.is_empty(),
            Target::CatalogPrevious => self.catalog.page > 0,
            Target::CatalogNext => self.catalog.page + 1 < self.catalog_pages(),
            Target::CatalogToggle => {
                if let Some(Subpage::ManagedApp(id)) = &self.subpage {
                    self.preferences.hidden.contains(id) || self.catalog_can_hide(id)
                } else {
                    false
                }
            }
            Target::CatalogRemove => {
                (self.catalog_removal().is_some() && !self.management.active())
                    || self.catalog_removal_status()
            }
            Target::HomeOption(HomeOption::FavoritesDock) => {
                !self.preferences.two_row_home_favorites
            }
            Target::HomeOption(HomeOption::FavoriteNames) | Target::AppNamesHome => {
                !self.dock_enabled()
            }
            _ => true,
        }
    }

    fn toggle_home_rows(&mut self) {
        self.preferences.two_row_home_favorites = !self.preferences.two_row_home_favorites;
        if self.preferences.two_row_home_favorites {
            self.preferences.home.favorites_dock = false;
        }
        self.home_scroll = 0.;
        self.dirty = true;
    }

    pub fn dock_enabled(&self) -> bool {
        self.preferences.home.favorites_dock && !self.preferences.two_row_home_favorites
    }

    /// Shared dock/strip limits for drawing, remote reveal, hit testing and wheel scrolling.
    pub fn home_strip(&self) -> (f32, f32) {
        if !self.dock_enabled() {
            return (HOME_STRIP_LEFT, HOME_STRIP_RIGHT);
        }
        let cards = self.home_cards();
        let count = self.apps().len().clamp(1, cards.columns);
        let width = (count - 1) as f32 * cards.pitch + cards.width;
        let left = (1920. - width) / 2.;
        (left, left + width)
    }

    pub fn favorites_viewport(&self) -> Rect {
        let content = self.content_viewport();
        if self.dock_enabled() {
            // Culling is called per card: keep it independent of the app inventory.
            // Short docks cannot scroll, so their wider viewport has no extra hit targets.
            let cards = self.home_cards();
            let width = (cards.columns - 1) as f32 * cards.pitch + cards.width;
            return Rect::new(
                (1920. - width) / 2. - 24.,
                content.y,
                width + 48.,
                content.h,
            );
        }
        let left = if self.home_scroll > 0. {
            HOME_FADE_LEFT
        } else {
            0.
        };
        Rect::new(left, content.y, HOME_FADE_RIGHT - left, content.h)
    }

    pub fn hit_test(&self, controls: &[Control], x: f32, y: f32) -> Option<Target> {
        controls
            .iter()
            .rev()
            .find(|control| {
                let viewport =
                    if self.page == Page::Home && matches!(control.target, Target::App(_)) {
                        self.favorites_viewport()
                    } else {
                        self.content_viewport()
                    };
                control.rect.contains(x, y) && (!control.scrollable || viewport.contains(x, y))
            })
            .map(|control| control.target.clone())
    }

    /// Returns whether this wheel event scrolled favorites, so a pending card press is cancelled.
    pub fn wheel_scroll(
        &mut self,
        controls: &[Control],
        position: Option<(f32, f32)>,
        x: i32,
        y: i32,
    ) -> bool {
        let over_favorites = self.page == Page::Home
            && position.is_some_and(|(x, y)| {
                self.favorites_viewport().contains(x, y)
                    && controls.iter().any(|control| {
                        matches!(control.target, Target::App(_)) && control.rect.contains(x, y)
                    })
            });
        let favorites = self.page == Page::Home && (x != 0 || over_favorites);
        if favorites {
            self.scroll_favorites((if x != 0 { x } else { -y }) as f32 * self.home_cards().pitch);
        } else {
            let bottom = self.content_viewport().y + self.content_viewport().h - 12.;
            let max = controls
                .iter()
                .filter(|c| c.scrollable)
                .map(|c| c.rect.y + c.rect.h + self.scroll - bottom)
                .fold(0., f32::max);
            self.scroll = (self.scroll - y as f32 * 90.).clamp(0., max.max(0.));
        }
        self.pointer = true;
        self.dirty = true;
        favorites
    }

    pub fn scroll_favorites(&mut self, delta: f32) {
        let count = self
            .preferences
            .favorites(&self.inventory.apps)
            .iter()
            .filter(|id| {
                !self.preferences.hidden.contains(id)
                    && self.inventory.apps.iter().any(|app| &app.id == *id)
            })
            .count();
        let content = if count == 0 {
            0.
        } else {
            (self.home_layout(count).columns - 1) as f32 * self.home_cards().pitch
                + self.home_cards().width
        };
        let (left, right) = self.home_strip();
        let max = (content - (right - left)).max(0.);
        let next = (self.home_scroll + delta).clamp(0., max);
        if next != self.home_scroll {
            self.home_scroll = next;
            self.dirty = true;
        }
    }

    pub fn home_layout(&self, count: usize) -> crate::features::apps::home_layout::Layout {
        crate::features::apps::home_layout::Layout::new(
            count,
            self.preferences.two_row_home_favorites,
        )
    }

    pub fn home_cards(&self) -> crate::features::appearance::Cards {
        self.preferences.appearance.density.home()
    }

    pub fn apps_cards(&self) -> crate::features::appearance::Cards {
        self.preferences.appearance.density.apps()
    }

    pub fn open_control_center(&mut self) -> Vec<Effect> {
        if matches!(self.overlay, Some(Overlay::ControlCenter { .. })) {
            self.cc_timer = false;
            return self.back();
        }
        if self.overlay.is_some() || self.moving.is_some() || self.launching || self.display_trial {
            return vec![];
        }
        self.sleep_draft = self.controls.report.sleep_index.unwrap_or(0);
        self.cc_timer = false;
        self.overlay = Some(Overlay::ControlCenter {
            restore: self.focus.clone(),
        });
        self.focus = Target::Quick(crate::features::control_center::Action::Screensaver);
        self.pointer = false;
        self.pressed = None;
        self.dirty = true;
        if !self.preferences.reduced_motion {
            self.panel_started = Some(Instant::now());
        }
        self.control_request(crate::features::controls::Command::QuickStatus)
    }
    pub fn slider_values(&self, target: &Target) -> Option<(usize, Vec<String>)> {
        use crate::features::{
            appearance::{Blur, Density, Dimming},
            home::GreetingSize,
            screensaver::Timeout,
        };
        let language = self.preferences.language;
        let labels = |keys: &[&str]| {
            keys.iter()
                .map(|key| language.t(key).to_owned())
                .collect::<Vec<_>>()
        };
        Some(match target {
            Target::Setting(19) => {
                let value = self.preferences.screensaver.timeout;
                (
                    Timeout::ALL.iter().position(|v| *v == value).unwrap_or(0),
                    Timeout::ALL
                        .iter()
                        .map(|v| {
                            if *v == Timeout::Off {
                                language.t("stateOff").into()
                            } else {
                                format!("{} {}", v.minutes(), language.t("minutesShort"))
                            }
                        })
                        .collect(),
                )
            }
            Target::AppearanceOption(AppearanceOption::Density) => (
                match self.preferences.appearance.density {
                    Density::Compact => 0,
                    Density::Standard => 1,
                    Density::Comfortable => 2,
                },
                labels(&[
                    "appearanceCompact",
                    "appearanceStandard",
                    "appearanceComfortable",
                ]),
            ),
            Target::AppearanceOption(AppearanceOption::Dimming) => (
                match self.preferences.appearance.dimming {
                    Dimming::Off => 0,
                    Dimming::Low => 1,
                    Dimming::Medium => 2,
                    Dimming::High => 3,
                },
                vec![
                    language.t("stateOff").into(),
                    "20%".into(),
                    "40%".into(),
                    "60%".into(),
                ],
            ),
            Target::AppearanceOption(AppearanceOption::Blur) => (
                match self.preferences.appearance.blur {
                    Blur::Off => 0,
                    Blur::Soft => 1,
                    Blur::Strong => 2,
                },
                labels(&["stateOff", "appearanceBlurSoft", "appearanceBlurStrong"]),
            ),
            Target::HomeOption(HomeOption::GreetingSize) => (
                match self.preferences.home.greeting_size {
                    GreetingSize::Small => 0,
                    GreetingSize::Medium => 1,
                    GreetingSize::Large => 2,
                },
                labels(&["homeSmall", "homeMedium", "homeLarge"]),
            ),
            Target::SleepSlider => {
                if self.controls.report.sleep_index.is_none() {
                    (
                        0,
                        vec![
                            language
                                .t(if self.webos {
                                    "infoUnavailable"
                                } else {
                                    "infoNotApplicable"
                                })
                                .into(),
                        ],
                    )
                } else {
                    (
                        self.sleep_draft.min(5),
                        crate::features::control_center::SLEEP_MINUTES
                            .iter()
                            .map(|m| {
                                if *m == 0 {
                                    language.t("stateOff").into()
                                } else {
                                    format!("{m} {}", language.t("minutesShort"))
                                }
                            })
                            .collect(),
                    )
                }
            }
            _ => return None,
        })
    }
    pub fn set_slider(&mut self, target: &Target, index: usize) -> Vec<Effect> {
        use crate::features::{
            appearance::{Blur, Density, Dimming},
            home::GreetingSize,
            screensaver::Timeout,
        };
        if self.display_trial || !self.slider_allowed(target) {
            return vec![];
        }
        let Some((previous, values)) = self.slider_values(target) else {
            return vec![];
        };
        let index = index.min(values.len().saturating_sub(1));
        if index == previous {
            return vec![];
        }
        match target {
            Target::Setting(19) => self.preferences.screensaver.timeout = Timeout::ALL[index],
            Target::AppearanceOption(AppearanceOption::Density) => {
                self.preferences.appearance.density =
                    [Density::Compact, Density::Standard, Density::Comfortable][index];
                self.home_scroll = 0.;
            }
            Target::AppearanceOption(AppearanceOption::Dimming) => {
                self.preferences.appearance.dimming =
                    [Dimming::Off, Dimming::Low, Dimming::Medium, Dimming::High][index]
            }
            Target::AppearanceOption(AppearanceOption::Blur) => {
                self.preferences.appearance.blur = [Blur::Off, Blur::Soft, Blur::Strong][index]
            }
            Target::HomeOption(HomeOption::GreetingSize) => {
                self.preferences.home.greeting_size = [
                    GreetingSize::Small,
                    GreetingSize::Medium,
                    GreetingSize::Large,
                ][index]
            }
            Target::SleepSlider => {
                self.sleep_draft = index;
                self.dirty = true;
                return vec![];
            }
            _ => return vec![],
        }
        self.dirty = true;
        vec![Effect::Save]
    }
    pub fn slider_allowed(&self, target: &Target) -> bool {
        if matches!(self.overlay, Some(Overlay::ControlCenter { .. })) {
            return if self.cc_timer {
                *target == Target::SleepSlider
            } else {
                *target == Target::AppearanceOption(AppearanceOption::Dimming)
            };
        }
        if self.overlay.is_some() || self.page != Page::Settings {
            return false;
        }
        match target {
            Target::Setting(19) => self.subpage.is_none() && self.section == 0,
            Target::AppearanceOption(_) => self.subpage == Some(Subpage::Appearance),
            Target::HomeOption(HomeOption::GreetingSize) => self.subpage == Some(Subpage::Home),
            _ => false,
        }
    }
    pub fn slider_at(&mut self, target: &Target, rect: Rect, x: f32) -> Vec<Effect> {
        let Some((_, values)) = self.slider_values(target) else {
            return vec![];
        };
        let index = crate::features::sliders::index_at(
            x,
            rect.x + crate::features::sliders::INSET,
            rect.w - 2. * crate::features::sliders::INSET,
            values.len(),
        );
        self.set_slider(target, index)
    }
    pub fn quick_action(&mut self, action: crate::features::control_center::Action) -> Vec<Effect> {
        use crate::features::{control_center::Action, controls::Command};
        if !matches!(self.overlay, Some(Overlay::ControlCenter { .. })) {
            return vec![];
        }
        if action == Action::Sleep {
            self.cc_timer = !self.cc_timer;
            self.focus = if self.cc_timer {
                Target::SleepSlider
            } else {
                Target::Quick(Action::Sleep)
            };
            self.dirty = true;
            return vec![];
        }
        if action == Action::Standby
            && (!self.webos || !self.controls.report.power_active || self.controls.loading)
        {
            self.notify(self.preferences.language.t(if self.webos {
                "infoUnavailable"
            } else {
                "infoNotApplicable"
            }));
            return vec![];
        }
        if action == Action::Audio
            && (!self.webos || self.controls.report.muted.is_none() || self.controls.loading)
        {
            self.notify(self.preferences.language.t(if self.webos {
                "infoUnavailable"
            } else {
                "infoNotApplicable"
            }));
            return vec![];
        }
        let muted = self.controls.report.muted.unwrap_or(false);
        self.cc_timer = false;
        self.back();
        match action {
            Action::Standby => {
                self.confirm_control(Command::Standby);
                vec![]
            }
            Action::Audio => self.control_request(Command::AudioMute {
                expected: muted,
                value: !muted,
            }),
            Action::Network => {
                self.launching = true;
                vec![Effect::Launch("com.palm.app.settings".into())]
            }
            Action::Screensaver => {
                self.screensaver.start(Instant::now());
                self.dirty = true;
                vec![]
            }
            Action::Search => {
                self.open(1);
                self.open_keyboard(TextField::Apps);
                vec![]
            }
            Action::Inputs => {
                if let Some(input) = self
                    .inventory
                    .inputs
                    .iter()
                    .find(|input| self.home_input_visible(input))
                    .map(|i| i.id.clone())
                {
                    self.open(0);
                    self.focus = Target::Input(input);
                } else {
                    self.notify(self.preferences.language.t("shortcutNoInputs"));
                }
                vec![]
            }
            Action::Recents => self.open_recents(),
            Action::Settings => {
                self.open(2);
                self.section = 0;
                self.focus = Target::Sidebar(0);
                vec![]
            }
            Action::Sleep => vec![],
        }
    }
    pub fn rotation_pool(&self) -> Vec<String> {
        let mut pool = vec![];
        for id in &self.preferences.wallpapers.rotation.selected {
            if (BACKGROUNDS.iter().any(|(key, _)| key == id) || self.wallpapers.contains(id))
                && !pool.contains(id)
            {
                pool.push(id.clone());
            }
        }
        pool
    }
    pub fn rotate_background(&mut self) -> Vec<Effect> {
        let pool = self.rotation_pool();
        let current = self.current_background().to_owned();
        let Some(next) = self
            .rotation
            .choose(
                &pool,
                &current,
                self.preferences.wallpapers.rotation.shuffle,
            )
            .map(str::to_owned)
        else {
            return vec![];
        };
        self.previous_background =
            (!self.preferences.reduced_motion).then(|| (current, Instant::now()));
        if crate::features::wallpapers::valid_id(&next) {
            self.preferences.wallpapers.selected = Some(next);
        } else {
            self.preferences.wallpapers.selected = None;
            self.preferences.background = next;
        }
        self.rotation.reset();
        self.dirty = true;
        vec![Effect::Save]
    }
    pub fn rotation_controls(&self) -> Vec<Target> {
        let mut rows = vec![
            Target::RotationMode,
            Target::RotationShuffle,
            Target::RotationNext,
        ];
        rows.extend(
            BACKGROUNDS
                .iter()
                .map(|(id, _)| Target::RotationInclude((*id).into())),
        );
        rows.extend(
            self.wallpapers
                .photos
                .iter()
                .map(|photo| Target::RotationInclude(photo.id.clone())),
        );
        rows
    }
    pub fn current_background(&self) -> &str {
        self.preferences
            .wallpapers
            .selected
            .as_deref()
            .filter(|id| self.wallpapers.contains(id))
            .unwrap_or(&self.preferences.background)
    }
    pub fn photo_controls(&self) -> Vec<Target> {
        let mut rows = vec![
            Target::PhotoFolder,
            Target::PhotoUrl,
            Target::PhotoFit,
            Target::PhotoImport,
            Target::PhotoStock,
            Target::PhotoClearSources,
        ];
        rows.extend((0..self.wallpapers.photos.len()).map(Target::Photo));
        rows
    }

    pub fn confirm_control(&mut self, command: crate::features::controls::Command) {
        if self.controls.loading || !self.webos || self.overlay.is_some() {
            return;
        }
        self.overlay = Some(Overlay::ControlConfirm {
            command,
            restore: self.focus.clone(),
        });
        self.focus = Target::CloseDialog;
        self.dirty = true;
    }
    pub fn control_request(&mut self, command: crate::features::controls::Command) -> Vec<Effect> {
        if !self.webos {
            return vec![];
        }
        self.controls
            .request(command)
            .map(|(generation, command)| {
                vec![Effect::Control {
                    generation,
                    command,
                }]
            })
            .unwrap_or_default()
    }

    pub fn shortcut_down(
        &mut self,
        button: ShortcutButton,
        repeat: bool,
        active: bool,
    ) -> Option<Vec<Effect>> {
        let observed = self.shortcut_session.observed(button);
        let fresh = self.shortcut_session.down(button, repeat, active);
        self.dirty |= observed != self.shortcut_session.observed(button);
        if !fresh
            || !active
            || !self.preferences.shortcuts.enabled
            || self.overlay.is_some()
            || self.moving.is_some()
            || self.launching
            || self.display_trial
            || self.subpage == Some(Subpage::Shortcuts)
        {
            return None;
        }
        let action = self.preferences.shortcuts.get(button);
        if action == ShortcutAction::None {
            return None;
        }
        self.pointer = false;
        self.pressed = None;
        self.dirty = true;
        match action {
            ShortcutAction::Search => {
                self.open(1);
                self.open_keyboard(TextField::Apps);
            }
            ShortcutAction::Favorites => {
                self.home_scroll = 0.;
                self.open(0);
                self.focus = self
                    .apps()
                    .first()
                    .map_or(Target::Browse, |app| Target::App(app.id.clone()));
            }
            ShortcutAction::Inputs => {
                let input = self
                    .inventory
                    .inputs
                    .iter()
                    .find(|input| self.home_input_visible(input))
                    .map(|input| input.id.clone());
                if let Some(id) = input {
                    self.open(0);
                    self.focus = Target::Input(id);
                } else {
                    self.notify(self.preferences.language.t("shortcutNoInputs"));
                }
            }
            ShortcutAction::Apps => {
                self.open(1);
                self.focus = self.first_content();
            }
            ShortcutAction::Settings => {
                self.open(2);
                self.section = 0;
                self.focus = Target::Sidebar(0);
            }
            ShortcutAction::Recents => return Some(self.open_recents()),
            ShortcutAction::ControlCenter => return Some(self.open_control_center()),
            ShortcutAction::None => {}
        }
        Some(vec![])
    }

    pub fn shortcut_controls() -> Vec<Target> {
        let mut rows = vec![Target::ShortcutsEnabled];
        rows.extend(ShortcutButton::ALL.into_iter().map(Target::ShortcutBinding));
        rows.push(Target::ShortcutsReset);
        rows
    }

    pub fn greeting_text(&self) -> &str {
        if self.preferences.home.greeting_mode == GreetingMode::Static {
            &self.preferences.home.custom_greeting
        } else {
            &self.preferences.greeting_name
        }
    }

    pub fn home_input_visible(&self, input: &crate::model::Input) -> bool {
        self.preferences.home.show_inputs && self.preferences.input_visible(input)
    }

    pub fn startup_choices(&self) -> Vec<(StartupFocus, String)> {
        let mut choices = vec![(
            StartupFocus::Auto,
            self.preferences.language.t("homeAutomatic").into(),
        )];
        for id in self.preferences.favorites(&self.inventory.apps) {
            if !self.preferences.hidden.contains(&id)
                && !crate::scroll_preview::is_placeholder(&id)
                && let Some(app) = self.inventory.apps.iter().find(|app| app.id == id)
            {
                choices.push((StartupFocus::App(id), app.title.clone()));
            }
        }
        for input in &self.inventory.inputs {
            if self.home_input_visible(input) {
                choices.push((
                    StartupFocus::Input(input.id.clone()),
                    self.preferences.input_title(input).into(),
                ));
            }
        }
        choices
    }

    pub fn startup_focus(&self) -> Target {
        let desired = &self.preferences.home.startup_focus;
        if self
            .startup_choices()
            .iter()
            .any(|(choice, _)| choice == desired)
        {
            match desired {
                StartupFocus::App(id) => return Target::App(id.clone()),
                StartupFocus::Input(id) => return Target::Input(id.clone()),
                StartupFocus::Auto => {}
            }
        }
        self.first_content()
    }

    fn adjacent_home_input(&self, controls: &[Control], above: bool) -> Option<Target> {
        let current = controls
            .iter()
            .find(|control| control.target == self.focus)?;
        let (x, y) = current.rect.center();
        let edge = controls
            .iter()
            .filter(|control| matches!(control.target, Target::Input(_)))
            .filter(|control| (control.rect.center().1 < y) == above)
            .map(|control| control.rect.y)
            .reduce(|a, b| if above { a.max(b) } else { a.min(b) })?;
        controls
            .iter()
            .filter(|control| {
                matches!(control.target, Target::Input(_)) && (control.rect.y - edge).abs() < 1.
            })
            .min_by(|a, b| {
                (a.rect.center().0 - x)
                    .abs()
                    .total_cmp(&(b.rect.center().0 - x).abs())
            })
            .map(|control| control.target.clone())
    }

    pub fn set_inventory(&mut self, next: Inventory, warning: Option<String>) {
        self.removed_apps
            .retain(|id| !next.apps.iter().any(|app| &app.id == id));
        self.inventory.apps = next.apps;
        if let Some(warning) = warning {
            self.notify(warning);
        } else {
            self.inventory.inputs = next.inputs;
        }
        self.loading = false;
        self.error = None;
        self.inventory_failures = 0;
        self.dirty = true;
        self.reconcile_focus();
    }

    /// Called only after a valid complete inventory proved the selected app absent.
    pub fn complete_removal(&mut self, id: &str) -> Vec<Effect> {
        self.catalog_invalidate_removed(id);
        self.inventory.apps.retain(|app| app.id != id);
        if let Some(favorites) = &mut self.preferences.favorites {
            favorites.retain(|item| item != id);
        }
        self.preferences.hidden.retain(|item| item != id);
        self.recents.entries.retain(|entry| entry.app.id != id);
        self.recents
            .dismissals
            .retain(|entry| entry.entry.app.id != id);
        self.removed_apps.insert(id.into());
        // Invalidate any inventory request that started before verified removal.
        self.inventory_generation = self.inventory_generation.wrapping_add(1);
        self.loading = false;
        if self.filter == 5 && self.hidden_count() == 0 {
            self.filter = 0;
        }
        self.reconcile_focus();
        self.notify(self.preferences.language.t(if self.webos {
            "uninstallDone"
        } else {
            "uninstallPreviewDone"
        }));
        vec![Effect::Save, Effect::Refresh]
    }

    /// Retry startup inventory reads without substituting demo data on the TV.
    pub fn inventory_failed(&mut self, error: String) -> Option<Duration> {
        self.dirty = true;
        if self.inventory.apps.is_empty() && self.inventory_failures < INVENTORY_RETRIES {
            self.inventory_failures += 1;
            return Some(INVENTORY_RETRY_DELAY);
        }
        self.inventory_failures = 0;
        self.loading = false;
        self.error = Some(error);
        None
    }

    pub fn reconcile_focus(&mut self) {
        if matches!(
            self.focus,
            Target::CatalogBack
                | Target::CatalogToggle
                | Target::CatalogRemove
                | Target::CatalogLgHome
        ) && !matches!(self.subpage, Some(Subpage::ManagedApp(_)))
        {
            self.focus = Target::CatalogRefresh;
        }
        if let Some(Subpage::ManagedApp(id)) = &self.subpage
            && self.catalog_entry(id).is_none()
        {
            self.subpage = Some(Subpage::AppManagement);
            self.focus = Target::CatalogRefresh;
            self.scroll = 0.;
        }
        if let Target::CatalogApp(id) = &self.focus
            && !self
                .catalog_entries()
                .into_iter()
                .skip(self.catalog.page * crate::features::apps::catalog::PAGE_SIZE)
                .take(crate::features::apps::catalog::PAGE_SIZE)
                .any(|app| &app.id == id)
        {
            self.focus = Target::CatalogFilter;
            self.scroll = 0.;
        }
        if let Target::StartupFocus(choice) = &self.focus
            && !self
                .startup_choices()
                .iter()
                .any(|(item, _)| item == choice)
        {
            self.focus = Target::StartupFocus(StartupFocus::Auto);
        }
        if self.page == Page::Home {
            self.scroll_favorites(0.);
        }
        if let Target::App(id) = &self.focus
            && !self.apps().iter().any(|app| &app.id == id)
        {
            self.focus = self.first_content();
        }
        if let Target::Input(id) = &self.focus
            && !self
                .inventory
                .inputs
                .iter()
                .any(|input| &input.id == id && self.home_input_visible(input))
        {
            self.focus = self.first_content();
        }
    }

    pub fn first_content(&self) -> Target {
        if self.page == Page::Home
            && self.preferences.home.inputs_first
            && let Some(input) = self
                .inventory
                .inputs
                .iter()
                .find(|input| self.home_input_visible(input))
        {
            return Target::Input(input.id.clone());
        }
        match self.page {
            Page::Home => self
                .apps()
                .get(((self.home_scroll - 0.01).max(0.) / self.home_cards().pitch).ceil() as usize)
                .map(|app| Target::App(app.id.clone()))
                .unwrap_or(if self.error.is_some() {
                    Target::Retry
                } else {
                    Target::Browse
                }),
            Page::Apps => self
                .apps()
                .first()
                .map(|app| Target::App(app.id.clone()))
                .unwrap_or(Target::Filter(1)),
            Page::Settings => Target::Sidebar(0),
        }
    }

    pub fn focus(&mut self, target: Target, controls: &[Control]) {
        self.pointer = false;
        self.focus = target;
        self.reveal_focus(controls);
        if let Target::Sidebar(section) = self.focus {
            if self.subpage == Some(Subpage::WeatherLocation) {
                self.weather.cancel_search();
            }
            self.section = section;
            self.subpage = None;
        }
        self.dirty = true;
    }

    /// Remote focus is revealed after layout changes, including hide/reorder.
    /// Pointer paths deliberately do not call this method.
    pub fn reveal_focus(&mut self, controls: &[Control]) -> bool {
        let previous = self.scroll;
        let previous_home = self.home_scroll;
        if let Some(control) = controls.iter().find(|c| c.target == self.focus) {
            if self.page == Page::Home && matches!(self.focus, Target::App(_)) {
                let (left, right) = self.home_strip();
                if control.rect.x < left {
                    self.home_scroll += control.rect.x - left;
                } else if control.rect.x + control.rect.w > right {
                    self.home_scroll += control.rect.x + control.rect.w - right;
                }
            }
            if matches!(
                self.focus,
                Target::Nav(_)
                    | Target::Filter(_)
                    | Target::ClearFilter
                    | Target::Search
                    | Target::ClearSearch
                    | Target::Sidebar(_)
            ) {
                self.scroll = 0.;
            } else if control.scrollable {
                let top = if self.page == Page::Apps {
                    164.8
                } else {
                    120.8
                };
                if control.rect.y < top {
                    self.scroll = (self.scroll + control.rect.y - top).max(0.);
                }
                let bottom = if self.page == Page::Home && self.moving.is_none() {
                    1041.6
                } else {
                    982.4
                };
                if control.rect.y + control.rect.h > bottom {
                    self.scroll += control.rect.y + control.rect.h - bottom;
                }
            }
        }
        if self.page == Page::Home {
            self.scroll_favorites(0.);
        }
        if self.scroll != previous || self.home_scroll != previous_home {
            self.dirty = true;
        }
        self.scroll != previous || self.home_scroll != previous_home
    }

    /// Hover and pointer-down must not move the control before pointer-up.
    pub fn pointer_focus(&mut self, target: Target) {
        if !self.pointer {
            self.dirty = true;
        }
        self.pointer = true;
        if self.focus == target {
            return;
        }
        if let Target::Sidebar(section) = target {
            if self.subpage == Some(Subpage::WeatherLocation) {
                self.weather.cancel_search();
            }
            self.section = section;
            self.subpage = None;
        }
        self.focus = target;
        self.dirty = true;
    }

    pub fn hidden_count(&self) -> usize {
        self.inventory
            .apps
            .iter()
            .filter(|app| self.preferences.hidden.contains(&app.id))
            .count()
    }

    fn section_entry(&self) -> Target {
        match self.section {
            1 => Target::Background(
                BACKGROUNDS
                    .iter()
                    .position(|(id, _)| *id == self.preferences.background)
                    .unwrap_or(3),
            ),
            WEATHER_SECTION => Target::WeatherToggle,
            SYSTEM_SECTION => Target::Setting(3),
            INPUTS_SECTION => self
                .inventory
                .inputs
                .first()
                .map(|input| Target::InputMode(input.id.clone()))
                .unwrap_or(Target::Setting(3)),
            _ => Target::Setting(0),
        }
    }

    fn close_subpage(&mut self) {
        if matches!(self.subpage, Some(Subpage::ManagedApp(_))) {
            self.catalog_back();
            return;
        }
        if self.subpage == Some(Subpage::WeatherLocation) {
            self.weather.cancel_search();
        }
        self.panel_started = None;
        self.pointer = false;
        self.focus = match self.subpage.take() {
            Some(Subpage::AppNames) => Target::Setting(0),
            Some(Subpage::Greeting) if self.greeting_from_home => {
                self.subpage = Some(Subpage::Home);
                self.greeting_from_home = false;
                Target::HomeOption(HomeOption::GreetingText)
            }
            Some(Subpage::Greeting) => Target::Setting(6),
            Some(Subpage::Home) => Target::Setting(11),
            Some(Subpage::Appearance) => Target::Setting(12),
            Some(Subpage::Shortcuts) => Target::Setting(13),
            Some(Subpage::Wallpapers) => Target::Setting(17),
            Some(Subpage::Rotation) => Target::Setting(18),
            Some(Subpage::Display) => Target::Setting(21),
            Some(Subpage::StartupFocus) => {
                self.subpage = Some(Subpage::Home);
                Target::HomeOption(HomeOption::StartupFocus)
            }
            Some(Subpage::Clock) => Target::Setting(1),
            Some(Subpage::ClockFormat) => {
                self.subpage = Some(Subpage::Clock);
                Target::ClockFormat
            }
            Some(Subpage::TimeFormat) => {
                self.subpage = Some(Subpage::Clock);
                Target::ClockTimeFormat
            }
            Some(Subpage::Input(id)) => Target::InputMode(id),
            Some(Subpage::WeatherLocation) => Target::WeatherLocation,
            Some(Subpage::Temperature) => Target::WeatherTemperature,
            Some(Subpage::Language) => Target::Setting(4),
            Some(Subpage::TextRendering) => Target::Setting(7),
            Some(Subpage::SystemInfo) => Target::Setting(8),
            Some(Subpage::Privacy) => Target::Setting(9),
            Some(Subpage::AppManagement) => Target::Setting(22),
            Some(Subpage::ManagedApp(_)) => unreachable!(),
            None => Target::Sidebar(self.section),
        };
        self.scroll = 0.;
        self.dirty = true;
    }

    pub fn direction(&mut self, direction: Direction, controls: &[Control]) -> Vec<Effect> {
        self.pointer = false;
        self.dirty = true;
        if self.display_trial {
            self.focus = if self.focus == Target::DisplayKeep {
                Target::DisplayHD
            } else {
                Target::DisplayKeep
            };
            return vec![];
        }
        if self.slider_allowed(&self.focus)
            && matches!(direction, Direction::Left | Direction::Right)
            && let Some((index, values)) = self.slider_values(&self.focus)
        {
            let next = if matches!(direction, Direction::Left) {
                index.saturating_sub(1)
            } else {
                (index + 1).min(values.len().saturating_sub(1))
            };
            return self.set_slider(&self.focus.clone(), next);
        }
        if matches!(self.overlay, Some(Overlay::Profile)) {
            let rows = [
                Target::CloseProfile,
                Target::ProfileName,
                Target::ProfileAvatar,
                Target::ProfileClearName,
            ];
            if let Some(index) = rows.iter().position(|target| target == &self.focus) {
                let next = match direction {
                    Direction::Up => index.saturating_sub(1),
                    Direction::Down => (index + 1).min(rows.len() - 1),
                    Direction::Left | Direction::Right => index,
                };
                self.focus(rows[next].clone(), controls);
            }
            return vec![];
        }
        if self.overlay.is_none() {
            if matches!(self.focus, Target::Nav(_)) && matches!(direction, Direction::Up) {
                self.focus(Target::ControlCenter, controls);
                return vec![];
            }
            let header_focus = matches!(
                self.focus,
                Target::Nav(_) | Target::ControlCenter | Target::Profile | Target::TvSettings
            );
            if header_focus && matches!(direction, Direction::Left | Direction::Right) {
                let left_profile = self.preferences.header.combined
                    && self.preferences.header.side == crate::features::header::Side::Left;
                let order = if left_profile {
                    [
                        Target::Profile,
                        Target::Nav(0),
                        Target::Nav(1),
                        Target::Nav(2),
                        Target::ControlCenter,
                        Target::TvSettings,
                    ]
                } else {
                    [
                        Target::Nav(0),
                        Target::Nav(1),
                        Target::Nav(2),
                        Target::Profile,
                        Target::ControlCenter,
                        Target::TvSettings,
                    ]
                };
                if let Some(index) = order.iter().position(|target| *target == self.focus) {
                    let next = if matches!(direction, Direction::Left) {
                        index.saturating_sub(1)
                    } else {
                        (index + 1).min(order.len() - 1)
                    };
                    if next == index {
                        return vec![];
                    }
                    if let Target::Nav(page) = order[next] {
                        if page == self.page_index() {
                            self.focus(Target::Nav(page), controls);
                        } else {
                            self.open(page);
                        }
                    } else {
                        self.focus(order[next].clone(), controls);
                    }
                }
                return vec![];
            }
            if matches!(
                self.focus,
                Target::ControlCenter | Target::Profile | Target::TvSettings
            ) {
                if matches!(direction, Direction::Down) {
                    self.focus(Target::Nav(self.page_index()), controls);
                }
                return vec![];
            }
        }
        if self.overlay.is_none()
            && matches!(
                self.subpage,
                Some(Subpage::Wallpapers | Subpage::Rotation | Subpage::Display)
            )
        {
            let rows = if self.subpage == Some(Subpage::Display) {
                if self.display_trial {
                    vec![Target::DisplayHD, Target::DisplayKeep]
                } else {
                    vec![Target::DisplayHD, Target::DisplayUHD]
                }
            } else if self.subpage == Some(Subpage::Rotation) {
                self.rotation_controls()
            } else {
                self.photo_controls()
            };
            if self.focus == Target::Nav(2) && matches!(direction, Direction::Down) {
                self.focus(rows[0].clone(), controls);
                return vec![];
            }
            if let Some(index) = rows.iter().position(|r| r == &self.focus) {
                match direction {
                    Direction::Up if index == 0 => self.focus(Target::Nav(2), controls),
                    Direction::Up => self.focus(rows[index - 1].clone(), controls),
                    Direction::Down if index + 1 < rows.len() => {
                        self.focus(rows[index + 1].clone(), controls)
                    }
                    Direction::Left => self.close_subpage(),
                    _ => {}
                }
                return vec![];
            }
        }
        if self.overlay.is_none() && self.subpage == Some(Subpage::Shortcuts) {
            let rows = Self::shortcut_controls();
            if self.focus == Target::Nav(2) && matches!(direction, Direction::Down) {
                self.focus(rows[0].clone(), controls);
                return vec![];
            }
            if let Some(index) = rows.iter().position(|row| row == &self.focus) {
                match direction {
                    Direction::Up if index == 0 => self.focus(Target::Nav(2), controls),
                    Direction::Up => self.focus(rows[index - 1].clone(), controls),
                    Direction::Down if index + 1 < rows.len() => {
                        self.focus(rows[index + 1].clone(), controls)
                    }
                    Direction::Left => self.close_subpage(),
                    _ => {}
                }
                return vec![];
            }
        }
        if self.overlay.is_none() && self.subpage == Some(Subpage::Appearance) {
            if self.focus == Target::Nav(2) && matches!(direction, Direction::Down) {
                self.focus(Target::AppearanceOption(AppearanceOption::ALL[0]), controls);
                return vec![];
            }
            if let Target::AppearanceOption(option) = self.focus
                && let Some(index) = AppearanceOption::ALL
                    .iter()
                    .position(|item| *item == option)
            {
                match direction {
                    Direction::Up if index == 0 => self.focus(Target::Nav(2), controls),
                    Direction::Up => self.focus(
                        Target::AppearanceOption(AppearanceOption::ALL[index - 1]),
                        controls,
                    ),
                    Direction::Down if index + 1 < AppearanceOption::ALL.len() => self.focus(
                        Target::AppearanceOption(AppearanceOption::ALL[index + 1]),
                        controls,
                    ),
                    Direction::Left => self.close_subpage(),
                    _ => {}
                }
                return vec![];
            }
        }
        if self.overlay.is_none() && self.subpage == Some(Subpage::Home) {
            if self.focus == Target::Nav(2) && matches!(direction, Direction::Down) {
                self.focus(Target::HomeOption(HomeOption::ALL[0]), controls);
                return vec![];
            }
            if let Target::HomeOption(option) = self.focus
                && let Some(index) = HomeOption::ALL.iter().position(|item| *item == option)
            {
                match direction {
                    Direction::Up if index == 0 => self.focus(Target::Nav(2), controls),
                    Direction::Up => {
                        self.focus(Target::HomeOption(HomeOption::ALL[index - 1]), controls)
                    }
                    Direction::Down if index + 1 < HomeOption::ALL.len() => {
                        self.focus(Target::HomeOption(HomeOption::ALL[index + 1]), controls)
                    }
                    Direction::Left => self.close_subpage(),
                    Direction::Right
                        if matches!(
                            option,
                            HomeOption::GreetingText | HomeOption::StartupFocus
                        ) =>
                    {
                        return self.activate();
                    }
                    _ => {}
                }
                return vec![];
            }
        }
        if self.overlay.is_none() && self.subpage == Some(Subpage::StartupFocus) {
            let choices = self.startup_choices();
            let index = if let Target::StartupFocus(choice) = &self.focus {
                choices.iter().position(|(item, _)| item == choice)
            } else {
                None
            };
            if let Some(index) = index {
                match direction {
                    Direction::Left => self.close_subpage(),
                    Direction::Up if index == 0 => self.focus(Target::Nav(2), controls),
                    Direction::Up => {
                        self.focus(Target::StartupFocus(choices[index - 1].0.clone()), controls)
                    }
                    Direction::Down if index + 1 < choices.len() => {
                        self.focus(Target::StartupFocus(choices[index + 1].0.clone()), controls)
                    }
                    _ => {}
                }
                return vec![];
            }
            if matches!(direction, Direction::Down) {
                self.focus(Target::StartupFocus(StartupFocus::Auto), controls);
                return vec![];
            }
        }
        if self.overlay.is_none()
            && matches!(
                self.subpage,
                Some(Subpage::AppManagement | Subpage::ManagedApp(_))
            )
        {
            let rows = self.catalog_rows();
            if self.focus == Target::Nav(2) && matches!(direction, Direction::Down) {
                self.focus(rows[0].clone(), controls);
                return vec![];
            }
            if let Some(index) = rows.iter().position(|target| target == &self.focus) {
                match direction {
                    Direction::Left if self.focus == Target::CatalogNext => {
                        self.focus(Target::CatalogPrevious, controls)
                    }
                    Direction::Left if self.focus == Target::CatalogRestoreAll => {
                        self.focus(Target::CatalogUndo, controls)
                    }
                    Direction::Left => self.close_subpage(),
                    Direction::Right if self.focus == Target::CatalogPrevious => {
                        self.focus(Target::CatalogNext, controls)
                    }
                    Direction::Right if self.focus == Target::CatalogUndo => {
                        self.focus(Target::CatalogRestoreAll, controls)
                    }
                    Direction::Down if self.focus == Target::CatalogPrevious => {
                        self.focus(Target::CatalogUndo, controls)
                    }
                    Direction::Down if self.focus == Target::CatalogNext => {
                        self.focus(Target::CatalogRestoreAll, controls)
                    }
                    Direction::Up if self.focus == Target::CatalogNext => {
                        self.focus(Target::CatalogFilter, controls)
                    }
                    Direction::Up if self.focus == Target::CatalogUndo => {
                        self.focus(Target::CatalogPrevious, controls)
                    }
                    Direction::Up if self.focus == Target::CatalogRestoreAll => {
                        self.focus(Target::CatalogNext, controls)
                    }
                    Direction::Down if self.focus == Target::CatalogUndo && rows.len() > 6 => {
                        self.focus(rows[6].clone(), controls)
                    }

                    Direction::Up if index == 0 => self.focus(Target::Nav(2), controls),
                    Direction::Up => self.focus(rows[index - 1].clone(), controls),
                    Direction::Down if index + 1 < rows.len() => {
                        self.focus(rows[index + 1].clone(), controls)
                    }
                    Direction::Right if matches!(self.focus, Target::CatalogApp(_)) => {
                        return self.catalog_action();
                    }
                    _ => {}
                }
            }
            return vec![];
        }
        let privacy_page = self.subpage == Some(Subpage::Privacy);
        if (self.subpage == Some(Subpage::SystemInfo) || privacy_page) && self.overlay.is_none() {
            let refresh = if privacy_page {
                Target::PrivacyRefresh
            } else {
                Target::SystemRefresh
            };
            let field = |index| {
                if privacy_page {
                    Target::PrivacyField(index)
                } else {
                    Target::SystemField(index)
                }
            };
            let fields = if privacy_page {
                crate::features::privacy::FIELDS.len()
            } else {
                7
            };
            if self.focus == Target::Nav(2) && matches!(direction, Direction::Down) {
                self.focus(refresh, controls);
                return vec![];
            }
            let index = match self.focus {
                Target::SystemRefresh | Target::PrivacyRefresh => Some(0),
                Target::SystemField(index) | Target::PrivacyField(index) => Some(index + 1),
                _ => None,
            };
            if let Some(index) = index {
                match direction {
                    Direction::Left => self.close_subpage(),
                    Direction::Up if index == 0 => self.focus(Target::Nav(2), controls),
                    Direction::Up => self.focus(
                        if index == 1 {
                            refresh
                        } else {
                            field(index - 2)
                        },
                        controls,
                    ),
                    Direction::Down if index < fields => self.focus(field(index), controls),
                    _ => {}
                }
                return vec![];
            }
        }
        if self.overlay.is_none() && self.page == Page::Settings && self.subpage.is_none() {
            match (&self.focus, direction) {
                (Target::Setting(17), Direction::Up) => {
                    self.focus(Target::Background(6), controls);
                    return vec![];
                }
                (Target::Setting(17), Direction::Down) => {
                    self.focus(Target::Setting(18), controls);
                    return vec![];
                }
                (Target::Setting(18), Direction::Up) => {
                    self.focus(Target::Setting(17), controls);
                    return vec![];
                }
                _ => {}
            }
        }
        if self.recents_open() {
            return self.recent_direction(direction);
        }
        if matches!(self.overlay, Some(Overlay::SystemKeyboard)) {
            // The TV keyboard owns its arrows until Done/Back dismisses it.
            return vec![];
        }
        if let Some(id) = self.moving.clone() {
            let mut ids = self.preferences.favorites(&self.inventory.apps);
            let slots: Vec<_> = ids
                .iter()
                .enumerate()
                .filter_map(|(index, id)| {
                    (!self.preferences.hidden.contains(id)
                        && self.inventory.apps.iter().any(|app| &app.id == id))
                    .then_some(index)
                })
                .collect();
            if let Some(index) = slots.iter().position(|slot| ids[*slot] == id) {
                let (dx, dy) = match direction {
                    Direction::Left => (-1, 0),
                    Direction::Right => (1, 0),
                    Direction::Up => (0, -1),
                    Direction::Down => (0, 1),
                };
                if let Some(next) = self.home_layout(slots.len()).neighbor(index, dx, dy) {
                    ids.swap(slots[index], slots[next]);
                    self.preferences.favorites = Some(ids);
                    self.dirty = true;
                    return vec![Effect::Save];
                }
            }
            return vec![];
        }
        if self.overlay.is_none() {
            if self.page == Page::Home
                && let Target::App(id) = &self.focus
            {
                let apps = self.apps();
                if let Some(index) = apps.iter().position(|app| &app.id == id) {
                    let layout = self.home_layout(apps.len());
                    match direction {
                        Direction::Left | Direction::Right => {
                            let dx = if matches!(direction, Direction::Left) {
                                -1
                            } else {
                                1
                            };
                            if let Some(next) = layout.neighbor(index, dx, 0) {
                                let id = apps[next].id.clone();
                                self.focus(Target::App(id), controls);
                            }
                        }
                        Direction::Up => {
                            let target = layout
                                .neighbor(index, 0, -1)
                                .map(|next| Target::App(apps[next].id.clone()))
                                .or_else(|| self.adjacent_home_input(controls, true))
                                .unwrap_or(Target::Nav(0));
                            self.focus(target, controls);
                        }
                        Direction::Down => {
                            if let Some(next) = layout.neighbor(index, 0, 1) {
                                let id = apps[next].id.clone();
                                self.focus(Target::App(id), controls);
                                return vec![];
                            }
                            let next = self.adjacent_home_input(controls, false);
                            if let Some(next) = next {
                                self.focus(next, controls);
                            }
                        }
                    }
                    return vec![];
                }
            }
            if self.subpage == Some(Subpage::Clock) {
                let rows = [
                    Target::ClockFormat,
                    Target::ClockTimeFormat,
                    Target::ClockAvatar,
                    Target::ClockSide,
                    Target::AvatarStyle,
                ];
                if let Some(index) = rows.iter().position(|target| target == &self.focus) {
                    match direction {
                        Direction::Up if index == 0 => self.focus(Target::Nav(2), controls),
                        Direction::Up => self.focus(rows[index - 1].clone(), controls),
                        Direction::Down if index + 1 < rows.len() => {
                            self.focus(rows[index + 1].clone(), controls)
                        }
                        Direction::Left => self.close_subpage(),
                        Direction::Right
                            if matches!(
                                self.focus,
                                Target::ClockFormat | Target::ClockTimeFormat
                            ) =>
                        {
                            return self.activate();
                        }
                        _ => {}
                    }
                    return vec![];
                }
            }
            let setting_rows = match self.subpage {
                Some(Subpage::Greeting) => Some([Target::GreetingName, Target::GreetingNameClear]),
                Some(Subpage::AppNames) => Some([Target::AppNamesHome, Target::AppNamesApps]),
                Some(Subpage::Clock) => Some([Target::ClockFormat, Target::ClockTimeFormat]),
                _ => None,
            };
            if let Some(rows) = setting_rows
                && let Some(index) = rows.iter().position(|target| *target == self.focus)
            {
                match direction {
                    Direction::Left => self.close_subpage(),
                    Direction::Up => self.focus(
                        if index == 0 {
                            Target::Nav(2)
                        } else {
                            rows[0].clone()
                        },
                        controls,
                    ),
                    Direction::Down if index == 0 => self.focus(rows[1].clone(), controls),
                    Direction::Right
                        if matches!(
                            self.focus,
                            Target::GreetingName | Target::ClockFormat | Target::ClockTimeFormat
                        ) =>
                    {
                        return self.activate();
                    }
                    _ => {}
                }
                return vec![];
            }
            if let Target::Choice(index) = self.focus {
                let count = match self.subpage {
                    Some(
                        Subpage::ClockFormat
                        | Subpage::TimeFormat
                        | Subpage::Temperature
                        | Subpage::TextRendering,
                    ) => 2,
                    Some(Subpage::Language) => Language::ALL.len(),
                    _ => 3,
                };
                match direction {
                    Direction::Left => self.close_subpage(),
                    Direction::Up => self.focus(
                        if index == 0 {
                            Target::Nav(2)
                        } else {
                            Target::Choice(index - 1)
                        },
                        controls,
                    ),
                    Direction::Down
                        if index + 1 == count
                            && matches!(self.subpage, Some(Subpage::Input(_))) =>
                    {
                        self.focus(Target::InputRename, controls)
                    }
                    Direction::Down => self.focus(Target::Choice((index + 1) % count), controls),
                    Direction::Right => {}
                }
                return vec![];
            }
            if matches!(
                self.focus,
                Target::InputRename | Target::InputNameReset | Target::InputXboxPreset
            ) {
                let has_reset = controls.iter().any(|c| c.target == Target::InputNameReset);
                let next = match (&self.focus, direction) {
                    (Target::InputRename, Direction::Up) => Some(Target::Choice(2)),
                    (Target::InputNameReset, Direction::Up) => Some(Target::InputRename),
                    (Target::InputXboxPreset, Direction::Up) => Some(if has_reset {
                        Target::InputNameReset
                    } else {
                        Target::InputRename
                    }),
                    (Target::InputRename, Direction::Down) => Some(if has_reset {
                        Target::InputNameReset
                    } else {
                        Target::InputXboxPreset
                    }),
                    (Target::InputNameReset, Direction::Down) => Some(Target::InputXboxPreset),
                    (_, Direction::Left) => {
                        self.close_subpage();
                        None
                    }
                    _ => None,
                };
                if let Some(next) = next {
                    self.focus(next, controls);
                }
                return vec![];
            }
            if let Target::Sidebar(index) = self.focus {
                match direction {
                    Direction::Right => {
                        let entry = self.section_entry();
                        self.focus(entry, controls);
                    }
                    Direction::Up => self.focus(
                        if index == 0 {
                            Target::Nav(2)
                        } else {
                            Target::Sidebar(index - 1)
                        },
                        controls,
                    ),
                    Direction::Down => {
                        self.focus(Target::Sidebar((index + 1) % SECTION_COUNT), controls)
                    }
                    Direction::Left => {}
                }
                return vec![];
            }
            if matches!(
                self.focus,
                Target::Setting(0 | 1 | 4 | 6 | 7 | 8 | 9 | 11..=22)
                    | Target::InputMode(_)
                    | Target::WeatherLocation
                    | Target::WeatherTemperature
            ) && matches!(direction, Direction::Right)
            {
                return self.activate();
            }
            if matches!(self.focus, Target::Setting(_) | Target::WeatherToggle)
                && matches!(direction, Direction::Right)
            {
                return vec![];
            }
            if let Target::InputMode(id) = &self.focus
                && matches!(direction, Direction::Up)
                && self
                    .inventory
                    .inputs
                    .first()
                    .is_some_and(|input| &input.id == id)
            {
                self.focus(Target::Sidebar(INPUTS_SECTION), controls);
                return vec![];
            }
            if self.focus == Target::Setting(0) && matches!(direction, Direction::Up) {
                self.focus(Target::Nav(2), controls);
                return vec![];
            }
            if self.focus == Target::WeatherToggle && matches!(direction, Direction::Up) {
                self.focus(Target::Nav(2), controls);
                return vec![];
            }
            if matches!(
                self.focus,
                Target::WeatherQuery | Target::WeatherSearch | Target::WeatherClear
            ) {
                match direction {
                    Direction::Left => {
                        self.close_subpage();
                        return vec![];
                    }
                    Direction::Up => {
                        self.focus(
                            if self.preferences.weather_location.is_some()
                                && self.focus != Target::WeatherClear
                            {
                                Target::WeatherClear
                            } else {
                                Target::Nav(2)
                            },
                            controls,
                        );
                        return vec![];
                    }
                    Direction::Down => {
                        let target = if self.focus == Target::WeatherClear {
                            Some(Target::WeatherQuery)
                        } else if !self.weather.results.is_empty() {
                            Some(Target::WeatherResult(0))
                        } else if self.focus == Target::WeatherQuery {
                            Some(Target::WeatherSearch)
                        } else {
                            None
                        };
                        if let Some(target) = target {
                            self.focus(target, controls);
                        }
                        return vec![];
                    }
                    _ => {}
                }
            }
            if let Target::WeatherResult(index) = self.focus {
                match direction {
                    Direction::Left => self.close_subpage(),
                    Direction::Up => self.focus(
                        if index == 0 {
                            Target::WeatherQuery
                        } else {
                            Target::WeatherResult(index - 1)
                        },
                        controls,
                    ),
                    Direction::Down if index + 1 < self.weather.results.len() => {
                        self.focus(Target::WeatherResult(index + 1), controls)
                    }
                    _ => {}
                }
                return vec![];
            }
            if self.focus == Target::Search {
                match direction {
                    Direction::Left => {
                        self.focus(
                            Target::Filter(if self.hidden_count() > 0 { 5 } else { 4 }),
                            controls,
                        );
                        return vec![];
                    }
                    Direction::Up => {
                        self.focus(Target::Nav(1), controls);
                        return vec![];
                    }
                    _ => {}
                }
            }
            if let Target::Nav(index) = self.focus {
                let next = match direction {
                    Direction::Left if index > 0 => Some(index - 1),
                    Direction::Right if index < 2 => Some(index + 1),
                    _ => None,
                };
                if let Some(next) = next {
                    self.open(next);
                    return vec![];
                }
            }
            if self.page == Page::Apps
                && let Target::App(id) = &self.focus
                && matches!(direction, Direction::Up | Direction::Down)
            {
                let apps = self.apps();
                if let Some(index) = apps.iter().position(|app| &app.id == id) {
                    // A preceding row may be above the visible scroll region.
                    // Its grid index still defines the route; reveal_focus then
                    // scrolls it into view, without letting the fixed nav steal Up.
                    let columns = self.apps_cards().columns;
                    let next = match direction {
                        Direction::Up if index < columns => Some(Target::Filter(1)),
                        Direction::Up => Some(Target::App(apps[index - columns].id.clone())),
                        Direction::Down if (index / columns + 1) * columns < apps.len() => Some(
                            Target::App(apps[(index + columns).min(apps.len() - 1)].id.clone()),
                        ),
                        _ => None,
                    };
                    if let Some(next) = next {
                        self.focus(next, controls);
                    }
                    return vec![];
                }
            }
            if let Target::Background(index) = self.focus {
                match direction {
                    Direction::Down if index >= 6 => {
                        self.focus(Target::Setting(17), controls);
                        return vec![];
                    }
                    Direction::Right if index + 1 < BACKGROUNDS.len() => {
                        self.focus(Target::Background(index + 1), controls);
                        return vec![];
                    }
                    Direction::Left if index % 3 != 0 => {
                        self.focus(Target::Background(index - 1), controls);
                        return vec![];
                    }
                    _ => {}
                }
            }
            if matches!(self.focus, Target::Nav(_)) && matches!(direction, Direction::Down) {
                if self.page == Page::Settings {
                    self.subpage = None;
                    self.section = 0;
                }
                self.focus(self.first_content(), controls);
                return vec![];
            }
            if self.page == Page::Settings
                && matches!(direction, Direction::Left)
                && !matches!(
                    self.focus,
                    Target::Nav(_) | Target::Sidebar(_) | Target::TvSettings
                )
            {
                if let Target::Background(index) = self.focus {
                    if index % 3 != 0 { /* left follows gallery order */
                    } else {
                        self.focus(Target::Sidebar(self.section), controls);
                        return vec![];
                    }
                } else {
                    self.focus(Target::Sidebar(self.section), controls);
                    return vec![];
                }
            }
        }
        let Some(current) = controls.iter().find(|c| c.target == self.focus) else {
            self.focus(self.first_content(), controls);
            return vec![];
        };
        let (x, y) = current.rect.center();
        let (strip_left, strip_right) = self.home_strip();
        let next = controls
            .iter()
            .filter(|candidate| candidate.target != self.focus)
            .filter(|candidate| {
                self.page != Page::Home
                    || !matches!(candidate.target, Target::App(_))
                    || (candidate.rect.x >= strip_left - 0.1
                        && candidate.rect.x + candidate.rect.w <= strip_right + 0.1)
            })
            .filter_map(|candidate| {
                // Header controls never steal an Up route from the active tab.
                if self.overlay.is_none()
                    && matches!(direction, Direction::Up)
                    && candidate.rect.y < 150.
                    && candidate.target != Target::Nav(self.page_index())
                {
                    return None;
                }
                let (cx, cy) = candidate.rect.center();
                let (primary, cross, overlap) = match direction {
                    Direction::Up => (
                        y - cy,
                        (x - cx).abs(),
                        (current.rect.w + candidate.rect.w) / 2.,
                    ),
                    Direction::Down => (
                        cy - y,
                        (x - cx).abs(),
                        (current.rect.w + candidate.rect.w) / 2.,
                    ),
                    Direction::Left => (
                        x - cx,
                        (y - cy).abs(),
                        (current.rect.h + candidate.rect.h) / 2.,
                    ),
                    Direction::Right => (
                        cx - x,
                        (y - cy).abs(),
                        (current.rect.h + candidate.rect.h) / 2.,
                    ),
                };
                (primary > 1.).then_some((
                    candidate.target.clone(),
                    primary + cross * 2. + if cross >= overlap { 10000. } else { 0. },
                ))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(target, _)| target);
        if let Some(next) = next {
            self.focus(next, controls);
        }
        vec![]
    }

    pub fn options(&mut self) {
        if self.overlay.is_some() || self.moving.is_some() {
            return;
        }
        if let Target::App(id) = &self.focus {
            let removal = self
                .inventory
                .apps
                .iter()
                .find(|app| &app.id == id)
                .and_then(|app| app.removal.clone())
                .filter(|target| {
                    target.id == *id
                        && !management::protected(id)
                        && !crate::scroll_preview::is_placeholder(id)
                        && ((target.route == Route::Preview) != self.webos)
                });
            self.overlay = Some(Overlay::Options {
                id: id.clone(),
                restore: self.focus.clone(),
                removal,
            });
            self.focus = Target::Option(0);
            if !self.preferences.reduced_motion {
                self.panel_started = Some(Instant::now());
            }
            self.dirty = true;
        }
    }

    pub fn hold(&mut self) {
        if self.page == Page::Home && self.overlay.is_none() {
            if let Target::App(id) = &self.focus {
                self.moving = Some(id.clone());
                self.notice = None;
                self.dirty = true;
            }
        } else {
            self.options();
        }
    }

    pub fn text(&mut self, text: &str) {
        if self.text_input_active() {
            if matches!(
                self.keyboard_field,
                TextField::PhotoFolder | TextField::PhotoUrl
            ) {
                self.photo_draft.extend(
                    text.chars()
                        .filter(|c| !c.is_control())
                        .take(2048usize.saturating_sub(self.photo_draft.chars().count())),
                );
                self.dirty = true;
                return;
            }
            if self.keyboard_field == TextField::Location {
                let mut query = self.weather.query.clone();
                query.extend(
                    text.chars()
                        .filter(|c| !c.is_control())
                        .take(120usize.saturating_sub(query.chars().count())),
                );
                self.weather.edit(query);
                self.dirty = true;
                return;
            }
            if matches!(
                self.keyboard_field,
                TextField::Name | TextField::ProfileName
            ) {
                let limit: usize = if self.keyboard_field == TextField::Name
                    && self.preferences.home.greeting_mode == GreetingMode::Static
                {
                    80
                } else {
                    40
                };
                let name = &mut self.greeting_name_draft;
                name.extend(
                    text.chars()
                        .filter(|c| !c.is_control())
                        .take(limit.saturating_sub(name.chars().count())),
                );
                self.dirty = true;
                return;
            }
            if self.keyboard_field == TextField::InputName {
                let length = self.input_name.chars().count();
                self.input_name.extend(
                    text.chars()
                        .filter(|c| !c.is_control())
                        .take(INPUT_NAME_LIMIT.saturating_sub(length)),
                );
                self.dirty = true;
                return;
            }
            self.query.extend(
                text.chars()
                    .filter(|c| !c.is_control())
                    .take(120usize.saturating_sub(self.query.chars().count())),
            );
            self.scroll = 0.;
            self.dirty = true;
        }
    }

    pub fn delete(&mut self) {
        if self.text_input_active() {
            if matches!(
                self.keyboard_field,
                TextField::PhotoFolder | TextField::PhotoUrl
            ) {
                self.photo_draft.pop();
                self.dirty = true;
                return;
            }
            if self.keyboard_field == TextField::Location {
                let mut query = self.weather.query.clone();
                query.pop();
                self.weather.edit(query);
            } else if matches!(
                self.keyboard_field,
                TextField::Name | TextField::ProfileName
            ) {
                self.greeting_name_draft.pop();
            } else if self.keyboard_field == TextField::InputName {
                self.input_name.pop();
            } else {
                self.query.pop();
            }
            self.dirty = true;
        }
    }

    pub fn text_input_active(&self) -> bool {
        matches!(
            self.overlay,
            Some(Overlay::Keyboard | Overlay::SystemKeyboard)
        )
    }

    pub fn finish_text_input(&mut self) -> Vec<Effect> {
        if self.text_input_active() {
            self.overlay = None;
            self.panel_started = None;
            self.focus = self.keyboard_return();
            self.pointer = false;
            self.dirty = true;
            return self.commit_text_input();
        }
        vec![]
    }

    fn commit_text_input(&mut self) -> Vec<Effect> {
        if matches!(
            self.keyboard_field,
            TextField::PhotoFolder | TextField::PhotoUrl
        ) {
            let value = std::mem::take(&mut self.photo_draft).trim().to_owned();
            if value.is_empty() {
                return vec![];
            }
            if self.keyboard_field == TextField::PhotoFolder {
                self.preferences.wallpapers.folder = value;
            } else {
                self.preferences.wallpapers.url = value;
            }
            return vec![Effect::Save];
        }
        if matches!(
            self.keyboard_field,
            TextField::Name | TextField::ProfileName
        ) {
            let name = std::mem::take(&mut self.greeting_name_draft);
            let name = name.trim();
            // Empty dismissal preserves the saved value; Clear name is explicit.
            if self.keyboard_field == TextField::ProfileName {
                self.overlay = Some(Overlay::Profile);
                self.focus = Target::ProfileName;
            }
            let saved = if self.keyboard_field == TextField::Name
                && self.preferences.home.greeting_mode == GreetingMode::Static
            {
                &mut self.preferences.home.custom_greeting
            } else {
                &mut self.preferences.greeting_name
            };
            if !name.is_empty() && saved != name {
                *saved = name.into();
                return vec![Effect::Save];
            }
            return vec![];
        }
        self.commit_input_name()
    }

    /// Closing the keyboard by any route keeps a typed input name. An empty
    /// field leaves the saved name unchanged; Clear name removes it explicitly.
    fn commit_input_name(&mut self) -> Vec<Effect> {
        let name = std::mem::take(&mut self.input_name);
        if self.keyboard_field != TextField::InputName || clean_input_name(&name).is_empty() {
            return vec![];
        }
        let Some(Subpage::Input(id)) = self.subpage.clone() else {
            return vec![];
        };
        if self.preferences.set_input_name(&id, &name) {
            vec![Effect::Save]
        } else {
            vec![]
        }
    }

    pub fn delete_surrounding_text(&mut self, offset: i32, length: i32) {
        if !self.text_input_active() || length <= 0 {
            return;
        }
        // SDL-webOS forwards the native text model's UTF-8 byte offsets,
        // relative to the insertion cursor (at the end of our query).
        let mut query = self.editing_text().to_owned();
        let mut start =
            (query.len() as i64 + i64::from(offset)).clamp(0, query.len() as i64) as usize;
        let mut end = start.saturating_add(length as usize).min(query.len());
        while !query.is_char_boundary(start) {
            start -= 1;
        }
        while !query.is_char_boundary(end) {
            end += 1;
        }
        query.replace_range(start..end, "");
        if self.keyboard_field == TextField::Location {
            self.weather.edit(query);
        } else if matches!(
            self.keyboard_field,
            TextField::Name | TextField::ProfileName
        ) {
            self.greeting_name_draft = query;
        } else if self.keyboard_field == TextField::InputName {
            self.input_name = query;
        } else {
            self.query = query;
            self.scroll = 0.;
        }
        self.dirty = true;
    }

    pub fn editing_text(&self) -> &str {
        match self.keyboard_field {
            TextField::Location => &self.weather.query,
            TextField::Name | TextField::ProfileName => &self.greeting_name_draft,
            TextField::InputName => &self.input_name,
            TextField::PhotoFolder | TextField::PhotoUrl => &self.photo_draft,
            TextField::Apps => &self.query,
        }
    }
    pub fn open_keyboard(&mut self, field: TextField) {
        self.keyboard_field = field;
        if matches!(field, TextField::PhotoFolder | TextField::PhotoUrl) {
            self.photo_draft.clear();
        }
        if matches!(field, TextField::Name | TextField::ProfileName) {
            // SDL-webOS cannot pre-fill the system keyboard's text model.
            self.greeting_name_draft.clear();
        }
        if field == TextField::InputName {
            // The TV keyboard starts empty, so a new name replaces the old one.
            self.input_name.clear();
        }
        if cfg!(all(
            feature = "webos",
            target_os = "linux",
            target_arch = "arm"
        )) {
            self.overlay = Some(Overlay::SystemKeyboard);
            self.focus = match field {
                TextField::Location => Target::WeatherQuery,
                TextField::Name => Target::GreetingName,
                TextField::ProfileName => Target::ProfileName,
                TextField::InputName => Target::InputRename,
                TextField::PhotoFolder => Target::PhotoFolder,
                TextField::PhotoUrl => Target::PhotoUrl,
                TextField::Apps => Target::Search,
            };
            self.panel_started = None;
            self.dirty = true;
            return;
        }
        self.overlay = Some(Overlay::Keyboard);
        self.focus = Target::Key(0);
        if !self.preferences.reduced_motion {
            self.panel_started = Some(Instant::now());
        }
        self.dirty = true;
    }
    fn open_location(&mut self) {
        self.weather.edit(String::new());
        self.subpage = Some(Subpage::WeatherLocation);
        self.focus = Target::WeatherQuery;
        self.scroll = 0.;
        if !self.preferences.reduced_motion {
            self.panel_started = Some(Instant::now());
        }
        self.dirty = true;
    }
    fn keyboard_return(&self) -> Target {
        match self.keyboard_field {
            TextField::PhotoFolder => Target::PhotoFolder,
            TextField::PhotoUrl => Target::PhotoUrl,
            TextField::Location => Target::WeatherSearch,
            TextField::Name => Target::GreetingName,
            TextField::ProfileName => Target::ProfileName,
            TextField::InputName => Target::InputRename,
            TextField::Apps => Target::Search,
        }
    }

    pub fn activate(&mut self) -> Vec<Effect> {
        if !self.target_enabled(&self.focus) {
            return vec![];
        }
        if self.overlay.is_none()
            && matches!(
                self.subpage,
                Some(Subpage::AppManagement | Subpage::ManagedApp(_))
            )
            && !matches!(
                self.focus,
                Target::Nav(_)
                    | Target::Profile
                    | Target::ControlCenter
                    | Target::TvSettings
                    | Target::Sidebar(_)
            )
        {
            return self.catalog_action();
        }
        if self.display_trial && !matches!(self.focus, Target::DisplayHD | Target::DisplayKeep) {
            return vec![];
        }
        if self.slider_allowed(&self.focus) && self.focus != Target::SleepSlider {
            let target = self.focus.clone();
            if let Some((index, values)) = self.slider_values(&target) {
                return self.set_slider(&target, (index + 1) % values.len());
            }
        }
        if self.focus == Target::Profile && self.overlay.is_none() {
            self.overlay = Some(Overlay::Profile);
            self.focus = Target::ProfileName;
            self.pointer = false;
            self.pressed = None;
            self.panel_started = (!self.preferences.reduced_motion).then(Instant::now);
            self.dirty = true;
            return vec![];
        }
        if matches!(self.overlay, Some(Overlay::Profile)) {
            match self.focus {
                Target::CloseProfile => return self.back(),
                Target::ProfileName => self.open_keyboard(TextField::ProfileName),
                Target::ProfileClearName => {
                    self.preferences.greeting_name.clear();
                    self.dirty = true;
                    return vec![Effect::Save];
                }
                Target::ProfileAvatar => {
                    use crate::features::header::Avatar;
                    self.preferences.header.avatar =
                        if self.preferences.header.avatar == Avatar::Initials {
                            Avatar::Symbol
                        } else {
                            Avatar::Initials
                        };
                    self.dirty = true;
                    return vec![Effect::Save];
                }
                _ => {}
            }
            return vec![];
        }
        if self.focus == Target::CloseControlCenter
            && matches!(self.overlay, Some(Overlay::ControlCenter { .. }))
        {
            self.cc_timer = false;
            return self.back();
        }
        if self.focus == Target::ControlCenter {
            return self.open_control_center();
        }
        if let Target::Quick(action) = self.focus {
            return self.quick_action(action);
        }
        if self.focus == Target::SleepSlider
            && self.cc_timer
            && matches!(self.overlay, Some(Overlay::ControlCenter { .. }))
        {
            if self.controls.loading || !self.webos {
                return vec![];
            }

            if let Some(expected) = self.controls.report.sleep_index {
                let value = self.sleep_draft;
                self.cc_timer = false;
                self.back();
                self.confirm_control(crate::features::controls::Command::SleepTimer {
                    expected,
                    value,
                });
            }
            return vec![];
        }
        if self.overlay.is_none() && self.subpage == Some(Subpage::Clock) {
            match self.focus {
                Target::ClockAvatar => {
                    self.preferences.header.combined = !self.preferences.header.combined;
                    self.dirty = true;
                    return vec![Effect::Save];
                }
                Target::ClockSide => {
                    use crate::features::header::Side;
                    self.preferences.header.side = if self.preferences.header.side == Side::Left {
                        Side::Right
                    } else {
                        Side::Left
                    };
                    self.dirty = true;
                    return vec![Effect::Save];
                }
                Target::AvatarStyle => {
                    use crate::features::header::Avatar;
                    self.preferences.header.avatar =
                        if self.preferences.header.avatar == Avatar::Initials {
                            Avatar::Symbol
                        } else {
                            Avatar::Initials
                        };
                    self.dirty = true;
                    return vec![Effect::Save];
                }
                _ => {}
            }
        }
        if self.subpage == Some(Subpage::Display) && self.overlay.is_none() {
            match self.focus {
                Target::DisplayHD => {
                    self.display_change = Some(false);
                    return vec![];
                }
                Target::DisplayUHD => {
                    self.display_change = Some(true);
                    return vec![];
                }
                Target::DisplayKeep if self.display_trial => {
                    self.preferences.display.four_k = true;
                    self.display_keep = true;
                    return vec![Effect::Save];
                }
                _ => {}
            }
        }
        if self.subpage == Some(Subpage::Rotation) && self.overlay.is_none() {
            let preferences = &mut self.preferences.wallpapers.rotation;
            match &self.focus {
                Target::RotationMode => preferences.mode = preferences.mode.next(),
                Target::RotationShuffle => preferences.shuffle = !preferences.shuffle,
                Target::RotationNext => return self.rotate_background(),
                Target::RotationInclude(id) => {
                    if preferences.selected.contains(id) {
                        preferences.selected.retain(|p| p != id);
                    } else {
                        preferences.selected.push(id.clone());
                    }
                }
                _ => return vec![],
            }
            self.rotation.reset();
            self.dirty = true;
            return vec![Effect::Save];
        }
        if self.subpage == Some(Subpage::Wallpapers) && self.overlay.is_none() {
            match self.focus {
                Target::PhotoFolder => {
                    self.open_keyboard(TextField::PhotoFolder);
                    return vec![];
                }
                Target::PhotoUrl => {
                    self.open_keyboard(TextField::PhotoUrl);
                    return vec![];
                }
                Target::PhotoFit => {
                    self.preferences.wallpapers.contain = !self.preferences.wallpapers.contain;
                    self.dirty = true;
                    return vec![Effect::Save];
                }
                Target::PhotoImport => {
                    if !self.wallpapers.loading {
                        self.wallpapers.pending = true;
                    }
                    self.dirty = true;
                    return vec![];
                }
                Target::PhotoStock => {
                    self.previous_background = (!self.preferences.reduced_motion)
                        .then(|| (self.current_background().to_owned(), Instant::now()));
                    self.preferences.wallpapers.selected = None;
                    self.dirty = true;
                    return vec![Effect::Save];
                }
                Target::PhotoClearSources => {
                    self.preferences.wallpapers.folder.clear();
                    self.preferences.wallpapers.url.clear();
                    self.dirty = true;
                    return vec![Effect::Save];
                }
                Target::Photo(index) => {
                    if let Some(photo) = self.wallpapers.photos.get(index) {
                        let id = photo.id.clone();
                        if self.current_background() == id {
                            return vec![];
                        }
                        self.previous_background = (!self.preferences.reduced_motion)
                            .then(|| (self.current_background().to_owned(), Instant::now()));
                        self.preferences.wallpapers.selected = Some(id);
                        self.dirty = true;
                        return vec![Effect::Save];
                    }
                    return vec![];
                }
                _ => {}
            }
        }
        if matches!(self.focus, Target::ControlApply | Target::ControlRestore) {
            if let Some(Overlay::ControlConfirm { mut command, .. }) = self.overlay.clone() {
                if self.focus == Target::ControlRestore {
                    if let crate::features::controls::Command::PrivacySet { index, .. } = command {
                        command = crate::features::controls::Command::PrivacyRestore { index };
                    } else {
                        return vec![];
                    }
                }
                self.back();
                return self.control_request(command);
            }
            return vec![];
        }
        if let Target::PrivacyField(index @ 0..=1) = self.focus {
            if self.subpage == Some(Subpage::Privacy)
                && self.overlay.is_none()
                && self.webos
                && !self.privacy.loading
                && !self.controls.loading
            {
                use crate::features::privacy::Observation;
                if let Some(value @ (Observation::On | Observation::Off)) =
                    self.privacy.report.as_ref().map(|r| r.values[index])
                {
                    let expected = value == Observation::On;
                    self.confirm_control(crate::features::controls::Command::PrivacySet {
                        index,
                        expected,
                        value: !expected,
                    });
                }
            }
            return vec![];
        }

        if self.recents_open() {
            use crate::recents::Command;
            return match self.focus.clone() {
                Target::Recent(id) => self.recent_request(Command::Launch(id)),
                Target::RecentRetry => self.recent_request(Command::List),
                _ => vec![],
            };
        }
        if self.moving.take().is_some() {
            // Order was saved at every move. A fresh OK release finishes the
            // mode and keeps the selected favorite, rather than launching it.
            self.dirty = true;
            return vec![];
        }
        self.dirty = true;
        match self.focus.clone() {
            Target::ShortcutsEnabled | Target::ShortcutsReset | Target::ShortcutBinding(_) => {
                if self.subpage != Some(Subpage::Shortcuts) || self.overlay.is_some() {
                    return vec![];
                }
                match self.focus {
                    Target::ShortcutsEnabled => {
                        self.preferences.shortcuts.enabled = !self.preferences.shortcuts.enabled
                    }
                    Target::ShortcutBinding(button) => self.preferences.shortcuts.cycle(button),
                    Target::ShortcutsReset => {
                        self.preferences.shortcuts = shortcuts::Preferences::default()
                    }
                    _ => {}
                }
                return vec![Effect::Save];
            }
            Target::AppearanceOption(option) => {
                if self.subpage != Some(Subpage::Appearance) || self.overlay.is_some() {
                    return vec![];
                }
                match option {
                    AppearanceOption::Density => {
                        self.preferences.appearance.density =
                            self.preferences.appearance.density.next();
                        self.home_scroll = 0.;
                    }
                    AppearanceOption::Accent => {
                        self.preferences.appearance.accent =
                            self.preferences.appearance.accent.next();
                    }
                    AppearanceOption::Dimming => {
                        self.preferences.appearance.dimming =
                            self.preferences.appearance.dimming.next()
                    }
                    AppearanceOption::Blur => {
                        self.preferences.appearance.blur = self.preferences.appearance.blur.next()
                    }
                    AppearanceOption::LiquidGlass => {
                        self.preferences.appearance.liquid_glass =
                            !self.preferences.appearance.liquid_glass;
                    }
                }
                return vec![Effect::Save];
            }
            Target::HomeOption(option) => {
                if self.subpage != Some(Subpage::Home) || self.overlay.is_some() {
                    return vec![];
                }
                let home = &mut self.preferences.home;
                match option {
                    HomeOption::TwoRows => self.toggle_home_rows(),
                    HomeOption::FavoriteNames => {
                        self.preferences.show_home_names = !self.preferences.show_home_names;
                    }
                    HomeOption::FavoritesDock => {
                        home.favorites_dock = !home.favorites_dock;
                        self.home_scroll = 0.;
                    }
                    HomeOption::ShowGreeting => home.show_greeting = !home.show_greeting,
                    HomeOption::GreetingMode => {
                        home.greeting_mode = if home.greeting_mode == GreetingMode::Timed {
                            GreetingMode::Static
                        } else {
                            GreetingMode::Timed
                        }
                    }
                    HomeOption::GreetingSize => home.greeting_size = home.greeting_size.next(),
                    HomeOption::ShowDate => home.show_date = !home.show_date,
                    HomeOption::DateFormat => home.compact_date = !home.compact_date,
                    HomeOption::ShowWeekday => home.show_weekday = !home.show_weekday,
                    HomeOption::ShowInputs => home.show_inputs = !home.show_inputs,
                    HomeOption::GreetingFirst => home.greeting_first = !home.greeting_first,
                    HomeOption::InputsFirst => home.inputs_first = !home.inputs_first,
                    HomeOption::GreetingText => {
                        self.greeting_from_home = true;
                        self.subpage = Some(Subpage::Greeting);
                        self.focus = Target::GreetingName;
                        self.scroll = 0.;
                        return vec![];
                    }
                    HomeOption::StartupFocus => {
                        self.subpage = Some(Subpage::StartupFocus);
                        let desired = self.preferences.home.startup_focus.clone();
                        self.focus = Target::StartupFocus(
                            if self
                                .startup_choices()
                                .iter()
                                .any(|(item, _)| item == &desired)
                            {
                                desired
                            } else {
                                StartupFocus::Auto
                            },
                        );
                        self.scroll = 0.;
                        return vec![];
                    }
                }
                return vec![Effect::Save];
            }
            Target::StartupFocus(choice) => {
                if self.subpage == Some(Subpage::StartupFocus)
                    && self.overlay.is_none()
                    && self
                        .startup_choices()
                        .iter()
                        .any(|(item, _)| item == &choice)
                {
                    self.preferences.home.startup_focus = choice;
                    self.close_subpage();
                    return vec![Effect::Save];
                }
                return vec![];
            }
            Target::RemovalCancel => return self.back(),
            Target::RemovalCheck => {
                return self
                    .management
                    .check()
                    .map(|(generation, command)| {
                        vec![Effect::Manage {
                            generation,
                            command,
                        }]
                    })
                    .unwrap_or_default();
            }
            Target::RemovalConfirm => {
                let Some(Overlay::RemovalConfirm {
                    target,
                    title,
                    restore,
                }) = self.overlay.clone()
                else {
                    return vec![];
                };
                let unchanged = self
                    .inventory
                    .apps
                    .iter()
                    .any(|app| app.id == target.id && app.removal.as_ref() == Some(&target));
                if unchanged
                    && let Some((generation, command)) = self.management.begin(target, title)
                {
                    self.overlay = Some(Overlay::RemovalStatus { restore });
                    self.focus = Target::CloseDialog;
                    self.panel_started = None;
                    return vec![Effect::Manage {
                        generation,
                        command,
                    }];
                }
                self.notify(self.preferences.language.t("uninstallRefused"));
                return vec![];
            }
            Target::Nav(index) => self.open(index),
            Target::Browse => self.open(1),
            Target::Retry => return vec![Effect::Reload],
            Target::DismissNotice => self.notice = None,
            Target::CloseDialog => return self.back(),
            Target::TvSettings => return vec![Effect::Launch("com.palm.app.settings".into())],
            Target::App(id) | Target::Input(id) if !self.launching => {
                return vec![Effect::Launch(id)];
            }
            Target::Filter(index) => {
                self.filter = if self.filter == index { 0 } else { index };
                self.scroll = 0.;
            }
            Target::ClearFilter => {
                self.filter = 0;
                self.focus = Target::Filter(1);
                self.scroll = 0.;
            }
            Target::ClearSearch => {
                self.query.clear();
                self.focus = Target::Search;
                self.scroll = 0.;
            }
            Target::Search => {
                self.open_keyboard(TextField::Apps);
            }
            Target::WeatherConfigure => {
                self.open(2);
                self.section = WEATHER_SECTION;
                self.open_location();
                self.open_keyboard(TextField::Location);
            }
            Target::WeatherToggle => {
                self.preferences.show_weather = !self.preferences.show_weather;
                self.weather.invalidate_forecast();
                return vec![Effect::Save];
            }
            Target::WeatherLocation => self.open_location(),
            Target::WeatherTemperature => {
                self.subpage = Some(Subpage::Temperature);
                self.focus = Target::Choice(usize::from(
                    self.preferences.temperature_unit == Unit::Fahrenheit,
                ));
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
            }
            Target::WeatherQuery => self.open_keyboard(TextField::Location),
            Target::GreetingName => self.open_keyboard(TextField::Name),
            Target::GreetingNameClear => {
                if self.preferences.home.greeting_mode == GreetingMode::Static {
                    self.preferences.home.custom_greeting.clear();
                } else {
                    self.preferences.greeting_name.clear();
                }
                self.focus = Target::GreetingName;
                return vec![Effect::Save];
            }
            Target::AppNamesHome => {
                self.preferences.show_home_names = !self.preferences.show_home_names;
                return vec![Effect::Save];
            }
            Target::AppNamesApps => {
                self.preferences.show_app_names = !self.preferences.show_app_names;
                return vec![Effect::Save];
            }
            Target::ClockFormat | Target::ClockTimeFormat => {
                let format = self.focus == Target::ClockFormat;
                self.subpage = Some(if format {
                    Subpage::ClockFormat
                } else {
                    Subpage::TimeFormat
                });
                self.focus = Target::Choice(usize::from(if format {
                    self.preferences.hour_24
                } else {
                    self.preferences.show_seconds
                }));
                self.scroll = 0.;
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
            }
            Target::InputXboxPreset => {
                if self.overlay.is_none()
                    && let Some(Subpage::Input(id)) = &self.subpage
                    && self.inventory.inputs.iter().any(|input| &input.id == id)
                    && self.preferences.set_input_name(id, "Xbox Series X")
                {
                    self.dirty = true;
                    return vec![Effect::Save];
                }
                return vec![];
            }
            Target::InputRename => self.open_keyboard(TextField::InputName),
            Target::InputNameReset => {
                if let Some(Subpage::Input(id)) = &self.subpage
                    && self.preferences.input_names.remove(id).is_some()
                {
                    self.focus = Target::InputRename;
                    return vec![Effect::Save];
                }
            }
            Target::WeatherSearch => {
                return self
                    .weather
                    .search(self.preferences.language)
                    .map(|command| vec![Effect::Weather(command)])
                    .unwrap_or_default();
            }
            Target::WeatherClear => {
                self.preferences.weather_location = None;
                self.weather.remove_location();
                self.focus = Target::WeatherSearch;
                return vec![Effect::Save, Effect::Weather(weather::Command::ClearCache)];
            }
            Target::WeatherResult(index) => {
                if let Some(place) = self.weather.results.get(index).cloned() {
                    self.preferences.weather_location = Some(place);
                    self.weather.invalidate_forecast();
                    self.weather.cancel_search();
                    self.focus = Target::WeatherSearch;
                    return vec![Effect::Save];
                }
            }
            Target::Sidebar(index) => {
                self.section = index;
                self.scroll = 0.;
                self.subpage = None;
                self.focus = self.section_entry();
            }
            Target::SystemRefresh => {
                return self
                    .system
                    .request()
                    .map(|generation| vec![Effect::SystemInfo { generation }])
                    .unwrap_or_default();
            }
            Target::PrivacyRefresh => {
                let mut effects =
                    self.control_request(crate::features::controls::Command::BlockerStatus);
                if let Some(generation) = self.privacy.request() {
                    effects.push(Effect::Privacy { generation });
                }
                return effects;
            }
            Target::Setting(index) => {
                match index {
                    22 => return self.open_app_catalog(),
                    21 => {
                        self.subpage = Some(Subpage::Display);
                        self.focus = Target::DisplayHD;
                        self.scroll = 0.;
                        return vec![];
                    }
                    19 => {
                        self.preferences.screensaver.timeout =
                            self.preferences.screensaver.timeout.next();
                        return vec![Effect::Save];
                    }
                    20 => {
                        self.preferences.screensaver.photos = !self.preferences.screensaver.photos;
                        return vec![Effect::Save];
                    }
                    18 => {
                        self.subpage = Some(Subpage::Rotation);
                        self.focus = Target::RotationMode;
                        self.scroll = 0.;
                        return vec![];
                    }
                    17 => {
                        self.subpage = Some(Subpage::Wallpapers);
                        self.focus = Target::PhotoFolder;
                        self.scroll = 0.;
                        return vec![];
                    }
                    14 => {
                        return self
                            .control_request(crate::features::controls::Command::BlockerStatus);
                    }
                    15 | 16 => {
                        self.confirm_control(if index == 15 {
                            crate::features::controls::Command::BlockerEnable
                        } else {
                            crate::features::controls::Command::BlockerDisable
                        });
                        return vec![];
                    }
                    9 => {
                        let mut controls =
                            self.control_request(crate::features::controls::Command::BlockerStatus);
                        self.subpage = Some(Subpage::Privacy);
                        self.focus = Target::PrivacyRefresh;
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        if let Some(generation) = self.privacy.request() {
                            controls.push(Effect::Privacy { generation });
                        }
                        return controls;
                    }
                    8 => {
                        self.subpage = Some(Subpage::SystemInfo);
                        self.focus = Target::SystemRefresh;
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return self
                            .system
                            .request()
                            .map(|generation| vec![Effect::SystemInfo { generation }])
                            .unwrap_or_default();
                    }
                    0 => {
                        self.subpage = Some(Subpage::AppNames);
                        self.focus = Target::AppNamesHome;
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    1 => {
                        self.subpage = Some(Subpage::Clock);
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        self.focus = Target::ClockFormat;
                        return vec![];
                    }
                    2 => self.preferences.reduced_motion = !self.preferences.reduced_motion,
                    10 => {
                        self.toggle_home_rows();
                    }
                    11 => {
                        self.subpage = Some(Subpage::Home);
                        self.focus = Target::HomeOption(HomeOption::ALL[0]);
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    12 => {
                        self.subpage = Some(Subpage::Appearance);
                        self.focus = Target::AppearanceOption(AppearanceOption::ALL[0]);
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    13 => {
                        self.subpage = Some(Subpage::Shortcuts);
                        self.focus = Target::ShortcutsEnabled;
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    4 => {
                        self.subpage = Some(Subpage::Language);
                        self.focus = Target::Choice(self.preferences.language.index());
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    5 => return vec![Effect::Quit],
                    7 => {
                        self.subpage = Some(Subpage::TextRendering);
                        self.focus = Target::Choice(self.preferences.text_rendering.index());
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    6 => {
                        self.greeting_from_home = false;
                        self.subpage = Some(Subpage::Greeting);
                        self.focus = Target::GreetingName;
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    _ => return vec![Effect::Reload],
                }
                return vec![Effect::Save];
            }
            Target::Background(index) => {
                let previous = self.current_background().to_owned();
                self.preferences.wallpapers.selected = None;
                if !self.preferences.reduced_motion && previous != BACKGROUNDS[index].0 {
                    self.previous_background = Some((previous, Instant::now()));
                }
                self.preferences.background = BACKGROUNDS[index].0.into();
                return vec![Effect::Save];
            }
            Target::InputMode(id) => {
                let mode = self
                    .preferences
                    .input_modes
                    .get(&id)
                    .copied()
                    .unwrap_or_default();
                self.subpage = Some(Subpage::Input(id));
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
                self.focus = Target::Choice(match mode {
                    InputMode::On => 0,
                    InputMode::Off => 1,
                    InputMode::Auto => 2,
                });
            }
            Target::Choice(index) => {
                match self.subpage.clone() {
                    Some(
                        Subpage::SystemInfo
                        | Subpage::Privacy
                        | Subpage::AppManagement
                        | Subpage::ManagedApp(_),
                    ) => return vec![],
                    Some(Subpage::ClockFormat) => self.preferences.hour_24 = index == 1,
                    Some(Subpage::TimeFormat) => self.preferences.show_seconds = index == 1,
                    Some(Subpage::Input(id)) => {
                        self.preferences.input_modes.insert(
                            id,
                            [InputMode::On, InputMode::Off, InputMode::Auto][index.min(2)],
                        );
                    }
                    Some(Subpage::Temperature) => {
                        self.preferences.temperature_unit = if index == 1 {
                            Unit::Fahrenheit
                        } else {
                            Unit::Celsius
                        }
                    }
                    Some(Subpage::Language) => {
                        self.preferences.language =
                            Language::ALL[index.min(Language::ALL.len() - 1)];
                        self.weather.cancel_search();
                    }
                    Some(Subpage::TextRendering) => {
                        let mode = TextRendering::ALL[index.min(TextRendering::ALL.len() - 1)];
                        if mode != self.preferences.text_rendering {
                            self.preferences.text_rendering = mode;
                            self.notify(self.preferences.language.t("textRenderingRestart"));
                        }
                    }
                    Some(
                        Subpage::Display
                        | Subpage::Rotation
                        | Subpage::Wallpapers
                        | Subpage::Shortcuts
                        | Subpage::Appearance
                        | Subpage::Home
                        | Subpage::StartupFocus
                        | Subpage::WeatherLocation
                        | Subpage::Greeting
                        | Subpage::AppNames
                        | Subpage::Clock,
                    ) => {
                        return vec![];
                    }
                    None => return vec![],
                }
                self.close_subpage();
                return vec![Effect::Save];
            }
            Target::Option(index) => {
                let Some(Overlay::Options {
                    id,
                    restore,
                    removal,
                }) = self.overlay.take()
                else {
                    return vec![];
                };
                if index == 2 {
                    if let Some(target) = removal {
                        if self.management.active() {
                            if self
                                .management
                                .target
                                .as_ref()
                                .is_some_and(|running| running.id == id)
                            {
                                self.overlay = Some(Overlay::RemovalStatus { restore });
                                self.focus = Target::CloseDialog;
                            } else {
                                self.overlay = Some(Overlay::Options {
                                    id,
                                    restore,
                                    removal: Some(target),
                                });
                                self.notify(self.preferences.language.t("uninstallBusy"));
                            }
                        } else {
                            let title = self
                                .inventory
                                .apps
                                .iter()
                                .find(|app| app.id == id)
                                .map_or_else(|| id.clone(), |app| app.title.clone());
                            self.overlay = Some(Overlay::RemovalConfirm {
                                target,
                                title,
                                restore,
                            });
                            self.focus = Target::RemovalCancel;
                        }
                    } else {
                        self.focus = restore;
                    }
                    self.panel_started = None;
                    return vec![];
                }
                self.panel_started = None;
                let old_ids: Vec<_> = self.apps().iter().map(|a| a.id.clone()).collect();
                let old_index = old_ids.iter().position(|item| item == &id).unwrap_or(0);
                let mut favorites = self.preferences.favorites(&self.inventory.apps);
                let title = self
                    .inventory
                    .apps
                    .iter()
                    .find(|app| app.id == id)
                    .map_or(id.clone(), |app| app.title.clone());
                if index == 0 {
                    if favorites.contains(&id) {
                        favorites.retain(|item| item != &id);
                        self.notify(
                            self.preferences
                                .language
                                .format("favoriteRemoved", &[("app", &title)]),
                        );
                    } else {
                        favorites.push(id.clone());
                        self.notify(
                            self.preferences
                                .language
                                .format("favoriteAdded", &[("app", &title)]),
                        );
                    }
                    self.preferences.hidden.retain(|item| item != &id);
                    self.preferences.favorites = Some(favorites);
                } else if index == 1 {
                    if self.preferences.hidden.contains(&id) {
                        self.preferences.hidden.retain(|item| item != &id);
                        self.notify(
                            self.preferences
                                .language
                                .format("appRestored", &[("app", &title)]),
                        );
                    } else {
                        self.preferences.hidden.push(id.clone());
                        self.notify(
                            self.preferences
                                .language
                                .format("appHidden", &[("app", &title)]),
                        );
                    }
                }
                if self.filter == 5 && self.hidden_count() == 0 {
                    self.filter = 0;
                }
                self.focus = restore;
                self.pointer = false;
                if let Target::App(id) = &self.focus {
                    let apps = self.apps();
                    if !apps.iter().any(|app| &app.id == id) {
                        self.focus = apps
                            .get(old_index.min(apps.len().saturating_sub(1)))
                            .map(|app| Target::App(app.id.clone()))
                            .unwrap_or_else(|| self.first_content());
                    }
                }
                return if index < 2 {
                    vec![Effect::Save]
                } else {
                    vec![]
                };
            }
            Target::Key(index) => match KEYS[index] {
                "Space" => self.text(" "),
                "Delete" => self.delete(),
                "Clear" => match self.keyboard_field {
                    TextField::PhotoFolder | TextField::PhotoUrl => self.photo_draft.clear(),
                    TextField::Location => self.weather.edit(String::new()),
                    TextField::Name | TextField::ProfileName => self.greeting_name_draft.clear(),
                    TextField::InputName => self.input_name.clear(),
                    TextField::Apps => self.query.clear(),
                },
                "Done" => {
                    self.overlay = None;
                    self.panel_started = None;
                    self.focus = self.keyboard_return();
                    return self.commit_text_input();
                }
                key => self.text(key),
            },
            _ => {}
        }
        vec![]
    }

    pub fn back(&mut self) -> Vec<Effect> {
        if self.cc_timer && matches!(self.overlay, Some(Overlay::ControlCenter { .. })) {
            self.cc_timer = false;
            self.focus = Target::Quick(crate::features::control_center::Action::Sleep);
            self.dirty = true;
            return vec![];
        }
        if self.display_trial {
            self.display_change = Some(false);
            return vec![];
        }
        self.pointer = false;
        self.dirty = true;
        if self.moving.take().is_some() {
            return vec![];
        }
        if let Some(overlay) = self.overlay.take() {
            self.panel_started = None;
            if let Overlay::RemovalConfirm {
                target, restore, ..
            } = overlay
            {
                self.overlay = Some(Overlay::Options {
                    id: target.id.clone(),
                    restore,
                    removal: Some(target),
                });
                self.focus = Target::Option(2);
                return vec![];
            }
            if matches!(overlay, Overlay::Profile) {
                self.focus = Target::Profile;
                return vec![];
            }
            if matches!(overlay, Overlay::Recents { .. }) {
                self.recents.closing_focus = Some(self.focus.clone());
                self.recents.updated = Instant::now();
                self.pressed = None;
                if self.preferences.reduced_motion {
                    self.recents.visibility = 0.;
                    self.recents.closing_focus = None;
                }
            }
            if let Overlay::Options { restore, .. }
            | Overlay::Recents { restore }
            | Overlay::RemovalStatus { restore }
            | Overlay::ControlConfirm { restore, .. }
            | Overlay::ControlCenter { restore } = overlay
            {
                self.focus = restore;
                self.recents.pending = None;
                self.reconcile_focus();
                return vec![];
            }
            self.focus = self.keyboard_return();
            return self.commit_text_input();
        }
        if self.page == Page::Settings && self.subpage.is_some() {
            self.close_subpage();
            return vec![];
        }
        if self.page == Page::Settings && !matches!(self.focus, Target::Nav(_)) {
            if matches!(self.focus, Target::Sidebar(_)) {
                self.focus = Target::Nav(2);
            } else {
                self.focus = Target::Sidebar(self.section);
            }
            self.scroll = 0.;
            return vec![];
        }
        if self.page == Page::Apps && matches!(self.focus, Target::Search | Target::ClearSearch) {
            self.query.clear();
            self.focus = Target::Nav(1);
            self.scroll = 0.;
            return vec![];
        }
        if self.page != Page::Home {
            self.open(0);
            vec![]
        } else {
            self.open_recents()
        }
    }
}
